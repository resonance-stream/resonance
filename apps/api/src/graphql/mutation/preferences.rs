//! User preferences mutations for Resonance GraphQL API
//!
//! This module provides mutations for user preference management:
//! - updatePreferences: Update user preferences with validation
//! - resetPreferences: Reset preferences to default values
//!
//! Preferences include:
//! - theme: UI theme (dark/light)
//! - quality: Audio quality (low/medium/high/lossless)
//! - crossfade_duration_ms: Crossfade duration between tracks
//! - gapless_playback: Enable gapless playback
//! - normalize_volume: Enable volume normalization
//! - show_explicit: Show explicit content
//! - private_session: Enable private listening (no scrobbling)
//! - discord_rpc: Enable Discord Rich Presence
//! - listenbrainz_scrobble: Enable ListenBrainz scrobbling

use async_graphql::{Context, InputObject, Object, Result};

use crate::graphql::types::{User, UserPreferencesType};
use crate::models::user::{Claims, UserPreferences};
use crate::repositories::UserRepository;

// =============================================================================
// Validation Constants
// =============================================================================

/// Valid theme values
const VALID_THEMES: &[&str] = &["dark", "light"];

/// Valid audio quality values
const VALID_QUALITIES: &[&str] = &["low", "medium", "high", "lossless"];

/// Maximum crossfade duration in milliseconds (12 seconds)
const MAX_CROSSFADE_MS: u32 = 12_000;

// =============================================================================
// Input Types
// =============================================================================

/// Input for updating user preferences
///
/// All fields are optional - only provided fields will be updated.
/// Validation is performed on each field when provided.
#[derive(Debug, InputObject)]
pub struct UpdatePreferencesInput {
    /// UI theme: "dark" or "light"
    pub theme: Option<String>,

    /// Audio quality: "low", "medium", "high", or "lossless"
    pub quality: Option<String>,

    /// Crossfade duration in milliseconds (0-12000)
    /// 0 means crossfade is disabled
    pub crossfade_duration_ms: Option<u32>,

    /// Enable gapless playback between tracks
    pub gapless_playback: Option<bool>,

    /// Normalize volume across tracks
    pub normalize_volume: Option<bool>,

    /// Show explicit content
    pub show_explicit: Option<bool>,

    /// Private listening session (no scrobbling)
    pub private_session: Option<bool>,

    /// Discord Rich Presence integration
    pub discord_rpc: Option<bool>,

    /// Enable ListenBrainz scrobbling
    pub listenbrainz_scrobble: Option<bool>,
}

impl UpdatePreferencesInput {
    /// Check if any field is provided
    fn has_any_field(&self) -> bool {
        self.theme.is_some()
            || self.quality.is_some()
            || self.crossfade_duration_ms.is_some()
            || self.gapless_playback.is_some()
            || self.normalize_volume.is_some()
            || self.show_explicit.is_some()
            || self.private_session.is_some()
            || self.discord_rpc.is_some()
            || self.listenbrainz_scrobble.is_some()
    }
}

// =============================================================================
// Validation Helpers
// =============================================================================

/// Validate theme value
fn validate_theme(theme: &str) -> Result<()> {
    let normalized = theme.trim().to_lowercase();
    if !VALID_THEMES.contains(&normalized.as_str()) {
        return Err(async_graphql::Error::new(format!(
            "Invalid theme '{}'. Valid values: {}",
            theme,
            VALID_THEMES.join(", ")
        )));
    }
    Ok(())
}

/// Validate audio quality value
fn validate_quality(quality: &str) -> Result<()> {
    let normalized = quality.trim().to_lowercase();
    if !VALID_QUALITIES.contains(&normalized.as_str()) {
        return Err(async_graphql::Error::new(format!(
            "Invalid quality '{}'. Valid values: {}",
            quality,
            VALID_QUALITIES.join(", ")
        )));
    }
    Ok(())
}

/// Validate crossfade duration
fn validate_crossfade(duration_ms: u32) -> Result<()> {
    if duration_ms > MAX_CROSSFADE_MS {
        return Err(async_graphql::Error::new(format!(
            "Crossfade duration cannot exceed {} ms (12 seconds)",
            MAX_CROSSFADE_MS
        )));
    }
    Ok(())
}

/// Validate the entire input
fn validate_input(input: &UpdatePreferencesInput) -> Result<()> {
    if let Some(ref theme) = input.theme {
        validate_theme(theme)?;
    }
    if let Some(ref quality) = input.quality {
        validate_quality(quality)?;
    }
    if let Some(duration) = input.crossfade_duration_ms {
        validate_crossfade(duration)?;
    }
    Ok(())
}

/// Apply input updates to existing preferences
fn apply_updates(mut prefs: UserPreferences, input: &UpdatePreferencesInput) -> UserPreferences {
    if let Some(ref theme) = input.theme {
        prefs.theme = theme.trim().to_lowercase();
    }
    if let Some(ref quality) = input.quality {
        prefs.quality = quality.trim().to_lowercase();
    }
    if let Some(duration) = input.crossfade_duration_ms {
        prefs.crossfade_duration_ms = duration;
    }
    if let Some(gapless) = input.gapless_playback {
        prefs.gapless_playback = gapless;
    }
    if let Some(normalize) = input.normalize_volume {
        prefs.normalize_volume = normalize;
    }
    if let Some(explicit) = input.show_explicit {
        prefs.show_explicit = explicit;
    }
    if let Some(private) = input.private_session {
        prefs.private_session = private;
    }
    if let Some(discord) = input.discord_rpc {
        prefs.discord_rpc = discord;
    }
    if let Some(listenbrainz) = input.listenbrainz_scrobble {
        prefs.listenbrainz_scrobble = listenbrainz;
    }
    prefs
}

// =============================================================================
// Mutations
// =============================================================================

/// User preferences mutations
#[derive(Default)]
pub struct PreferencesMutation;

#[Object]
impl PreferencesMutation {
    /// Update user preferences
    ///
    /// Updates the authenticated user's preferences. Only provided fields
    /// will be updated; omitted fields retain their current values.
    ///
    /// # Arguments
    /// * `input` - The preferences to update
    ///
    /// # Returns
    /// The updated user with new preferences
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if no fields are provided
    /// - Returns error if theme is invalid (must be "dark" or "light")
    /// - Returns error if quality is invalid (must be "low", "medium", "high", or "lossless")
    /// - Returns error if crossfade duration exceeds 12000 ms
    ///
    /// # Example
    /// ```graphql
    /// mutation {
    ///   updatePreferences(input: {
    ///     theme: "dark"
    ///     quality: "lossless"
    ///     crossfadeDurationMs: 3000
    ///   }) {
    ///     id
    ///     preferences {
    ///       theme
    ///       quality
    ///       crossfadeDurationMs
    ///     }
    ///   }
    /// }
    /// ```
    async fn update_preferences(
        &self,
        ctx: &Context<'_>,
        input: UpdatePreferencesInput,
    ) -> Result<User> {
        // Get authenticated user
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        // Validate that at least one field is provided
        if !input.has_any_field() {
            return Err(async_graphql::Error::new(
                "At least one preference field must be provided",
            ));
        }

        // Validate input values
        validate_input(&input)?;

        let user_repo = ctx.data::<UserRepository>()?;

        // Get current user to merge preferences
        let user = user_repo
            .find_by_id(claims.sub)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %claims.sub, "Failed to fetch user");
                async_graphql::Error::new("Failed to fetch user preferences")
            })?
            .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        // Apply updates to existing preferences
        let updated_prefs = apply_updates(user.preferences.clone(), &input);

        // Serialize to JSON for storage
        let prefs_json = serde_json::to_value(&updated_prefs).map_err(|e| {
            tracing::error!(error = %e, "Failed to serialize preferences");
            async_graphql::Error::new("Failed to update preferences")
        })?;

        // Update in database
        user_repo
            .update_preferences(claims.sub, &prefs_json)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %claims.sub, "Failed to update preferences");
                async_graphql::Error::new("Failed to update preferences")
            })?;

        // Fetch updated user
        let updated_user = user_repo
            .find_by_id(claims.sub)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %claims.sub, "Failed to fetch updated user");
                async_graphql::Error::new("Failed to fetch updated user")
            })?
            .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        tracing::info!(
            user_id = %claims.sub,
            "User preferences updated successfully"
        );

        Ok(User::from(updated_user))
    }

    /// Reset user preferences to default values
    ///
    /// Resets all preferences for the authenticated user to their default values:
    /// - theme: "dark"
    /// - quality: "high"
    /// - crossfade_duration_ms: 0
    /// - gapless_playback: true
    /// - normalize_volume: false
    /// - show_explicit: true
    /// - private_session: false
    /// - discord_rpc: true
    /// - listenbrainz_scrobble: false
    ///
    /// # Returns
    /// The updated user with default preferences
    ///
    /// # Errors
    /// - Returns error if not authenticated
    ///
    /// # Example
    /// ```graphql
    /// mutation {
    ///   resetPreferences {
    ///     id
    ///     preferences {
    ///       theme
    ///       quality
    ///     }
    ///   }
    /// }
    /// ```
    async fn reset_preferences(&self, ctx: &Context<'_>) -> Result<User> {
        // Get authenticated user
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let user_repo = ctx.data::<UserRepository>()?;

        // Create default preferences
        let default_prefs = UserPreferences::default();

        // Serialize to JSON for storage
        let prefs_json = serde_json::to_value(&default_prefs).map_err(|e| {
            tracing::error!(error = %e, "Failed to serialize default preferences");
            async_graphql::Error::new("Failed to reset preferences")
        })?;

        // Update in database
        user_repo
            .update_preferences(claims.sub, &prefs_json)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %claims.sub, "Failed to reset preferences");
                async_graphql::Error::new("Failed to reset preferences")
            })?;

        // Fetch updated user
        let updated_user = user_repo
            .find_by_id(claims.sub)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %claims.sub, "Failed to fetch updated user");
                async_graphql::Error::new("Failed to fetch updated user")
            })?
            .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        tracing::info!(
            user_id = %claims.sub,
            "User preferences reset to defaults"
        );

        Ok(User::from(updated_user))
    }

    /// Get the current user's preferences
    ///
    /// A convenience query available as a mutation for consistency with
    /// the preferences API. For normal querying, use the `me.preferences` query.
    ///
    /// # Returns
    /// The current user's preferences
    ///
    /// # Errors
    /// - Returns error if not authenticated
    async fn get_preferences(&self, ctx: &Context<'_>) -> Result<UserPreferencesType> {
        // Get authenticated user
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let user_repo = ctx.data::<UserRepository>()?;

        // Fetch user preferences
        let user = user_repo
            .find_by_id(claims.sub)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %claims.sub, "Failed to fetch user");
                async_graphql::Error::new("Failed to fetch preferences")
            })?
            .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        Ok(UserPreferencesType::from(user.preferences))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_theme_valid() {
        assert!(validate_theme("dark").is_ok());
        assert!(validate_theme("light").is_ok());
        assert!(validate_theme("DARK").is_ok());
        assert!(validate_theme("  Light  ").is_ok());
    }

    #[test]
    fn test_validate_theme_invalid() {
        assert!(validate_theme("auto").is_err());
        assert!(validate_theme("system").is_err());
        assert!(validate_theme("").is_err());
    }

    #[test]
    fn test_validate_quality_valid() {
        assert!(validate_quality("low").is_ok());
        assert!(validate_quality("medium").is_ok());
        assert!(validate_quality("high").is_ok());
        assert!(validate_quality("lossless").is_ok());
        assert!(validate_quality("HIGH").is_ok());
        assert!(validate_quality("  Lossless  ").is_ok());
    }

    #[test]
    fn test_validate_quality_invalid() {
        assert!(validate_quality("ultra").is_err());
        assert!(validate_quality("normal").is_err());
        assert!(validate_quality("").is_err());
    }

    #[test]
    fn test_validate_crossfade_valid() {
        assert!(validate_crossfade(0).is_ok());
        assert!(validate_crossfade(3000).is_ok());
        assert!(validate_crossfade(12000).is_ok());
    }

    #[test]
    fn test_validate_crossfade_invalid() {
        assert!(validate_crossfade(12001).is_err());
        assert!(validate_crossfade(15000).is_err());
    }

    #[test]
    fn test_apply_updates() {
        let prefs = UserPreferences::default();
        let input = UpdatePreferencesInput {
            theme: Some("light".to_string()),
            quality: Some("lossless".to_string()),
            crossfade_duration_ms: Some(3000),
            gapless_playback: None,
            normalize_volume: Some(true),
            show_explicit: None,
            private_session: None,
            discord_rpc: None,
            listenbrainz_scrobble: None,
        };

        let updated = apply_updates(prefs, &input);

        assert_eq!(updated.theme, "light");
        assert_eq!(updated.quality, "lossless");
        assert_eq!(updated.crossfade_duration_ms, 3000);
        assert!(updated.gapless_playback); // Unchanged from default
        assert!(updated.normalize_volume); // Updated
    }

    #[test]
    fn test_has_any_field() {
        let empty = UpdatePreferencesInput {
            theme: None,
            quality: None,
            crossfade_duration_ms: None,
            gapless_playback: None,
            normalize_volume: None,
            show_explicit: None,
            private_session: None,
            discord_rpc: None,
            listenbrainz_scrobble: None,
        };
        assert!(!empty.has_any_field());

        let with_theme = UpdatePreferencesInput {
            theme: Some("dark".to_string()),
            ..empty
        };
        assert!(with_theme.has_any_field());
    }
}
