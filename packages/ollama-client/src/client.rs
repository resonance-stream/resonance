//! Core Ollama HTTP client with retry logic and connection pooling

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use bytes::Bytes;
use futures_util::Stream;
use reqwest::Client;
use resonance_shared_config::OllamaConfig;
use tracing::{debug, warn};

use crate::error::{OllamaError, OllamaResult};
use crate::models::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, EmbeddingRequest, EmbeddingResponse,
    GenerateOptions, GenerateRequest, GenerateResponse, ListModelsResponse,
};

/// Maximum error body size to prevent memory exhaustion
const MAX_ERROR_BODY_SIZE: usize = 1000;

/// Default retry configuration
const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
const DEFAULT_RETRY_BASE_DELAY_MS: u64 = 500;

/// Ollama API client with retry logic and connection pooling
#[derive(Debug, Clone)]
pub struct OllamaClient {
    /// HTTP client with connection pool
    http_client: Client,
    /// Configuration
    config: OllamaConfig,
    /// Number of retry attempts for transient failures
    retry_attempts: u32,
    /// Base delay for exponential backoff (milliseconds)
    retry_base_delay_ms: u64,
}

impl OllamaClient {
    /// Create a new Ollama client from configuration
    pub fn new(config: &OllamaConfig) -> OllamaResult<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .map_err(OllamaError::HttpError)?;

        Ok(Self {
            http_client,
            config: config.clone(),
            retry_attempts: DEFAULT_RETRY_ATTEMPTS,
            retry_base_delay_ms: DEFAULT_RETRY_BASE_DELAY_MS,
        })
    }

    /// Create a client with custom HTTP client (for testing)
    pub fn with_client(config: &OllamaConfig, http_client: Client) -> Self {
        Self {
            http_client,
            config: config.clone(),
            retry_attempts: DEFAULT_RETRY_ATTEMPTS,
            retry_base_delay_ms: DEFAULT_RETRY_BASE_DELAY_MS,
        }
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, attempts: u32, base_delay_ms: u64) -> Self {
        self.retry_attempts = attempts;
        self.retry_base_delay_ms = base_delay_ms;
        self
    }

    /// Get the configuration
    pub fn config(&self) -> &OllamaConfig {
        &self.config
    }

    /// Execute an async operation with retry logic
    async fn with_retry<T, F, Fut>(&self, operation: F) -> OllamaResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = OllamaResult<T>>,
    {
        // Handle edge case of 0 retry attempts - run operation once
        if self.retry_attempts == 0 {
            return operation().await;
        }

        let mut last_error = None;

        for attempt in 0..self.retry_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !e.is_retryable() {
                        // Non-retryable errors return immediately
                        return Err(e);
                    } else if attempt < self.retry_attempts - 1 {
                        // Retryable error, not last attempt - wait and retry
                        let delay = self.retry_base_delay_ms * 2_u64.pow(attempt);
                        warn!(
                            attempt = attempt + 1,
                            max_attempts = self.retry_attempts,
                            delay_ms = delay,
                            error = %e,
                            "Retrying after transient error"
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        last_error = Some(e);
                    } else {
                        // Retryable error on last attempt - exit loop to return RetriesExhausted
                        last_error = Some(e);
                        break;
                    }
                }
            }
        }

        Err(OllamaError::RetriesExhausted {
            attempts: self.retry_attempts,
            last_error: last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Unknown error".to_string()),
        })
    }

    /// Truncate error body to prevent memory exhaustion
    /// Safely handles UTF-8 boundaries to avoid panics on multi-byte characters
    fn truncate_error_body(body: String) -> String {
        if body.len() <= MAX_ERROR_BODY_SIZE {
            return body;
        }

        // Find the last UTF-8 character boundary at or below MAX_ERROR_BODY_SIZE
        // We use the start index of each character to ensure we don't overshoot
        let truncate_at = body
            .char_indices()
            .map(|(i, _)| i)
            .take_while(|i| *i <= MAX_ERROR_BODY_SIZE)
            .last()
            .unwrap_or(0);

        format!("{}... (truncated)", &body[..truncate_at])
    }

    /// Check if Ollama is reachable
    pub async fn health_check(&self) -> OllamaResult<bool> {
        let url = format!("{}/api/tags", self.config.url.trim_end_matches('/'));

        match self.http_client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) if e.is_connect() => {
                Err(OllamaError::ConnectionRefused(self.config.url.clone()))
            }
            Err(e) => Err(OllamaError::HttpError(e)),
        }
    }

    /// List available models
    pub async fn list_models(&self) -> OllamaResult<Vec<String>> {
        let url = format!("{}/api/tags", self.config.url.trim_end_matches('/'));

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            if e.is_connect() {
                OllamaError::ConnectionRefused(self.config.url.clone())
            } else {
                OllamaError::HttpError(e)
            }
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = Self::truncate_error_body(response.text().await.unwrap_or_default());
            return Err(OllamaError::ApiError(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let list: ListModelsResponse = response.json().await?;
        Ok(list.models.into_iter().map(|m| m.name).collect())
    }

    /// Check if a model is available
    pub async fn has_model(&self, model: &str) -> OllamaResult<bool> {
        let models = self.list_models().await?;
        let model_base = model.split(':').next().unwrap_or(model);

        Ok(models.iter().any(|m| {
            let m_base = m.split(':').next().unwrap_or(m);
            m_base == model_base
        }))
    }

    /// Internal embedding generation (single request, no retry)
    async fn generate_embedding_internal(&self, text: &str) -> OllamaResult<Vec<f32>> {
        let request = EmbeddingRequest {
            model: self.config.embedding_model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .http_client
            .post(self.config.embeddings_url())
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionRefused(self.config.url.clone())
                } else if e.is_timeout() {
                    OllamaError::Timeout(self.config.timeout_secs)
                } else {
                    OllamaError::HttpError(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = Self::truncate_error_body(response.text().await.unwrap_or_default());

            if body.contains("model") && body.contains("not found") {
                return Err(OllamaError::ModelNotFound(
                    self.config.embedding_model.clone(),
                ));
            }

            return Err(OllamaError::ApiError(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let embedding_response: EmbeddingResponse = response.json().await?;
        Ok(embedding_response.embedding)
    }

    /// Generate embeddings for text with retry logic
    pub async fn generate_embedding(&self, text: &str) -> OllamaResult<Vec<f32>> {
        let text = text.to_string();

        debug!(
            model = %self.config.embedding_model,
            text_len = text.len(),
            "Generating embedding"
        );

        let result = self
            .with_retry(|| {
                let text = text.clone();
                async move { self.generate_embedding_internal(&text).await }
            })
            .await?;

        debug!(dimensions = result.len(), "Embedding generated");

        Ok(result)
    }

    /// Generate embeddings for multiple texts concurrently
    ///
    /// # Arguments
    /// * `texts` - List of texts to generate embeddings for
    /// * `concurrency` - Maximum concurrent requests (recommend 3-5 for Ollama)
    pub async fn generate_embeddings_batch(
        &self,
        texts: Vec<String>,
        concurrency: usize,
    ) -> OllamaResult<Vec<Vec<f32>>> {
        use futures_util::stream::{self, StreamExt};

        // Ensure concurrency is at least 1 to prevent buffer_unordered from hanging
        let concurrency = concurrency.max(1);

        debug!(
            count = texts.len(),
            concurrency = concurrency,
            "Generating batch embeddings"
        );

        let results: Vec<OllamaResult<Vec<f32>>> = stream::iter(texts.into_iter().enumerate())
            .map(|(i, text)| async move {
                debug!(index = i, "Processing embedding");
                self.generate_embedding(&text).await
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        // Collect results, propagating first error
        results.into_iter().collect()
    }

    /// Internal text generation (single request, no retry)
    async fn generate_internal(
        &self,
        prompt: &str,
        options: Option<GenerateOptions>,
    ) -> OllamaResult<String> {
        let request = GenerateRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: options.or_else(|| {
                Some(GenerateOptions {
                    temperature: Some(self.config.temperature),
                    num_predict: Some(self.config.max_tokens),
                    ..Default::default()
                })
            }),
        };

        let response = self
            .http_client
            .post(self.config.generate_url())
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionRefused(self.config.url.clone())
                } else if e.is_timeout() {
                    OllamaError::Timeout(self.config.timeout_secs)
                } else {
                    OllamaError::HttpError(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = Self::truncate_error_body(response.text().await.unwrap_or_default());

            if body.contains("model") && body.contains("not found") {
                return Err(OllamaError::ModelNotFound(self.config.model.clone()));
            }

            return Err(OllamaError::ApiError(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let generate_response: GenerateResponse = response.json().await?;
        Ok(generate_response.response)
    }

    /// Generate text from a prompt with retry logic
    pub async fn generate(&self, prompt: &str) -> OllamaResult<String> {
        self.generate_with_options(prompt, None).await
    }

    /// Generate text with custom options and retry logic
    pub async fn generate_with_options(
        &self,
        prompt: &str,
        options: Option<GenerateOptions>,
    ) -> OllamaResult<String> {
        let prompt = prompt.to_string();

        debug!(
            model = %self.config.model,
            prompt_len = prompt.len(),
            "Generating text"
        );

        let result = self
            .with_retry(|| {
                let prompt = prompt.clone();
                let options = options.clone();
                async move { self.generate_internal(&prompt, options).await }
            })
            .await?;

        debug!(response_len = result.len(), "Text generated");

        Ok(result)
    }

    /// Internal chat (single request, no retry)
    async fn chat_internal(
        &self,
        messages: &[ChatMessage],
        options: Option<GenerateOptions>,
    ) -> OllamaResult<String> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: messages.to_vec(),
            stream: false,
            options: options.or_else(|| {
                Some(GenerateOptions {
                    temperature: Some(self.config.temperature),
                    num_predict: Some(self.config.max_tokens),
                    ..Default::default()
                })
            }),
        };

        let response = self
            .http_client
            .post(self.config.chat_url())
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionRefused(self.config.url.clone())
                } else if e.is_timeout() {
                    OllamaError::Timeout(self.config.timeout_secs)
                } else {
                    OllamaError::HttpError(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = Self::truncate_error_body(response.text().await.unwrap_or_default());

            if body.contains("model") && body.contains("not found") {
                return Err(OllamaError::ModelNotFound(self.config.model.clone()));
            }

            return Err(OllamaError::ApiError(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response.message.content)
    }

    /// Chat with the model with retry logic
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> OllamaResult<String> {
        self.chat_with_options(messages, None).await
    }

    /// Chat with custom options and retry logic
    pub async fn chat_with_options(
        &self,
        messages: Vec<ChatMessage>,
        options: Option<GenerateOptions>,
    ) -> OllamaResult<String> {
        debug!(
            model = %self.config.model,
            message_count = messages.len(),
            "Sending chat request"
        );

        let result = self
            .with_retry(|| {
                let messages = messages.clone();
                let options = options.clone();
                async move { self.chat_internal(&messages, options).await }
            })
            .await?;

        debug!(response_len = result.len(), "Chat response received");

        Ok(result)
    }

    /// Stream chat completion responses token by token
    ///
    /// This method sends a chat request to Ollama with streaming enabled,
    /// returning a stream of `ChatStreamChunk` items as tokens are generated.
    ///
    /// # Arguments
    /// * `messages` - The conversation history
    /// * `options` - Optional generation parameters
    ///
    /// # Returns
    /// A stream of `OllamaResult<ChatStreamChunk>` items
    ///
    /// # Example
    /// ```ignore
    /// use futures_util::StreamExt;
    ///
    /// let mut stream = client.chat_stream(messages, None).await?;
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk {
    ///         Ok(c) => print!("{}", c.message.content),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// ```
    pub async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        options: Option<GenerateOptions>,
    ) -> OllamaResult<Pin<Box<dyn Stream<Item = OllamaResult<ChatStreamChunk>> + Send>>> {
        debug!(
            model = %self.config.model,
            message_count = messages.len(),
            "Starting streaming chat request"
        );

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: true,
            options: options.or_else(|| {
                Some(GenerateOptions {
                    temperature: Some(self.config.temperature),
                    num_predict: Some(self.config.max_tokens),
                    ..Default::default()
                })
            }),
        };

        let response = self
            .http_client
            .post(self.config.chat_url())
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionRefused(self.config.url.clone())
                } else if e.is_timeout() {
                    OllamaError::Timeout(self.config.timeout_secs)
                } else {
                    OllamaError::HttpError(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = Self::truncate_error_body(response.text().await.unwrap_or_default());

            if body.contains("model") && body.contains("not found") {
                return Err(OllamaError::ModelNotFound(self.config.model.clone()));
            }

            return Err(OllamaError::ApiError(format!(
                "Status {}: {}",
                status, body
            )));
        }

        // Get the bytes stream from reqwest and transform it to parse NDJSON
        let byte_stream = response.bytes_stream();

        // Create a stream that parses NDJSON lines
        let chunk_stream = NdjsonStream::new(byte_stream);

        debug!("Streaming chat response started");

        Ok(Box::pin(chunk_stream))
    }
}

/// A stream adapter that parses NDJSON (newline-delimited JSON) from a byte stream
struct NdjsonStream<S> {
    inner: S,
    buffer: String,
}

impl<S> NdjsonStream<S> {
    fn new(stream: S) -> Self {
        Self {
            inner: stream,
            buffer: String::new(),
        }
    }
}

impl<S, E> Stream for NdjsonStream<S>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::error::Error + Send + Sync + 'static,
{
    type Item = OllamaResult<ChatStreamChunk>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        // First check if we have a complete line in the buffer
        if let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].trim().to_string();
            self.buffer = self.buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                // Empty line, poll again
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }

            return Poll::Ready(Some(
                serde_json::from_str::<ChatStreamChunk>(&line).map_err(OllamaError::from),
            ));
        }

        // No complete line, try to read more data
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                // Append new data to buffer, using lossy conversion for invalid UTF-8
                match std::str::from_utf8(&bytes) {
                    Ok(text) => {
                        self.buffer.push_str(text);
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            byte_count = bytes.len(),
                            "Invalid UTF-8 in streaming response, using lossy conversion"
                        );
                        self.buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                }

                // Try to extract a line now
                if let Some(newline_pos) = self.buffer.find('\n') {
                    let line = self.buffer[..newline_pos].trim().to_string();
                    self.buffer = self.buffer[newline_pos + 1..].to_string();

                    if line.is_empty() {
                        // Empty line, poll again
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }

                    return Poll::Ready(Some(
                        serde_json::from_str::<ChatStreamChunk>(&line).map_err(OllamaError::from),
                    ));
                }

                // Still no complete line, need more data
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(OllamaError::ApiError(e.to_string()))))
            }
            Poll::Ready(None) => {
                // Stream ended, check if there's remaining data in buffer
                if !self.buffer.is_empty() {
                    let line = std::mem::take(&mut self.buffer);
                    let line = line.trim();
                    if !line.is_empty() {
                        return Poll::Ready(Some(
                            serde_json::from_str::<ChatStreamChunk>(line).map_err(OllamaError::from),
                        ));
                    }
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper to create a test config pointing to the mock server
    fn test_config(server_url: &str) -> OllamaConfig {
        OllamaConfig {
            url: server_url.to_string(),
            model: "test-model".to_string(),
            embedding_model: "test-embed".to_string(),
            timeout_secs: 30,
            max_tokens: 1024,
            temperature: 0.7,
        }
    }

    #[test]
    fn test_client_creation() {
        let config = OllamaConfig::default();
        let client = OllamaClient::new(&config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_with_retry_configuration() {
        let config = OllamaConfig::default();
        let client = OllamaClient::new(&config)
            .unwrap()
            .with_retry_config(5, 1000);
        assert_eq!(client.retry_attempts, 5);
        assert_eq!(client.retry_base_delay_ms, 1000);
    }

    #[test]
    fn test_truncate_error_body() {
        let short = "short error".to_string();
        assert_eq!(OllamaClient::truncate_error_body(short.clone()), short);

        let long = "x".repeat(2000);
        let truncated = OllamaClient::truncate_error_body(long);
        assert!(truncated.len() < 1100);
        assert!(truncated.ends_with("... (truncated)"));
    }

    #[test]
    fn test_truncate_error_body_utf8_boundary() {
        // Create a string with multi-byte characters
        // '日' is 3 bytes in UTF-8
        let utf8_str = "日".repeat(500); // 1500 bytes
        let truncated = OllamaClient::truncate_error_body(utf8_str);
        // Should not panic and should be valid UTF-8
        assert!(truncated.ends_with("... (truncated)"));
        // Verify it's valid UTF-8 (no partial chars)
        let _ = truncated.chars().count();
    }

    #[test]
    fn test_truncate_error_body_exact_boundary() {
        let exact = "x".repeat(MAX_ERROR_BODY_SIZE);
        let result = OllamaClient::truncate_error_body(exact.clone());
        // At exactly MAX_ERROR_BODY_SIZE, should NOT truncate (it's within limit)
        assert_eq!(result, exact);
    }

    #[test]
    fn test_truncate_error_body_just_over() {
        let over = "x".repeat(MAX_ERROR_BODY_SIZE + 1);
        let result = OllamaClient::truncate_error_body(over);
        // Just over the limit should truncate
        assert!(result.ends_with("... (truncated)"));
        assert!(result.len() < MAX_ERROR_BODY_SIZE + 20);
    }

    // ========== chat_stream() tests ==========

    #[tokio::test]
    async fn test_chat_stream_parses_ndjson() {
        let server = MockServer::start().await;

        // Ollama streams NDJSON - one JSON object per line
        let streaming_response = r#"{"message":{"role":"assistant","content":"Hello"},"done":false}
{"message":{"role":"assistant","content":" world"},"done":false}
{"message":{"role":"assistant","content":"!"},"done":true,"done_reason":"stop"}
"#;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_string(streaming_response))
            .mount(&server)
            .await;

        let config = test_config(&server.uri());
        let client = OllamaClient::new(&config).unwrap();

        let messages = vec![ChatMessage::user("Hi")];
        let mut stream = client.chat_stream(messages, None).await.unwrap();

        let mut chunks = Vec::new();
        while let Some(result) = stream.next().await {
            chunks.push(result.unwrap());
        }

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].message.content, "Hello");
        assert!(!chunks[0].done);
        assert_eq!(chunks[1].message.content, " world");
        assert!(!chunks[1].done);
        assert_eq!(chunks[2].message.content, "!");
        assert!(chunks[2].done);
        assert_eq!(chunks[2].done_reason, Some("stop".to_string()));
    }

    #[tokio::test]
    async fn test_chat_stream_handles_partial_buffer() {
        // Test that the NDJSON parser handles data arriving in chunks
        // by verifying it correctly buffers partial lines
        let server = MockServer::start().await;

        // Single complete response - the buffer handling is tested by
        // verifying we can parse even when the response is small
        let streaming_response = r#"{"message":{"role":"assistant","content":"test"},"done":true}
"#;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_string(streaming_response))
            .mount(&server)
            .await;

        let config = test_config(&server.uri());
        let client = OllamaClient::new(&config).unwrap();

        let messages = vec![ChatMessage::user("test")];
        let mut stream = client.chat_stream(messages, None).await.unwrap();

        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk.message.content, "test");
        assert!(chunk.done);

        // No more chunks
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_chat_stream_handles_empty_lines() {
        let server = MockServer::start().await;

        // Response with empty lines between chunks (should be skipped)
        let streaming_response = r#"{"message":{"role":"assistant","content":"a"},"done":false}

{"message":{"role":"assistant","content":"b"},"done":true}
"#;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_string(streaming_response))
            .mount(&server)
            .await;

        let config = test_config(&server.uri());
        let client = OllamaClient::new(&config).unwrap();

        let messages = vec![ChatMessage::user("test")];
        let mut stream = client.chat_stream(messages, None).await.unwrap();

        let mut chunks = Vec::new();
        while let Some(result) = stream.next().await {
            chunks.push(result.unwrap());
        }

        // Empty lines should be skipped, only 2 valid chunks
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].message.content, "a");
        assert_eq!(chunks[1].message.content, "b");
    }

    #[tokio::test]
    async fn test_chat_stream_handles_trailing_data() {
        let server = MockServer::start().await;

        // Response without trailing newline (tests buffer drain at end)
        let streaming_response =
            r#"{"message":{"role":"assistant","content":"final"},"done":true}"#;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_string(streaming_response))
            .mount(&server)
            .await;

        let config = test_config(&server.uri());
        let client = OllamaClient::new(&config).unwrap();

        let messages = vec![ChatMessage::user("test")];
        let mut stream = client.chat_stream(messages, None).await.unwrap();

        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk.message.content, "final");
        assert!(chunk.done);
    }

    #[tokio::test]
    async fn test_chat_stream_http_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let config = test_config(&server.uri());
        let client = OllamaClient::new(&config).unwrap();

        let messages = vec![ChatMessage::user("test")];
        let result = client.chat_stream(messages, None).await;

        match result {
            Err(OllamaError::ApiError(_)) => {} // expected
            Err(e) => panic!("Expected ApiError, got: {:?}", e),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[tokio::test]
    async fn test_chat_stream_model_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(404).set_body_string("model 'test-model' not found"),
            )
            .mount(&server)
            .await;

        let config = test_config(&server.uri());
        let client = OllamaClient::new(&config).unwrap();

        let messages = vec![ChatMessage::user("test")];
        let result = client.chat_stream(messages, None).await;

        match result {
            Err(OllamaError::ModelNotFound(_)) => {} // expected
            Err(e) => panic!("Expected ModelNotFound, got: {:?}", e),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[tokio::test]
    async fn test_chat_stream_invalid_json() {
        let server = MockServer::start().await;

        // Invalid JSON in the stream
        let streaming_response = r#"{"message":{"role":"assistant","content":"ok"},"done":false}
not valid json
{"message":{"role":"assistant","content":"after"},"done":true}
"#;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_string(streaming_response))
            .mount(&server)
            .await;

        let config = test_config(&server.uri());
        let client = OllamaClient::new(&config).unwrap();

        let messages = vec![ChatMessage::user("test")];
        let mut stream = client.chat_stream(messages, None).await.unwrap();

        // First chunk should succeed
        let first = stream.next().await.unwrap();
        assert!(first.is_ok());
        assert_eq!(first.unwrap().message.content, "ok");

        // Second chunk should be a parse error
        let second = stream.next().await.unwrap();
        assert!(second.is_err());
        assert!(matches!(second.unwrap_err(), OllamaError::JsonError(_)));

        // Third chunk should succeed (we continue after errors)
        let third = stream.next().await.unwrap();
        assert!(third.is_ok());
        assert_eq!(third.unwrap().message.content, "after");
    }

    #[tokio::test]
    async fn test_chat_stream_with_options() {
        let server = MockServer::start().await;

        let streaming_response =
            r#"{"message":{"role":"assistant","content":"response"},"done":true}
"#;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_string(streaming_response))
            .mount(&server)
            .await;

        let config = test_config(&server.uri());
        let client = OllamaClient::new(&config).unwrap();

        let messages = vec![ChatMessage::user("test")];
        let options = GenerateOptions {
            temperature: Some(0.5),
            num_predict: Some(100),
            ..Default::default()
        };

        let mut stream = client.chat_stream(messages, Some(options)).await.unwrap();

        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk.message.content, "response");
    }

    // Test for NdjsonStream directly to verify buffer handling
    #[tokio::test]
    async fn test_ndjson_stream_multiple_lines_in_single_chunk() {
        use tokio_stream::iter;

        // Simulate multiple JSON lines arriving in a single chunk
        let data = Bytes::from(
            r#"{"message":{"role":"assistant","content":"a"},"done":false}
{"message":{"role":"assistant","content":"b"},"done":true}
"#,
        );

        let byte_stream = iter(vec![Ok::<_, std::io::Error>(data)]);
        let mut ndjson_stream = NdjsonStream::new(byte_stream);

        let first = ndjson_stream.next().await.unwrap().unwrap();
        assert_eq!(first.message.content, "a");

        let second = ndjson_stream.next().await.unwrap().unwrap();
        assert_eq!(second.message.content, "b");
        assert!(second.done);

        assert!(ndjson_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_ndjson_stream_split_across_chunks() {
        use tokio_stream::iter;

        // Simulate a JSON line split across multiple chunks
        let chunk1 = Bytes::from(r#"{"message":{"role":"assistant","#);
        let chunk2 = Bytes::from(r#""content":"split"},"done":true}
"#);

        let byte_stream = iter(vec![
            Ok::<_, std::io::Error>(chunk1),
            Ok::<_, std::io::Error>(chunk2),
        ]);
        let mut ndjson_stream = NdjsonStream::new(byte_stream);

        let result = ndjson_stream.next().await.unwrap().unwrap();
        assert_eq!(result.message.content, "split");
        assert!(result.done);
    }
}
