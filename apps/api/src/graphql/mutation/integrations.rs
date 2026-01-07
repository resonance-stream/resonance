//! Integration mutations for Resonance GraphQL API
//!
//! This module provides mutations for external service integrations:
//! - updateIntegrations: Update ListenBrainz/Discord settings
//! - submitScrobble: Submit a scrobble to ListenBrainz
//! - testListenbrainzConnection: Validate ListenBrainz token

use async_graphql::{Context, InputObject, Object, Result, SimpleObject, ID};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::models::user::Claims;
use crate::repositories::{TrackRepository, UserRepository};
use crate::services::encryption::EncryptionService;
use crate::services::listenbrainz::{ListenBrainzService, ScrobbleTrack};

// ============================================================================
// Validation Constants
// ============================================================================

/// Maximum token length (ListenBrainz tokens are 36 chars, allow some buffer)
const MAX_TOKEN_LENGTH: usize = 256;

/// Maximum allowed duration_played in seconds (24 hours - reasonable upper bound)
const MAX_DURATION_PLAYED_SECS: i32 = 86400;

/// Maximum age of played_at timestamp in seconds (7 days)
const MAX_PLAYED_AT_AGE_SECS: i64 = 604800;

// ============================================================================
// Error Handling
// ============================================================================

/// Sanitize database errors - log details internally, return generic message
fn sanitize_db_error(error: sqlx::Error, context: &str) -> async_graphql::Error {
    error!(error = %error, context, "Database error in integrations mutation");
    async_graphql::Error::new("An error occurred while processing your request")
}

/// Sanitize service errors - log details internally, return generic message
fn sanitize_service_error(error: impl std::fmt::Display, context: &str) -> async_graphql::Error {
    error!(error = %error, context, "Service error in integrations mutation");
    async_graphql::Error::new("An error occurred while connecting to external service")
}

// ============================================================================
// Input Types
// ============================================================================

/// Input for updating integration settings
#[derive(Debug, InputObject)]
pub struct UpdateIntegrationsInput {
    /// ListenBrainz user token (null to keep unchanged, empty string to remove)
    pub listenbrainz_token: Option<String>,
    /// Enable/disable ListenBrainz scrobbling
    pub listenbrainz_enabled: Option<bool>,
    /// Enable/disable Discord Rich Presence
    pub discord_rpc_enabled: Option<bool>,
}

/// Input for submitting a scrobble
#[derive(Debug, InputObject)]
pub struct ScrobbleInput {
    /// ID of the track that was played
    pub track_id: ID,
    /// When playback started (ISO 8601)
    pub played_at: DateTime<Utc>,
    /// Duration played in seconds (must be >= 0)
    pub duration_played: i32,
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of integration settings query/mutation
#[derive(Debug, SimpleObject)]
pub struct IntegrationsPayload {
    /// Whether a ListenBrainz token is configured (never exposes actual token)
    pub has_listenbrainz_token: bool,
    /// Whether ListenBrainz scrobbling is enabled
    pub listenbrainz_enabled: bool,
    /// ListenBrainz username (if token is valid and connected)
    pub listenbrainz_username: Option<String>,
    /// Whether Discord Rich Presence is enabled
    pub discord_rpc_enabled: bool,
}

/// Result of a scrobble submission
#[derive(Debug, SimpleObject)]
pub struct ScrobbleResult {
    /// Whether the scrobble was successfully submitted
    pub success: bool,
    /// Error message if scrobble failed
    pub error: Option<String>,
}

/// Result of ListenBrainz connection test
#[derive(Debug, SimpleObject)]
#[graphql(name = "ListenBrainzConnectionTestResult")]
pub struct ListenBrainzConnectionTestResult {
    /// Whether the connection is valid
    pub valid: bool,
    /// ListenBrainz username if valid
    pub username: Option<String>,
    /// Error message if invalid
    pub error: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get authenticated user ID from context
fn get_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    let claims = ctx
        .data_opt::<Claims>()
        .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;
    Ok(claims.sub)
}

/// Get ListenBrainz service from context, with graceful error if not configured
fn get_listenbrainz_service<'a>(ctx: &'a Context<'a>) -> Result<&'a ListenBrainzService> {
    ctx.data_opt::<ListenBrainzService>().ok_or_else(|| {
        warn!("ListenBrainz service not configured on this server");
        async_graphql::Error::new("ListenBrainz integration is not configured on this server")
    })
}

/// Get encryption service from context (optional - falls back to plaintext if not configured)
fn get_encryption_service<'a>(ctx: &'a Context<'a>) -> Option<&'a EncryptionService> {
    ctx.data_opt::<EncryptionService>()
}

/// Encrypt a token using the encryption service, returning base64-encoded ciphertext
///
/// If encryption service is not configured, returns the plaintext token.
/// This allows for backwards compatibility during migration.
fn encrypt_token(encryption_service: Option<&EncryptionService>, token: &str) -> Result<String> {
    match encryption_service {
        Some(service) => {
            let encrypted = service.encrypt(token).map_err(|e| {
                error!(error = %e, "Failed to encrypt token");
                async_graphql::Error::new("Failed to securely store token")
            })?;
            Ok(BASE64.encode(encrypted))
        }
        None => {
            // No encryption service configured - store plaintext (for backwards compatibility)
            warn!("Encryption service not configured, storing token in plaintext");
            Ok(token.to_string())
        }
    }
}

/// Decrypt a token using the encryption service
///
/// Handles both encrypted (base64) and legacy plaintext tokens for backwards compatibility.
/// If the token appears to be base64-encoded encrypted data, it will be decrypted.
/// Otherwise, it's treated as a legacy plaintext token.
///
/// For backward compatibility, if decryption fails (e.g., wrong key, corrupted data),
/// the original stored token is returned as-is, assuming it may be a legacy plaintext token.
fn decrypt_token(
    encryption_service: Option<&EncryptionService>,
    stored_token: &str,
) -> Result<String> {
    match encryption_service {
        Some(service) => {
            // Try to decode as base64 (encrypted format)
            match BASE64.decode(stored_token) {
                Ok(encrypted) => {
                    match service.decrypt(&encrypted) {
                        Ok(decrypted) => Ok(decrypted),
                        Err(e) => {
                            // If decryption fails, it might be a legacy plaintext token
                            // or data encrypted with a different key. Fall back to plaintext
                            // for backward compatibility during migration.
                            warn!(
                                error = %e,
                                "Failed to decrypt token - falling back to plaintext for backward compatibility"
                            );
                            Ok(stored_token.to_string())
                        }
                    }
                }
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

// ============================================================================
// Mutations
// ============================================================================

/// Integrations mutations
#[derive(Default)]
pub struct IntegrationsMutation;

#[Object]
impl IntegrationsMutation {
    /// Update integration settings
    ///
    /// Updates the user's ListenBrainz token and/or preference toggles.
    /// The token is validated before being saved.
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if token validation fails
    #[instrument(skip(self, ctx, input), fields(user_id))]
    async fn update_integrations(
        &self,
        ctx: &Context<'_>,
        input: UpdateIntegrationsInput,
    ) -> Result<IntegrationsPayload> {
        let user_id = get_user_id(ctx)?;
        tracing::Span::current().record("user_id", user_id.to_string());

        let user_repo = ctx.data::<UserRepository>()?;

        // Get current user
        let user = user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| sanitize_db_error(e, "find user"))?
            .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        let mut listenbrainz_username = None;

        // Update ListenBrainz token if provided
        if let Some(ref token) = input.listenbrainz_token {
            let token = token.trim();

            if token.is_empty() {
                // Remove token
                info!(user_id = %user_id, "Removing ListenBrainz token");
                user_repo
                    .update_listenbrainz_token(user_id, None)
                    .await
                    .map_err(|e| sanitize_db_error(e, "remove token"))?;

                // Ensure scrobbling is disabled when no token is configured
                if user.preferences.listenbrainz_scrobble {
                    let mut preferences = user.preferences.clone();
                    preferences.listenbrainz_scrobble = false;
                    let prefs_json = serde_json::to_value(&preferences).map_err(|e| {
                        error!(error = %e, "Failed to serialize preferences");
                        async_graphql::Error::new("Failed to update preferences")
                    })?;
                    user_repo
                        .update_preferences(user_id, &prefs_json)
                        .await
                        .map_err(|e| sanitize_db_error(e, "disable scrobbling on token removal"))?;
                    info!(user_id = %user_id, "Disabled scrobbling after token removal");
                }
            } else {
                // Validate token length
                if token.len() > MAX_TOKEN_LENGTH {
                    return Err(async_graphql::Error::new(
                        "Token exceeds maximum allowed length",
                    ));
                }

                // Validate token with ListenBrainz API
                let lb_service = get_listenbrainz_service(ctx)?;
                match lb_service.validate_token(token).await {
                    Ok(Some(username)) => {
                        // Token is valid, encrypt and save it
                        let encryption_service = get_encryption_service(ctx);
                        let encrypted_token = encrypt_token(encryption_service, token)?;

                        info!(user_id = %user_id, username = %username, encrypted = encryption_service.is_some(), "Saving valid ListenBrainz token");
                        user_repo
                            .update_listenbrainz_token(user_id, Some(&encrypted_token))
                            .await
                            .map_err(|e| sanitize_db_error(e, "save token"))?;
                        listenbrainz_username = Some(username);
                    }
                    Ok(None) => {
                        warn!(user_id = %user_id, "Invalid ListenBrainz token provided");
                        return Err(async_graphql::Error::new("Invalid ListenBrainz token"));
                    }
                    Err(e) => {
                        return Err(sanitize_service_error(e, "validate token"));
                    }
                }
            }
        }

        // Update preferences if provided
        let mut preferences = user.preferences.clone();
        let mut prefs_changed = false;

        if let Some(enabled) = input.listenbrainz_enabled {
            preferences.listenbrainz_scrobble = enabled;
            prefs_changed = true;
        }

        if let Some(enabled) = input.discord_rpc_enabled {
            preferences.discord_rpc = enabled;
            prefs_changed = true;
        }

        if prefs_changed {
            let prefs_json = serde_json::to_value(&preferences).map_err(|e| {
                error!(error = %e, "Failed to serialize preferences");
                async_graphql::Error::new("Failed to update preferences")
            })?;
            user_repo
                .update_preferences(user_id, &prefs_json)
                .await
                .map_err(|e| sanitize_db_error(e, "update preferences"))?;

            info!(
                user_id = %user_id,
                listenbrainz = preferences.listenbrainz_scrobble,
                discord = preferences.discord_rpc,
                "Updated integration preferences"
            );
        }

        // Re-fetch to get updated state
        let updated_user = user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| sanitize_db_error(e, "fetch updated user"))?
            .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        // If username is not set but token exists (token wasn't changed), fetch username (best-effort)
        if listenbrainz_username.is_none() {
            if let Some(ref encrypted_token) = updated_user.listenbrainz_token {
                // Use data_opt to avoid error when service is not configured
                if let Some(lb_service) = ctx.data_opt::<ListenBrainzService>() {
                    // Decrypt the token before validating
                    let encryption_service = get_encryption_service(ctx);
                    if let Ok(decrypted_token) = decrypt_token(encryption_service, encrypted_token)
                    {
                        if let Ok(Some(username)) =
                            lb_service.validate_token(&decrypted_token).await
                        {
                            listenbrainz_username = Some(username);
                        }
                    }
                } else {
                    warn!("ListenBrainz service not configured, skipping username fetch");
                }
            }
        }

        Ok(IntegrationsPayload {
            has_listenbrainz_token: updated_user.listenbrainz_token.is_some(),
            listenbrainz_enabled: updated_user.preferences.listenbrainz_scrobble,
            listenbrainz_username,
            discord_rpc_enabled: updated_user.preferences.discord_rpc,
        })
    }

    /// Submit a scrobble to ListenBrainz
    ///
    /// Called by the frontend when the scrobble threshold is reached
    /// (50% of track or 4 minutes, whichever comes first).
    ///
    /// # Arguments
    /// * `input` - Track info and playback details
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if track not found
    /// - Returns error if duration_played is invalid
    #[instrument(skip(self, ctx, input), fields(user_id, track_id))]
    async fn submit_scrobble(
        &self,
        ctx: &Context<'_>,
        input: ScrobbleInput,
    ) -> Result<ScrobbleResult> {
        let user_id = get_user_id(ctx)?;
        tracing::Span::current().record("user_id", user_id.to_string());

        // Validate duration_played
        if input.duration_played < 0 {
            return Err(async_graphql::Error::new(
                "duration_played must be non-negative",
            ));
        }
        if input.duration_played > MAX_DURATION_PLAYED_SECS {
            return Err(async_graphql::Error::new(
                "duration_played exceeds maximum allowed value",
            ));
        }

        // Validate played_at is not in the future and not too old
        let now = Utc::now();
        if input.played_at > now {
            return Err(async_graphql::Error::new(
                "played_at cannot be in the future",
            ));
        }
        let age_secs = (now - input.played_at).num_seconds();
        if age_secs > MAX_PLAYED_AT_AGE_SECS {
            return Err(async_graphql::Error::new(
                "played_at is too old (maximum 7 days)",
            ));
        }

        // Get ListenBrainz service
        let lb_service = get_listenbrainz_service(ctx)?;

        // Parse track ID
        let track_id: Uuid = input
            .track_id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid track ID format"))?;
        tracing::Span::current().record("track_id", track_id.to_string());

        // Get track info from repository
        let track_repo = ctx.data::<TrackRepository>()?;
        let track_info = track_repo
            .get_track_for_scrobble(track_id)
            .await
            .map_err(|e| sanitize_db_error(e, "fetch track"))?
            .ok_or_else(|| async_graphql::Error::new("Track not found"))?;

        let scrobble_track = ScrobbleTrack {
            title: track_info.title,
            artist: track_info.artist_name,
            album: track_info.album_title,
            duration_secs: (track_info.duration_ms / 1000) as i32,
            musicbrainz_recording_id: track_info.recording_mbid,
            musicbrainz_release_id: track_info.release_mbid,
            musicbrainz_artist_id: track_info.artist_mbid,
        };

        match lb_service
            .submit_scrobble(
                user_id,
                &scrobble_track,
                input.played_at,
                input.duration_played,
            )
            .await
        {
            Ok(true) => {
                info!(
                    user_id = %user_id,
                    track_id = %track_id,
                    "Scrobble submitted successfully"
                );
                Ok(ScrobbleResult {
                    success: true,
                    error: None,
                })
            }
            Ok(false) => Ok(ScrobbleResult {
                success: false,
                error: Some("Scrobble not submitted (threshold not met or disabled)".to_string()),
            }),
            Err(e) => {
                warn!(
                    user_id = %user_id,
                    track_id = %track_id,
                    error = %e,
                    "Scrobble submission failed"
                );
                Ok(ScrobbleResult {
                    success: false,
                    error: Some("Failed to submit scrobble".to_string()),
                })
            }
        }
    }

    /// Test ListenBrainz connection
    ///
    /// Validates the provided token against the ListenBrainz API.
    /// Does not save the token - use updateIntegrations to save.
    ///
    /// # Arguments
    /// * `token` - ListenBrainz user token to test
    #[instrument(skip(self, ctx, token))]
    async fn test_listenbrainz_connection(
        &self,
        ctx: &Context<'_>,
        token: String,
    ) -> Result<ListenBrainzConnectionTestResult> {
        // Require authentication
        let user_id = get_user_id(ctx)?;

        let token = token.trim();

        // Validate token length
        if token.is_empty() {
            return Ok(ListenBrainzConnectionTestResult {
                valid: false,
                username: None,
                error: Some("Token cannot be empty".to_string()),
            });
        }
        if token.len() > MAX_TOKEN_LENGTH {
            return Ok(ListenBrainzConnectionTestResult {
                valid: false,
                username: None,
                error: Some("Token exceeds maximum length".to_string()),
            });
        }

        let lb_service = get_listenbrainz_service(ctx)?;

        match lb_service.validate_token(token).await {
            Ok(Some(username)) => {
                info!(user_id = %user_id, username = %username, "ListenBrainz connection test successful");
                Ok(ListenBrainzConnectionTestResult {
                    valid: true,
                    username: Some(username),
                    error: None,
                })
            }
            Ok(None) => {
                info!(user_id = %user_id, "ListenBrainz connection test failed: invalid token");
                Ok(ListenBrainzConnectionTestResult {
                    valid: false,
                    username: None,
                    error: Some("Invalid token".to_string()),
                })
            }
            Err(e) => {
                warn!(user_id = %user_id, error = %e, "ListenBrainz connection test error");
                Ok(ListenBrainzConnectionTestResult {
                    valid: false,
                    username: None,
                    error: Some("Unable to connect to ListenBrainz".to_string()),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrations_payload_default() {
        let payload = IntegrationsPayload {
            has_listenbrainz_token: false,
            listenbrainz_enabled: false,
            listenbrainz_username: None,
            discord_rpc_enabled: true,
        };
        assert!(!payload.has_listenbrainz_token);
        assert!(payload.discord_rpc_enabled);
    }

    #[test]
    fn test_scrobble_result() {
        let success = ScrobbleResult {
            success: true,
            error: None,
        };
        assert!(success.success);

        let failure = ScrobbleResult {
            success: false,
            error: Some("Test error".to_string()),
        };
        assert!(!failure.success);
        assert_eq!(failure.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_connection_test_result() {
        let valid = ListenBrainzConnectionTestResult {
            valid: true,
            username: Some("testuser".to_string()),
            error: None,
        };
        assert!(valid.valid);
        assert_eq!(valid.username, Some("testuser".to_string()));

        let invalid = ListenBrainzConnectionTestResult {
            valid: false,
            username: None,
            error: Some("Invalid token".to_string()),
        };
        assert!(!invalid.valid);
        assert!(invalid.error.is_some());
    }

    #[test]
    fn test_validation_constants() {
        // Verify constants have expected values
        // ListenBrainz tokens are 36 chars, so MAX_TOKEN_LENGTH must be >= 36
        const _: () = assert!(MAX_TOKEN_LENGTH > 36);
        assert_eq!(MAX_DURATION_PLAYED_SECS, 86400); // 24 hours
        assert_eq!(MAX_PLAYED_AT_AGE_SECS, 604800); // 7 days
    }
}
