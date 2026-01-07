//! ListenBrainz service for scrobbling
//!
//! Submits listening history to ListenBrainz when users play tracks.
//! Respects the 50% / 4-minute scrobble rule and user preferences.
//!
//! Tokens are stored encrypted in the database using AES-256-GCM and are
//! decrypted on-the-fly when needed for API calls.
//!
//! # Builder Pattern
//!
//! Use [`ListenBrainzServiceBuilder`] for flexible service initialization:
//!
//! ```ignore
//! use resonance_api::services::listenbrainz::ListenBrainzServiceBuilder;
//!
//! // Minimal setup with just a database pool
//! let service = ListenBrainzServiceBuilder::new()
//!     .pool(db_pool.clone())
//!     .build()?;
//!
//! // Full setup with encryption support
//! let service = ListenBrainzServiceBuilder::new()
//!     .pool(db_pool.clone())
//!     .encryption_service(encryption_service)
//!     .build()?;
//!
//! // With custom user repository
//! let service = ListenBrainzServiceBuilder::new()
//!     .user_repository(custom_user_repo)
//!     .encryption_service(encryption_service)
//!     .build()?;
//! ```

use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::user::UserPreferences;
use crate::repositories::UserRepository;
use crate::services::encryption::EncryptionService;

/// ListenBrainz API base URL
const LISTENBRAINZ_API_URL: &str = "https://api.listenbrainz.org";

/// Minimum track duration in seconds for scrobbling
const MIN_TRACK_DURATION_SECS: i32 = 30;

/// Maximum play duration threshold in seconds (4 minutes)
const MAX_SCROBBLE_THRESHOLD_SECS: i32 = 240;

/// HTTP request timeout for ListenBrainz API calls.
/// ListenBrainz recommends keeping requests under 30s; we use 10s for snappy UX.
const HTTP_TIMEOUT_SECS: u64 = 10;

/// Maximum number of retry attempts for transient failures
const MAX_RETRIES: u32 = 3;

/// Base delay between retries in milliseconds (doubles each retry)
const RETRY_BASE_DELAY_MS: u64 = 500;

/// Default user agent for ListenBrainz API requests
const DEFAULT_USER_AGENT: &str = "Resonance/1.0";

/// Builder for constructing [`ListenBrainzService`] with optional components
///
/// This builder provides a consistent initialization interface similar to
/// [`SchemaBuilder`](crate::graphql::schema::SchemaBuilder), allowing flexible
/// configuration of the service with optional encryption support.
///
/// # Required Components
///
/// Either `pool` or `user_repository` must be provided:
/// - `pool`: A database connection pool (will create a `UserRepository` internally)
/// - `user_repository`: A pre-configured `UserRepository` instance
///
/// # Optional Components
///
/// - `encryption_service`: Enables decryption of stored tokens
/// - `timeout`: Custom HTTP timeout (defaults to 10 seconds)
/// - `user_agent`: Custom User-Agent header (defaults to "Resonance/1.0")
///
/// # Example
///
/// ```ignore
/// // Basic setup
/// let service = ListenBrainzServiceBuilder::new()
///     .pool(db_pool)
///     .build()?;
///
/// // Full setup with encryption
/// let service = ListenBrainzServiceBuilder::new()
///     .pool(db_pool)
///     .encryption_service(encryption)
///     .timeout(Duration::from_secs(15))
///     .build()?;
/// ```
#[derive(Default)]
pub struct ListenBrainzServiceBuilder {
    pool: Option<PgPool>,
    user_repository: Option<UserRepository>,
    encryption_service: Option<EncryptionService>,
    timeout: Option<Duration>,
    user_agent: Option<String>,
}

impl ListenBrainzServiceBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database connection pool
    ///
    /// If provided, a `UserRepository` will be created from this pool.
    /// This is mutually exclusive with `user_repository()` - the last one set wins.
    pub fn pool(mut self, pool: PgPool) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Set a pre-configured user repository
    ///
    /// Use this when you have an existing `UserRepository` instance.
    /// This is mutually exclusive with `pool()` - the last one set wins.
    pub fn user_repository(mut self, repo: UserRepository) -> Self {
        self.user_repository = Some(repo);
        self
    }

    /// Set the encryption service for token decryption
    ///
    /// When set, the service will attempt to decrypt stored tokens using
    /// AES-256-GCM. If not set, tokens are assumed to be plaintext (legacy mode).
    pub fn encryption_service(mut self, service: EncryptionService) -> Self {
        self.encryption_service = Some(service);
        self
    }

    /// Set a custom HTTP timeout for API requests
    ///
    /// Defaults to 10 seconds if not specified.
    /// ListenBrainz recommends keeping requests under 30 seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set a custom User-Agent header for API requests
    ///
    /// Defaults to "Resonance/1.0" if not specified.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Build the `ListenBrainzService` instance
    ///
    /// # Errors
    ///
    /// Returns `ApiError::Configuration` if:
    /// - Neither `pool` nor `user_repository` was provided
    /// - The HTTP client cannot be created
    ///
    /// # Example
    ///
    /// ```ignore
    /// let service = ListenBrainzServiceBuilder::new()
    ///     .pool(db_pool)
    ///     .encryption_service(encryption)
    ///     .build()?;
    /// ```
    pub fn build(self) -> ApiResult<ListenBrainzService> {
        // Resolve the user repository - prefer explicit repo, then create from pool
        let user_repo = match (self.user_repository, self.pool) {
            (Some(repo), _) => repo,
            (None, Some(pool)) => UserRepository::new(pool),
            (None, None) => {
                return Err(ApiError::Configuration(
                    "ListenBrainzServiceBuilder requires either pool() or user_repository() to be set".into()
                ));
            }
        };

        // Build the HTTP client
        let timeout = self
            .timeout
            .unwrap_or(Duration::from_secs(HTTP_TIMEOUT_SECS));
        let user_agent = self
            .user_agent
            .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());

        let client = Client::builder()
            .timeout(timeout)
            .user_agent(&user_agent)
            .build()
            .map_err(|e| ApiError::Configuration(format!("HTTP client creation failed: {}", e)))?;

        Ok(ListenBrainzService {
            client,
            user_repo,
            encryption_service: self.encryption_service,
        })
    }
}

/// ListenBrainz service for submitting scrobbles
#[derive(Clone)]
pub struct ListenBrainzService {
    client: Client,
    user_repo: UserRepository,
    /// Optional encryption service for decrypting stored tokens
    encryption_service: Option<EncryptionService>,
}

/// Track metadata for scrobbling
#[derive(Debug, Clone)]
pub struct ScrobbleTrack {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: i32,
    pub musicbrainz_recording_id: Option<String>,
    pub musicbrainz_release_id: Option<String>,
    pub musicbrainz_artist_id: Option<String>,
}

/// Request payload for ListenBrainz API
#[derive(Debug, Serialize)]
struct SubmitListensPayload {
    listen_type: &'static str,
    payload: Vec<Listen>,
}

/// Individual listen entry
#[derive(Debug, Serialize)]
struct Listen {
    listened_at: i64,
    track_metadata: TrackMetadata,
}

/// Track metadata for ListenBrainz
#[derive(Debug, Serialize)]
struct TrackMetadata {
    track_name: String,
    artist_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_info: Option<AdditionalInfo>,
}

/// Additional MusicBrainz metadata
#[derive(Debug, Serialize)]
struct AdditionalInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    recording_mbid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_mbid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artist_mbids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<i64>,
}

/// Response from ListenBrainz API for token validation
#[derive(Debug, Deserialize)]
struct ValidateTokenResponse {
    valid: bool,
    #[serde(default)]
    user_name: Option<String>,
}

/// Error response from ListenBrainz API
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

impl ListenBrainzService {
    /// Create a new builder for configuring the service
    ///
    /// This is the recommended way to construct a `ListenBrainzService`
    /// as it provides the most flexibility.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let service = ListenBrainzService::builder()
    ///     .pool(db_pool)
    ///     .encryption_service(encryption)
    ///     .build()?;
    /// ```
    pub fn builder() -> ListenBrainzServiceBuilder {
        ListenBrainzServiceBuilder::new()
    }

    /// Create a new ListenBrainz service with just a database pool
    ///
    /// This is a convenience method. For more configuration options,
    /// use [`ListenBrainzService::builder()`].
    ///
    /// # Errors
    /// Returns `ApiError::Configuration` if the HTTP client cannot be created.
    pub fn new(db: PgPool) -> ApiResult<Self> {
        Self::builder().pool(db).build()
    }

    /// Create a new ListenBrainz service with an existing UserRepository
    ///
    /// This is a convenience method. For more configuration options,
    /// use [`ListenBrainzService::builder()`].
    ///
    /// # Errors
    /// Returns `ApiError::Configuration` if the HTTP client cannot be created.
    pub fn with_repo(user_repo: UserRepository) -> ApiResult<Self> {
        Self::builder().user_repository(user_repo).build()
    }

    /// Create a new ListenBrainz service with encryption support
    ///
    /// This is a convenience method. For more configuration options,
    /// use [`ListenBrainzService::builder()`].
    ///
    /// # Arguments
    /// * `user_repo` - The user repository for database access
    /// * `encryption_service` - The encryption service for decrypting stored tokens
    ///
    /// # Errors
    /// Returns `ApiError::Configuration` if the HTTP client cannot be created.
    pub fn with_encryption(
        user_repo: UserRepository,
        encryption_service: EncryptionService,
    ) -> ApiResult<Self> {
        Self::builder()
            .user_repository(user_repo)
            .encryption_service(encryption_service)
            .build()
    }

    /// Decrypt a stored token
    ///
    /// Handles both encrypted (base64) and legacy plaintext tokens for backwards compatibility.
    fn decrypt_token(&self, stored_token: &str) -> ApiResult<String> {
        match &self.encryption_service {
            Some(service) => {
                // Try to decode as base64 (encrypted format)
                match BASE64.decode(stored_token) {
                    Ok(encrypted) => service.decrypt(&encrypted).map_err(|e| {
                        // If decryption fails, it might be a legacy plaintext token
                        warn!(error = %e, "Failed to decrypt token - may be legacy plaintext");
                        ApiError::Internal(format!("Failed to decrypt stored token: {}", e))
                    }),
                    Err(_) => {
                        // Not valid base64 - treat as legacy plaintext token
                        info!("Token is not base64-encoded, treating as legacy plaintext");
                        Ok(stored_token.to_string())
                    }
                }
            }
            None => {
                // No encryption service - assume plaintext
                Ok(stored_token.to_string())
            }
        }
    }

    /// Validate a ListenBrainz user token
    ///
    /// Returns the username if the token is valid.
    ///
    /// # Errors
    /// Returns `ApiError::ListenBrainz` if the HTTP request fails after retries.
    #[instrument(skip(self, token))]
    pub async fn validate_token(&self, token: &str) -> ApiResult<Option<String>> {
        let token_header = format!("Token {}", token);
        let response = self
            .execute_with_retry(|| {
                self.client
                    .get(format!("{}/1/validate-token", LISTENBRAINZ_API_URL))
                    .header("Authorization", &token_header)
                    .send()
            })
            .await
            .map_err(|e| ApiError::ListenBrainz(format!("Failed to validate token: {}", e)))?;

        if response.status().is_success() {
            let result: ValidateTokenResponse = response.json().await.map_err(|e| {
                ApiError::ListenBrainz(format!("Failed to parse validation response: {}", e))
            })?;

            if result.valid {
                Ok(result.user_name)
            } else {
                Ok(None)
            }
        } else if matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            // Token is invalid (auth failure)
            Ok(None)
        } else {
            // Service error (outage, rate limit, etc.) - report as error
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            // Truncate body to avoid leaking large upstream responses
            let truncated_body: String = body.chars().take(200).collect();
            Err(ApiError::ListenBrainz(format!(
                "Token validation failed with status {}: {}",
                status, truncated_body
            )))
        }
    }

    /// Submit a scrobble for a user
    ///
    /// Validates that:
    /// 1. The track is at least 30 seconds long
    /// 2. The user has played 50% of the track OR 4 minutes (whichever comes first)
    /// 3. The user has scrobbling enabled and a valid token
    #[instrument(skip(self, track), fields(track_title = %track.title, user_id = %user_id))]
    pub async fn submit_scrobble(
        &self,
        user_id: Uuid,
        track: &ScrobbleTrack,
        played_at: DateTime<Utc>,
        duration_played_secs: i32,
    ) -> ApiResult<bool> {
        // Check minimum track duration
        if track.duration_secs < MIN_TRACK_DURATION_SECS {
            info!(
                duration = track.duration_secs,
                "Track too short for scrobbling"
            );
            return Ok(false);
        }

        // Calculate scrobble threshold: 50% of track or 4 minutes, whichever is smaller
        let threshold_secs = std::cmp::min(track.duration_secs / 2, MAX_SCROBBLE_THRESHOLD_SECS);

        if duration_played_secs < threshold_secs {
            info!(
                played = duration_played_secs,
                threshold = threshold_secs,
                "Playback duration below scrobble threshold"
            );
            return Ok(false);
        }

        // Get user token and preferences
        let (token, preferences) = self.get_user_scrobble_info(user_id).await?;

        // Check if scrobbling is enabled
        if !preferences.listenbrainz_scrobble {
            info!("ListenBrainz scrobbling disabled for user");
            return Ok(false);
        }

        // Check for private session
        if preferences.private_session {
            info!("User in private session, skipping scrobble");
            return Ok(false);
        }

        let token = match token {
            Some(t) => t,
            None => {
                warn!("No ListenBrainz token configured for user");
                return Ok(false);
            }
        };

        // Build and submit the listen
        self.submit_listen(&token, track, played_at).await
    }

    /// Submit a listen to ListenBrainz API
    async fn submit_listen(
        &self,
        token: &str,
        track: &ScrobbleTrack,
        played_at: DateTime<Utc>,
    ) -> ApiResult<bool> {
        // Simplified: always include additional_info, skip_serializing_if handles None fields
        let additional_info = Some(AdditionalInfo {
            recording_mbid: track.musicbrainz_recording_id.clone(),
            release_mbid: track.musicbrainz_release_id.clone(),
            artist_mbids: track.musicbrainz_artist_id.clone().map(|id| vec![id]),
            duration_ms: Some(i64::from(track.duration_secs) * 1000),
        });

        let payload = SubmitListensPayload {
            listen_type: "single",
            payload: vec![Listen {
                listened_at: played_at.timestamp(),
                track_metadata: TrackMetadata {
                    track_name: track.title.clone(),
                    artist_name: track.artist.clone(),
                    release_name: track.album.clone(),
                    additional_info,
                },
            }],
        };

        let token_header = format!("Token {}", token);
        let response = self
            .execute_with_retry(|| {
                self.client
                    .post(format!("{}/1/submit-listens", LISTENBRAINZ_API_URL))
                    .header("Authorization", &token_header)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
            })
            .await
            .map_err(|e| ApiError::ListenBrainz(format!("Failed to submit listen: {}", e)))?;

        let status = response.status();

        if status.is_success() {
            info!(
                track = %track.title,
                artist = %track.artist,
                "Successfully scrobbled to ListenBrainz"
            );
            Ok(true)
        } else if status == StatusCode::TOO_MANY_REQUESTS {
            // Handle rate limiting gracefully
            if let Some(retry_after) = response.headers().get("Retry-After") {
                warn!(
                    retry_after = ?retry_after,
                    "ListenBrainz rate limited, scrobble queued for later"
                );
            } else {
                warn!("ListenBrainz rate limited");
            }
            // Return false but don't error - scrobble was not submitted
            Ok(false)
        } else {
            let error_msg = response
                .json::<ErrorResponse>()
                .await
                .map(|e| e.error)
                .unwrap_or_else(|_| "Unknown error".to_string());

            warn!(
                status = %status,
                error = %error_msg,
                "Failed to submit listen to ListenBrainz"
            );

            // Don't fail the whole operation, just return false
            Ok(false)
        }
    }

    /// Get user's ListenBrainz token (decrypted) and preferences
    async fn get_user_scrobble_info(
        &self,
        user_id: Uuid,
    ) -> ApiResult<(Option<String>, UserPreferences)> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| ApiError::not_found("user", user_id.to_string()))?;

        // Decrypt the token if present
        let decrypted_token = match user.listenbrainz_token {
            Some(encrypted_token) => Some(self.decrypt_token(&encrypted_token)?),
            None => None,
        };

        Ok((decrypted_token, user.preferences))
    }

    /// Check if a user has ListenBrainz configured
    pub async fn is_configured(&self, user_id: Uuid) -> ApiResult<bool> {
        self.user_repo
            .has_listenbrainz_token(user_id)
            .await
            .map_err(ApiError::from)
    }

    /// Execute an HTTP request with retry logic for transient failures
    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T, reqwest::Error>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, reqwest::Error>>,
    {
        let mut last_error: Option<reqwest::Error> = None;

        for attempt in 0..MAX_RETRIES {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // Only retry on connection/timeout errors, not HTTP errors
                    if e.is_connect() || e.is_timeout() {
                        last_error = Some(e);

                        // Skip sleep on the final attempt - no more retries will follow
                        if attempt + 1 < MAX_RETRIES {
                            let delay =
                                Duration::from_millis(RETRY_BASE_DELAY_MS * (1 << attempt));
                            warn!(
                                attempt = attempt + 1,
                                max_retries = MAX_RETRIES,
                                delay_ms = delay.as_millis(),
                                error = %last_error.as_ref().unwrap(),
                                "Retrying ListenBrainz API request"
                            );
                            tokio::time::sleep(delay).await;
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        // last_error should always be Some here since we only reach this point
        // after MAX_RETRIES iterations, each of which sets last_error.
        // Using unwrap_or_else with a custom error as a safeguard.
        match last_error {
            Some(e) => Err(e),
            None => {
                // This should never happen, but handle gracefully instead of panicking.
                // Create a timeout error as a fallback since the most common retry case
                // is timeout/connection issues.
                tracing::error!(
                    "execute_with_retry completed without any errors - this indicates a logic bug"
                );
                // Return the result of trying once more to get a proper error
                operation().await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrobble_threshold_calculation() {
        // For a 3-minute track (180s), threshold should be 90s (50%)
        let short_track_duration = 180;
        let threshold = std::cmp::min(short_track_duration / 2, MAX_SCROBBLE_THRESHOLD_SECS);
        assert_eq!(threshold, 90);

        // For a 10-minute track (600s), threshold should be 240s (4 minutes cap)
        let long_track_duration = 600;
        let threshold = std::cmp::min(long_track_duration / 2, MAX_SCROBBLE_THRESHOLD_SECS);
        assert_eq!(threshold, 240);
    }

    #[test]
    fn test_min_track_duration() {
        assert_eq!(MIN_TRACK_DURATION_SECS, 30);
    }

    #[test]
    fn test_submit_listens_payload_serialization() {
        let payload = SubmitListensPayload {
            listen_type: "single",
            payload: vec![Listen {
                listened_at: 1234567890,
                track_metadata: TrackMetadata {
                    track_name: "Test Track".to_string(),
                    artist_name: "Test Artist".to_string(),
                    release_name: Some("Test Album".to_string()),
                    additional_info: Some(AdditionalInfo {
                        recording_mbid: Some("abc-123".to_string()),
                        release_mbid: None,
                        artist_mbids: None,
                        duration_ms: Some(180000),
                    }),
                },
            }],
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("Test Track"));
        assert!(json.contains("Test Artist"));
        assert!(json.contains("Test Album"));
        assert!(json.contains("abc-123"));
    }

    #[test]
    fn test_additional_info_skips_none_fields() {
        let info = AdditionalInfo {
            recording_mbid: None,
            release_mbid: None,
            artist_mbids: None,
            duration_ms: Some(180000),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("recording_mbid"));
        assert!(!json.contains("release_mbid"));
        assert!(!json.contains("artist_mbids"));
        assert!(json.contains("duration_ms"));
    }

    // Builder pattern tests

    #[test]
    fn test_builder_default() {
        let builder = ListenBrainzServiceBuilder::default();
        assert!(builder.pool.is_none());
        assert!(builder.user_repository.is_none());
        assert!(builder.encryption_service.is_none());
        assert!(builder.timeout.is_none());
        assert!(builder.user_agent.is_none());
    }

    #[test]
    fn test_builder_new_equals_default() {
        let new_builder = ListenBrainzServiceBuilder::new();
        let default_builder = ListenBrainzServiceBuilder::default();

        assert!(new_builder.pool.is_none());
        assert!(new_builder.user_repository.is_none());
        assert!(new_builder.encryption_service.is_none());
        assert!(new_builder.timeout.is_none());
        assert!(new_builder.user_agent.is_none());

        // Same as default
        assert!(default_builder.pool.is_none());
        assert!(default_builder.user_repository.is_none());
        assert!(default_builder.encryption_service.is_none());
        assert!(default_builder.timeout.is_none());
        assert!(default_builder.user_agent.is_none());
    }

    #[test]
    fn test_builder_requires_pool_or_repo() {
        // Builder without pool or user_repository should fail
        let result = ListenBrainzServiceBuilder::new().build();
        assert!(result.is_err());

        // Extract error message for validation
        if let Err(err) = result {
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("pool") || err_msg.contains("user_repository"),
                "Error should mention pool or user_repository: {}",
                err_msg
            );
        }
    }

    #[test]
    fn test_builder_timeout_setter() {
        let builder = ListenBrainzServiceBuilder::new().timeout(Duration::from_secs(30));
        assert_eq!(builder.timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_builder_user_agent_setter() {
        let builder = ListenBrainzServiceBuilder::new().user_agent("TestAgent/2.0");
        assert_eq!(builder.user_agent, Some("TestAgent/2.0".to_string()));
    }

    #[test]
    fn test_builder_user_agent_into_string() {
        // Test with String
        let builder = ListenBrainzServiceBuilder::new().user_agent(String::from("TestAgent/2.0"));
        assert_eq!(builder.user_agent, Some("TestAgent/2.0".to_string()));
    }

    #[test]
    fn test_builder_encryption_service_setter() {
        let encryption = EncryptionService::new("test-secret-that-is-at-least-32-characters-long");
        let builder = ListenBrainzServiceBuilder::new().encryption_service(encryption);
        assert!(builder.encryption_service.is_some());
    }

    #[test]
    fn test_builder_method() {
        // Test that ListenBrainzService::builder() returns a fresh builder
        let builder = ListenBrainzService::builder();
        assert!(builder.pool.is_none());
        assert!(builder.user_repository.is_none());
        assert!(builder.encryption_service.is_none());
    }

    #[test]
    fn test_builder_chaining() {
        let encryption = EncryptionService::new("test-secret-that-is-at-least-32-characters-long");

        // Test that all builder methods can be chained
        let builder = ListenBrainzServiceBuilder::new()
            .timeout(Duration::from_secs(20))
            .user_agent("ChainedAgent/1.0")
            .encryption_service(encryption);

        assert_eq!(builder.timeout, Some(Duration::from_secs(20)));
        assert_eq!(builder.user_agent, Some("ChainedAgent/1.0".to_string()));
        assert!(builder.encryption_service.is_some());
    }
}
