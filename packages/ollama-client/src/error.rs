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
    ///
    /// Only retry on:
    /// - Timeouts
    /// - Connection refused
    /// - HTTP transport errors (connect, timeout)
    /// - Server errors (5xx) and rate limiting (429)
    ///
    /// Does NOT retry on client errors (4xx except 429).
    pub fn is_retryable(&self) -> bool {
        match self {
            OllamaError::Timeout(_) | OllamaError::ConnectionRefused(_) => true,
            OllamaError::HttpError(e) => {
                // Retry on transport issues
                if e.is_timeout() || e.is_connect() {
                    return true;
                }
                // Retry on server errors (5xx) or rate limiting (429)
                matches!(e.status(), Some(status) if status.is_server_error() || status.as_u16() == 429)
            }
            _ => false,
        }
    }
}

/// Result type for Ollama operations
pub type OllamaResult<T> = Result<T, OllamaError>;
