//! Mock Ollama server for testing embedding and generation
//!
//! Provides a [`MockOllamaServer`] that simulates Ollama API endpoints
//! for testing AI-related functionality without a real Ollama instance.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mock Ollama server for testing embedding and generation
///
/// This struct wraps a [`wiremock::MockServer`] and provides convenience methods
/// for setting up common Ollama API responses.
///
/// # Example
///
/// ```rust,ignore
/// use resonance_test_utils::MockOllamaServer;
///
/// #[tokio::test]
/// async fn test_embeddings() {
///     let server = MockOllamaServer::start().await;
///     server.mock_embeddings_success().await;
///
///     // Configure your Ollama client with server.url()
///     let url = server.url();
///     // ... run your test
/// }
/// ```
pub struct MockOllamaServer {
    server: MockServer,
    embedding_call_count: Arc<AtomicUsize>,
    generate_call_count: Arc<AtomicUsize>,
    chat_call_count: Arc<AtomicUsize>,
}

impl MockOllamaServer {
    /// Start a new mock Ollama server
    pub async fn start() -> Self {
        let server = MockServer::start().await;
        let embedding_call_count = Arc::new(AtomicUsize::new(0));
        let generate_call_count = Arc::new(AtomicUsize::new(0));
        let chat_call_count = Arc::new(AtomicUsize::new(0));

        Self {
            server,
            embedding_call_count,
            generate_call_count,
            chat_call_count,
        }
    }

    /// Get the server URL
    pub fn url(&self) -> String {
        self.server.uri()
    }

    /// Mount a mock for successful embedding generation
    ///
    /// Returns a 768-dimensional embedding (nomic-embed-text dimension)
    pub async fn mock_embeddings_success(&self) {
        let embedding: Vec<f32> = (0..768).map(|i| (i as f32 * 0.001) % 1.0).collect();

        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "embedding": embedding
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for embedding generation with custom embedding
    pub async fn mock_embeddings_with_value(&self, embedding: Vec<f32>) {
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "embedding": embedding
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for embedding generation failure
    pub async fn mock_embeddings_failure(&self, status_code: u16, error_message: &str) {
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(
                ResponseTemplate::new(status_code).set_body_json(json!({
                    "error": error_message
                })),
            )
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for model not found error
    pub async fn mock_embeddings_model_not_found(&self) {
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "error": "model 'nomic-embed-text' not found, try pulling it first"
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for successful text generation
    pub async fn mock_generate_success(&self, response_text: &str) {
        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "mistral",
                "response": response_text,
                "done": true
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for text generation failure
    pub async fn mock_generate_failure(&self, status_code: u16, error_message: &str) {
        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(
                ResponseTemplate::new(status_code).set_body_json(json!({
                    "error": error_message
                })),
            )
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for successful chat completion
    pub async fn mock_chat_success(&self, response_text: &str) {
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "mistral",
                "message": {
                    "role": "assistant",
                    "content": response_text
                },
                "done": true
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for mood analysis response
    pub async fn mock_mood_analysis(&self, moods: &[&str], tags: &[&str], description: &str) {
        let response = json!({
            "moods": moods,
            "tags": tags,
            "description": description,
            "energy": "medium",
            "valence": "positive"
        });

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "mistral",
                "message": {
                    "role": "assistant",
                    "content": serde_json::to_string(&response).unwrap()
                },
                "done": true
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for the /api/tags endpoint (list models)
    pub async fn mock_list_models(&self, models: &[&str]) {
        let model_list: Vec<serde_json::Value> = models
            .iter()
            .map(|name| {
                json!({
                    "name": name,
                    "modified_at": "2024-01-01T00:00:00Z",
                    "size": 4_000_000_000_i64
                })
            })
            .collect();

        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": model_list
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for connection timeout (delayed response)
    pub async fn mock_timeout(&self, delay_ms: u64) {
        Mock::given(method("POST"))
            .and(path_regex("/api/(embeddings|generate|chat)"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_millis(delay_ms))
                    .set_body_json(json!({"error": "timeout"})),
            )
            .mount(&self.server)
            .await;
    }

    /// Get embedding call count
    pub fn embedding_calls(&self) -> usize {
        self.embedding_call_count.load(Ordering::SeqCst)
    }

    /// Get generate call count
    pub fn generate_calls(&self) -> usize {
        self.generate_call_count.load(Ordering::SeqCst)
    }

    /// Get chat call count
    pub fn chat_calls(&self) -> usize {
        self.chat_call_count.load(Ordering::SeqCst)
    }

    /// Get reference to the underlying mock server for custom mock setups
    pub fn inner(&self) -> &MockServer {
        &self.server
    }

    /// Mount a mock for chat completion with custom response JSON
    pub async fn mock_chat_with_json(&self, response_json: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "mistral",
                "message": {
                    "role": "assistant",
                    "content": serde_json::to_string(&response_json).unwrap()
                },
                "done": true
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for chat completion failure
    pub async fn mock_chat_failure(&self, status_code: u16, error_message: &str) {
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(status_code).set_body_json(json!({
                    "error": error_message
                })),
            )
            .mount(&self.server)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_ollama_server_starts() {
        let server = MockOllamaServer::start().await;
        assert!(!server.url().is_empty());
        assert!(server.url().starts_with("http://"));
    }

    #[tokio::test]
    async fn test_mock_ollama_embeddings() {
        let server = MockOllamaServer::start().await;
        server.mock_embeddings_success().await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/embeddings", server.url()))
            .json(&serde_json::json!({"model": "nomic-embed-text", "prompt": "test"}))
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());

        let body: serde_json::Value = response.json().await.unwrap();
        let embedding = body["embedding"].as_array().unwrap();
        assert_eq!(embedding.len(), 768);
    }

    #[tokio::test]
    async fn test_mock_ollama_chat() {
        let server = MockOllamaServer::start().await;
        server.mock_chat_success("Hello, world!").await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/chat", server.url()))
            .json(&serde_json::json!({
                "model": "mistral",
                "messages": [{"role": "user", "content": "Hi"}]
            }))
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());

        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["message"]["content"], "Hello, world!");
    }

    #[tokio::test]
    async fn test_mock_ollama_embeddings_failure() {
        let server = MockOllamaServer::start().await;
        server.mock_embeddings_failure(500, "Internal error").await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/embeddings", server.url()))
            .json(&serde_json::json!({"model": "nomic-embed-text", "prompt": "test"}))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), 500);
    }

    #[tokio::test]
    async fn test_mock_ollama_list_models() {
        let server = MockOllamaServer::start().await;
        server.mock_list_models(&["mistral", "llama2"]).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/api/tags", server.url()))
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());

        let body: serde_json::Value = response.json().await.unwrap();
        let models = body["models"].as_array().unwrap();
        assert_eq!(models.len(), 2);
    }
}
