//! Core Ollama HTTP client with retry logic and connection pooling

use std::future::Future;
use std::time::Duration;

use reqwest::Client;
use resonance_shared_config::OllamaConfig;
use tracing::{debug, warn};

use crate::error::{OllamaError, OllamaResult};
use crate::models::{
    ChatMessage, ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, GenerateOptions,
    GenerateRequest, GenerateResponse, ListModelsResponse,
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
    fn truncate_error_body(body: String) -> String {
        if body.len() > MAX_ERROR_BODY_SIZE {
            format!("{}... (truncated)", &body[..MAX_ERROR_BODY_SIZE])
        } else {
            body
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
