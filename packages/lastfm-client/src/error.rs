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
    ///
    /// Retries on:
    /// - Timeouts
    /// - Rate limiting
    /// - Transport errors (connect, timeout)
    /// - Server errors (5xx)
    ///
    /// Does NOT retry on client errors (4xx except 429 rate limiting).
    pub fn is_retryable(&self) -> bool {
        match self {
            LastfmError::Timeout | LastfmError::RateLimited => true,
            LastfmError::Http(e) => {
                // Retry on transport issues
                if e.is_timeout() || e.is_connect() {
                    return true;
                }
                // Retry on server errors (5xx) but not client errors (4xx)
                matches!(e.status(), Some(status) if status.is_server_error())
            }
            _ => false,
        }
    }
}

/// Result type for Last.fm operations
pub type LastfmResult<T> = Result<T, LastfmError>;
