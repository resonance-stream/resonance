//! Error types for Ollama client

use thiserror::Error;

/// Errors that can occur when interacting with Ollama
#[derive(Error, Debug)]
pub enum OllamaError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Failed to serialize/deserialize JSON
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Ollama API returned an error
    #[error("Ollama API error: {0}")]
    ApiError(String),

    /// Model not found or not pulled
    #[error("Model not found: {0}. Try running 'ollama pull {0}'")]
    ModelNotFound(String),

    /// Request timeout
    #[error("Request timed out after {0} seconds")]
    Timeout(u64),

    /// Invalid response from Ollama
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Embedding dimension mismatch
    #[error("Embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// Connection refused (Ollama not running)
    #[error("Connection refused. Is Ollama running at {0}?")]
    ConnectionRefused(String),

    /// All retry attempts exhausted
    #[error("All {attempts} retry attempts failed. Last error: {last_error}")]
    RetriesExhausted { attempts: u32, last_error: String },
}

impl OllamaError {
    /// Check if this error is retryable (transient)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            OllamaError::Timeout(_) | OllamaError::ConnectionRefused(_) | OllamaError::HttpError(_)
        )
    }
}

/// Result type for Ollama operations
pub type OllamaResult<T> = Result<T, OllamaError>;
