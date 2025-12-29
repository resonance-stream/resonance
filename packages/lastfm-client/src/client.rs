//! Last.fm API client implementation

use std::fmt;
use std::future::Future;
use std::time::Duration;

use reqwest::Client;
use tracing::{debug, instrument, warn};

use crate::error::{LastfmError, LastfmResult};
use crate::models::{
    ArtistTag, ErrorResponse, SimilarArtist, SimilarArtistsResponse, TopTagsResponse,
};

/// Last.fm API base URL
const LASTFM_API_URL: &str = "https://ws.audioscrobbler.com/2.0/";

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// Default connection timeout in seconds
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 5;

/// Default number of similar artists to return
const DEFAULT_SIMILAR_LIMIT: u32 = 10;

/// Maximum artist name length
const MAX_ARTIST_NAME_LENGTH: usize = 256;

/// Default number of retry attempts for transient failures
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (milliseconds)
const RETRY_BASE_DELAY_MS: u64 = 100;

/// Last.fm API client
#[derive(Clone)]
pub struct LastfmClient {
    http_client: Client,
    api_key: String,
    max_retries: u32,
}

/// API key validation status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeyStatus {
    /// API key is valid
    Valid,
    /// API key is invalid
    Invalid,
    /// Could not determine validity (network error, etc.)
    Unknown(String),
}

impl fmt::Debug for LastfmClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LastfmClient")
            .field("api_key", &"[REDACTED]")
            .field("max_retries", &self.max_retries)
            .finish()
    }
}

impl LastfmClient {
    /// Create a new Last.fm client with the given API key
    ///
    /// # Errors
    /// Returns `LastfmError::MissingApiKey` if the API key is empty
    pub fn new(api_key: impl Into<String>) -> LastfmResult<Self> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LastfmError::MissingApiKey);
        }

        let http_client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
            .pool_max_idle_per_host(5)
            .pool_idle_timeout(Duration::from_secs(90))
            .user_agent("Resonance/1.0")
            .build()?;

        Ok(Self {
            http_client,
            api_key,
            max_retries: DEFAULT_MAX_RETRIES,
        })
    }

    /// Create a Last.fm client from environment variable
    ///
    /// Reads `LASTFM_API_KEY` from the environment.
    ///
    /// # Errors
    /// - `LastfmError::MissingApiKey` if the variable is not set or is empty
    /// - `LastfmError::InvalidInput` if the variable contains invalid UTF-8
    pub fn from_env() -> LastfmResult<Self> {
        match std::env::var("LASTFM_API_KEY") {
            Ok(key) if key.is_empty() => Err(LastfmError::MissingApiKey),
            Ok(key) => Self::new(key),
            Err(std::env::VarError::NotPresent) => Err(LastfmError::MissingApiKey),
            Err(std::env::VarError::NotUnicode(_)) => Err(LastfmError::InvalidInput(
                "LASTFM_API_KEY contains invalid UTF-8".to_string(),
            )),
        }
    }

    /// Validate artist name input
    fn validate_artist_name(artist_name: &str) -> LastfmResult<&str> {
        let trimmed = artist_name.trim();
        if trimmed.is_empty() {
            return Err(LastfmError::InvalidInput(
                "artist name cannot be empty".to_string(),
            ));
        }
        if trimmed.len() > MAX_ARTIST_NAME_LENGTH {
            return Err(LastfmError::InvalidInput(format!(
                "artist name too long (max {} characters)",
                MAX_ARTIST_NAME_LENGTH
            )));
        }
        Ok(trimmed)
    }

    /// Execute an operation with retry logic for transient failures
    async fn with_retry<T, F, Fut>(&self, operation: F) -> LastfmResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = LastfmResult<T>>,
    {
        let mut attempt = 0;
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if e.is_retryable() && attempt < self.max_retries => {
                    attempt += 1;
                    let delay_ms = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                    warn!(
                        attempt = attempt,
                        max_retries = self.max_retries,
                        delay_ms = delay_ms,
                        error = %e,
                        "Last.fm request failed, retrying"
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Make an API request and handle common error cases
    async fn make_request(&self, params: &[(&str, &str)]) -> LastfmResult<String> {
        let response = self
            .http_client
            .get(LASTFM_API_URL)
            .query(params)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LastfmError::Timeout
                } else {
                    LastfmError::Http(e)
                }
            })?;

        // Check for rate limiting
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Last.fm API rate limited");
            return Err(LastfmError::RateLimited);
        }

        response.text().await.map_err(LastfmError::Http)
    }

    /// Parse response text and handle API errors
    fn parse_api_error(&self, text: &str, artist_name: &str) -> Option<LastfmError> {
        if let Ok(error) = serde_json::from_str::<ErrorResponse>(text) {
            // Error code 6 = "Artist not found"
            if error.error == 6 {
                return Some(LastfmError::ArtistNotFound(artist_name.to_string()));
            }
            return Some(LastfmError::Api {
                code: error.error,
                message: error.message,
            });
        }
        None
    }

    /// Get similar artists for a given artist name
    ///
    /// # Arguments
    /// * `artist_name` - The artist name to find similar artists for
    /// * `limit` - Maximum number of similar artists to return (default: 10)
    ///
    /// # Errors
    /// - `LastfmError::InvalidInput` - If the artist name is empty or too long
    /// - `LastfmError::ArtistNotFound` - If the artist is not found
    /// - `LastfmError::Api` - If Last.fm returns an error
    /// - `LastfmError::Http` - If the HTTP request fails
    #[instrument(skip(self))]
    pub async fn get_similar_artists(
        &self,
        artist_name: &str,
        limit: Option<u32>,
    ) -> LastfmResult<Vec<SimilarArtist>> {
        let artist_name = Self::validate_artist_name(artist_name)?;
        let limit = limit.unwrap_or(DEFAULT_SIMILAR_LIMIT);
        let limit_str = limit.to_string();

        debug!(artist = %artist_name, limit, "Fetching similar artists from Last.fm");

        let text = self
            .with_retry(|| async {
                self.make_request(&[
                    ("method", "artist.getSimilar"),
                    ("artist", artist_name),
                    ("api_key", &self.api_key),
                    ("format", "json"),
                    ("limit", &limit_str),
                ])
                .await
            })
            .await?;

        // Check for API error response
        if let Some(error) = self.parse_api_error(&text, artist_name) {
            return Err(error);
        }

        // Parse as success response
        let response: SimilarArtistsResponse = serde_json::from_str(&text)?;

        let artists: Vec<SimilarArtist> = response
            .similarartists
            .artist
            .into_iter()
            .map(Into::into)
            .collect();

        debug!(
            artist = %artist_name,
            result_count = artists.len(),
            "Found similar artists"
        );

        Ok(artists)
    }

    /// Get top tags for a given artist
    ///
    /// # Arguments
    /// * `artist_name` - The artist name to get tags for
    ///
    /// # Errors
    /// - `LastfmError::InvalidInput` - If the artist name is empty or too long
    /// - `LastfmError::ArtistNotFound` - If the artist is not found
    /// - `LastfmError::Api` - If Last.fm returns an error
    /// - `LastfmError::Http` - If the HTTP request fails
    #[instrument(skip(self))]
    pub async fn get_artist_tags(&self, artist_name: &str) -> LastfmResult<Vec<ArtistTag>> {
        let artist_name = Self::validate_artist_name(artist_name)?;

        debug!(artist = %artist_name, "Fetching artist tags from Last.fm");

        let text = self
            .with_retry(|| async {
                self.make_request(&[
                    ("method", "artist.getTopTags"),
                    ("artist", artist_name),
                    ("api_key", &self.api_key),
                    ("format", "json"),
                ])
                .await
            })
            .await?;

        // Check for API error response
        if let Some(error) = self.parse_api_error(&text, artist_name) {
            return Err(error);
        }

        // Parse as success response
        let response: TopTagsResponse = serde_json::from_str(&text)?;

        let tags: Vec<ArtistTag> = response.toptags.tag.into_iter().map(Into::into).collect();

        debug!(
            artist = %artist_name,
            tag_count = tags.len(),
            "Found artist tags"
        );

        Ok(tags)
    }

    /// Check if the API key is valid by making a simple request
    ///
    /// Returns `ApiKeyStatus` indicating whether the key is valid, invalid,
    /// or if the check could not be completed due to network issues.
    pub async fn validate_api_key(&self) -> ApiKeyStatus {
        // Try to get similar artists for a well-known artist
        match self.get_similar_artists("The Beatles", Some(1)).await {
            Ok(_) => ApiKeyStatus::Valid,
            Err(LastfmError::Api { code: 10, .. }) => {
                // Error 10 = Invalid API key
                ApiKeyStatus::Invalid
            }
            Err(e) => ApiKeyStatus::Unknown(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_requires_api_key() {
        let result = LastfmClient::new("");
        assert!(matches!(result, Err(LastfmError::MissingApiKey)));
    }

    #[test]
    fn test_client_accepts_valid_api_key() {
        let result = LastfmClient::new("test_api_key");
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_debug_redacts_api_key() {
        let client = LastfmClient::new("secret_key").unwrap();
        let debug_str = format!("{:?}", client);
        assert!(!debug_str.contains("secret_key"));
        assert!(debug_str.contains("[REDACTED]"));
    }

    #[test]
    fn test_validate_artist_name_empty() {
        let result = LastfmClient::validate_artist_name("");
        assert!(matches!(result, Err(LastfmError::InvalidInput(_))));
    }

    #[test]
    fn test_validate_artist_name_whitespace_only() {
        let result = LastfmClient::validate_artist_name("   ");
        assert!(matches!(result, Err(LastfmError::InvalidInput(_))));
    }

    #[test]
    fn test_validate_artist_name_too_long() {
        let long_name = "a".repeat(MAX_ARTIST_NAME_LENGTH + 1);
        let result = LastfmClient::validate_artist_name(&long_name);
        assert!(matches!(result, Err(LastfmError::InvalidInput(_))));
    }

    #[test]
    fn test_validate_artist_name_valid() {
        let result = LastfmClient::validate_artist_name("  Radiohead  ");
        assert!(matches!(result, Ok("Radiohead")));
    }

    #[test]
    fn test_similar_artist_parsing() {
        use crate::models::RawSimilarArtist;

        let raw = RawSimilarArtist {
            name: "Test Artist".to_string(),
            mbid: Some("abc123".to_string()),
            match_score: "0.75".to_string(),
            url: Some("https://last.fm/artist/test".to_string()),
        };

        let artist: SimilarArtist = raw.into();
        assert_eq!(artist.name, "Test Artist");
        assert_eq!(artist.mbid, Some("abc123".to_string()));
        assert!((artist.match_score - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_empty_mbid_becomes_none() {
        use crate::models::RawSimilarArtist;

        let raw = RawSimilarArtist {
            name: "Test Artist".to_string(),
            mbid: Some("".to_string()),
            match_score: "0.5".to_string(),
            url: None,
        };

        let artist: SimilarArtist = raw.into();
        assert!(artist.mbid.is_none());
    }

    #[test]
    fn test_error_is_retryable() {
        assert!(LastfmError::Timeout.is_retryable());
        assert!(LastfmError::RateLimited.is_retryable());
        assert!(!LastfmError::MissingApiKey.is_retryable());
        assert!(!LastfmError::ArtistNotFound("test".to_string()).is_retryable());
    }

    #[test]
    fn test_api_key_status_equality() {
        assert_eq!(ApiKeyStatus::Valid, ApiKeyStatus::Valid);
        assert_eq!(ApiKeyStatus::Invalid, ApiKeyStatus::Invalid);
        assert_ne!(ApiKeyStatus::Valid, ApiKeyStatus::Invalid);
    }
}
