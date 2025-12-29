//! Last.fm API error types

use thiserror::Error;

/// Last.fm API client errors
#[derive(Error, Debug)]
pub enum LastfmError {
    /// API key is missing or invalid
    #[error("API key is required for Last.fm API access")]
    MissingApiKey,

    /// Invalid input provided to API method
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing failed
    #[error("Failed to parse Last.fm response: {0}")]
    Parse(#[from] serde_json::Error),

    /// Last.fm API returned an error
    #[error("Last.fm API error {code}: {message}")]
    Api { code: i32, message: String },

    /// Artist not found
    #[error("Artist not found: {0}")]
    ArtistNotFound(String),

    /// Rate limited by Last.fm
    #[error("Rate limited by Last.fm API")]
    RateLimited,

    /// Request timeout
    #[error("Request to Last.fm timed out")]
    Timeout,
}

impl LastfmError {
    /// Check if this error is retryable (transient failure)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            LastfmError::Timeout | LastfmError::Http(_) | LastfmError::RateLimited
        )
    }
}

/// Result type for Last.fm operations
pub type LastfmResult<T> = Result<T, LastfmError>;
