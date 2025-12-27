//! User and authentication GraphQL types
//!
//! This module defines the GraphQL types for user data and authentication payloads.

use async_graphql::{Enum, Object, SimpleObject};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::user::{
    AuthTokens, User as DbUser, UserPreferences as DbUserPreferences, UserRole as DbUserRole,
};

/// User role enum for GraphQL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum UserRole {
    /// Administrator with full access
    Admin,
    /// Regular user
    User,
    /// Guest with limited access
    Guest,
}

impl From<DbUserRole> for UserRole {
    fn from(role: DbUserRole) -> Self {
        match role {
            DbUserRole::Admin => Self::Admin,
            DbUserRole::User => Self::User,
            DbUserRole::Guest => Self::Guest,
        }
    }
}

/// User preferences exposed via GraphQL
#[derive(Debug, Clone, SimpleObject)]
pub struct UserPreferencesType {
    /// UI theme: "dark" or "light"
    pub theme: String,
    /// Audio quality: "low", "medium", "high", "lossless"
    pub quality: String,
    /// Crossfade duration in milliseconds
    pub crossfade_duration_ms: u32,
    /// Enable gapless playback between tracks
    pub gapless_playback: bool,
    /// Normalize volume across tracks
    pub normalize_volume: bool,
    /// Show explicit content
    pub show_explicit: bool,
    /// Private listening session (no scrobbling)
    pub private_session: bool,
    /// Discord Rich Presence integration
    pub discord_rpc: bool,
    /// Enable ListenBrainz scrobbling
    pub listenbrainz_scrobble: bool,
}

impl From<DbUserPreferences> for UserPreferencesType {
    fn from(prefs: DbUserPreferences) -> Self {
        Self {
            theme: prefs.theme,
            quality: prefs.quality,
            crossfade_duration_ms: prefs.crossfade_duration_ms,
            gapless_playback: prefs.gapless_playback,
            normalize_volume: prefs.normalize_volume,
            show_explicit: prefs.show_explicit,
            private_session: prefs.private_session,
            discord_rpc: prefs.discord_rpc,
            listenbrainz_scrobble: prefs.listenbrainz_scrobble,
        }
    }
}

/// User account information exposed via GraphQL
pub struct User {
    inner: DbUser,
}

impl User {
    /// Create a new GraphQL User from a database User
    pub fn new(user: DbUser) -> Self {
        Self { inner: user }
    }
}

impl From<DbUser> for User {
    fn from(user: DbUser) -> Self {
        Self::new(user)
    }
}

#[Object]
impl User {
    /// Unique user identifier
    async fn id(&self) -> Uuid {
        self.inner.id
    }

    /// User's email address
    async fn email(&self) -> &str {
        &self.inner.email
    }

    /// Display name shown in the UI
    async fn display_name(&self) -> &str {
        &self.inner.display_name
    }

    /// URL to user's avatar image
    async fn avatar_url(&self) -> Option<&str> {
        self.inner.avatar_url.as_deref()
    }

    /// User's role (admin, user, guest)
    async fn role(&self) -> UserRole {
        self.inner.role.into()
    }

    /// User preferences
    async fn preferences(&self) -> UserPreferencesType {
        self.inner.preferences.clone().into()
    }

    /// Discord user ID for Rich Presence (if connected)
    async fn discord_user_id(&self) -> Option<&str> {
        self.inner.discord_user_id.as_deref()
    }

    /// Whether email has been verified
    async fn email_verified(&self) -> bool {
        self.inner.email_verified
    }

    /// Last time user was seen online
    async fn last_seen_at(&self) -> Option<DateTime<Utc>> {
        self.inner.last_seen_at
    }

    /// Account creation timestamp
    async fn created_at(&self) -> DateTime<Utc> {
        self.inner.created_at
    }

    /// Last profile update timestamp
    async fn updated_at(&self) -> DateTime<Utc> {
        self.inner.updated_at
    }
}

/// Authentication payload returned after login or registration
#[derive(Debug, Clone, SimpleObject)]
pub struct AuthPayload {
    /// The authenticated user
    #[graphql(flatten)]
    pub user: AuthPayloadUser,
    /// JWT access token for API authentication
    pub access_token: String,
    /// Refresh token for obtaining new access tokens
    pub refresh_token: String,
    /// Access token expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Token type (always "Bearer")
    pub token_type: String,
}

/// User data within auth payload (subset of full user)
#[derive(Debug, Clone, SimpleObject)]
pub struct AuthPayloadUser {
    /// Unique user identifier
    pub id: Uuid,
    /// User's email address
    pub email: String,
    /// Display name shown in the UI
    pub display_name: String,
    /// URL to user's avatar image
    pub avatar_url: Option<String>,
    /// User's role
    pub role: UserRole,
    /// Whether email has been verified
    pub email_verified: bool,
}

impl AuthPayload {
    /// Create a new AuthPayload from a database user and tokens
    pub fn new(user: DbUser, tokens: AuthTokens) -> Self {
        Self {
            user: AuthPayloadUser {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                avatar_url: user.avatar_url,
                role: user.role.into(),
                email_verified: user.email_verified,
            },
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at: tokens.expires_at,
            token_type: tokens.token_type.to_string(),
        }
    }
}

/// Token refresh payload returned after refreshing tokens
#[derive(Debug, Clone, SimpleObject)]
pub struct RefreshPayload {
    /// New JWT access token
    pub access_token: String,
    /// New refresh token (tokens are rotated)
    pub refresh_token: String,
    /// Access token expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Token type (always "Bearer")
    pub token_type: String,
}

impl From<AuthTokens> for RefreshPayload {
    fn from(tokens: AuthTokens) -> Self {
        Self {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at: tokens.expires_at,
            token_type: tokens.token_type.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::{UserPreferences, UserRole as DbRole};

    #[test]
    fn test_user_role_conversion() {
        assert!(matches!(UserRole::from(DbRole::Admin), UserRole::Admin));
        assert!(matches!(UserRole::from(DbRole::User), UserRole::User));
        assert!(matches!(UserRole::from(DbRole::Guest), UserRole::Guest));
    }

    #[test]
    fn test_user_preferences_conversion() {
        let db_prefs = UserPreferences::default();
        let gql_prefs: UserPreferencesType = db_prefs.into();

        assert_eq!(gql_prefs.theme, "dark");
        assert_eq!(gql_prefs.quality, "high");
        assert!(gql_prefs.gapless_playback);
        assert!(!gql_prefs.normalize_volume);
    }
}
