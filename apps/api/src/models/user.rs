//! User and authentication models for Resonance
//!
//! This module contains the database models for:
//! - User accounts with preferences
//! - Sessions and device tracking
//! - JWT claims and token structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User role enum matching PostgreSQL user_role type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    #[default]
    User,
    Guest,
}

/// User preferences stored as JSONB in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// UI theme: "dark" or "light"
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Audio quality: "low", "medium", "high", "lossless"
    #[serde(default = "default_quality")]
    pub quality: String,

    /// Crossfade duration in milliseconds
    #[serde(default)]
    pub crossfade_duration_ms: u32,

    /// Enable gapless playback between tracks
    #[serde(default = "default_true")]
    pub gapless_playback: bool,

    /// Normalize volume across tracks
    #[serde(default)]
    pub normalize_volume: bool,

    /// Show explicit content
    #[serde(default = "default_true")]
    pub show_explicit: bool,

    /// Private listening session (no scrobbling)
    #[serde(default)]
    pub private_session: bool,

    /// Discord Rich Presence integration
    #[serde(default = "default_true")]
    pub discord_rpc: bool,

    /// Enable ListenBrainz scrobbling
    #[serde(default)]
    pub listenbrainz_scrobble: bool,
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_quality() -> String {
    "high".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            quality: default_quality(),
            crossfade_duration_ms: 0,
            gapless_playback: true,
            normalize_volume: false,
            show_explicit: true,
            private_session: false,
            discord_rpc: true,
            listenbrainz_scrobble: false,
        }
    }
}

/// User account from the users table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    /// Unique user identifier
    pub id: Uuid,

    /// User's email address (unique)
    pub email: String,

    /// Argon2 hashed password
    #[serde(skip_serializing)]
    pub password_hash: String,

    /// Display name shown in the UI
    pub display_name: String,

    /// URL to user's avatar image
    pub avatar_url: Option<String>,

    /// User's role (admin, user, guest)
    pub role: UserRole,

    /// User preferences as JSONB
    #[sqlx(json)]
    pub preferences: UserPreferences,

    /// ListenBrainz API token for scrobbling
    #[serde(skip_serializing)]
    #[allow(dead_code)] // Used when ListenBrainz integration is implemented
    pub listenbrainz_token: Option<String>,

    /// Discord user ID for Rich Presence
    pub discord_user_id: Option<String>,

    /// Whether email has been verified
    pub email_verified: bool,

    /// Last time user was seen online
    pub last_seen_at: Option<DateTime<Utc>>,

    /// Account creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last profile update timestamp
    pub updated_at: DateTime<Utc>,

    /// When password was last changed
    pub password_updated_at: DateTime<Utc>,
}

/// Public user profile (safe to expose to other users)
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)] // Infrastructure for public profile features
pub struct PublicUser {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
        }
    }
}

/// Session record from the sessions table
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)] // Infrastructure for session management features
pub struct Session {
    /// Unique session identifier
    pub id: Uuid,

    /// User who owns this session
    pub user_id: Uuid,

    /// SHA-256 hash of the access token
    pub token_hash: String,

    /// SHA-256 hash of the refresh token
    pub refresh_token_hash: Option<String>,

    /// Human-readable device name
    pub device_name: Option<String>,

    /// Device type (desktop, mobile, tablet, web, tv)
    pub device_type: Option<String>,

    /// Unique device identifier
    pub device_id: Option<String>,

    /// Client IP address (stored as string from INET type)
    pub ip_address: Option<String>,

    /// Client user agent string
    pub user_agent: Option<String>,

    /// Whether session is currently active
    pub is_active: bool,

    /// Last activity timestamp
    pub last_active_at: DateTime<Utc>,

    /// Session expiration timestamp
    pub expires_at: DateTime<Utc>,

    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Device information for session creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Human-readable device name (e.g., "iPhone 15 Pro")
    pub device_name: Option<String>,

    /// Device type category
    pub device_type: Option<DeviceType>,

    /// Unique device identifier (for device limits)
    pub device_id: Option<String>,
}

/// Device type categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Desktop,
    Mobile,
    Tablet,
    Web,
    Tv,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Desktop => write!(f, "desktop"),
            Self::Mobile => write!(f, "mobile"),
            Self::Tablet => write!(f, "tablet"),
            Self::Web => write!(f, "web"),
            Self::Tv => write!(f, "tv"),
        }
    }
}

/// JWT claims payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: Uuid,

    /// User's email
    pub email: String,

    /// User's role
    pub role: UserRole,

    /// Session ID
    pub sid: Uuid,

    /// Issued at timestamp (Unix epoch)
    pub iat: i64,

    /// Expiration timestamp (Unix epoch)
    pub exp: i64,

    /// Issuer
    #[serde(default = "default_issuer")]
    pub iss: String,

    /// Audience
    #[serde(default = "default_audience")]
    pub aud: String,
}

fn default_issuer() -> String {
    "resonance".to_string()
}

fn default_audience() -> String {
    "resonance".to_string()
}

impl Claims {
    /// Create new claims for a user session
    pub fn new(user: &User, session_id: Uuid, access_token_ttl_secs: i64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: user.id,
            email: user.email.clone(),
            role: user.role,
            sid: session_id,
            iat: now,
            exp: now + access_token_ttl_secs,
            iss: default_issuer(),
            aud: default_audience(),
        }
    }

    /// Check if the token has expired
    #[allow(dead_code)] // Useful helper for token validation
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

/// Authentication tokens returned after login
#[derive(Debug, Clone, Serialize)]
pub struct AuthTokens {
    /// JWT access token for API authentication
    pub access_token: String,

    /// Refresh token for obtaining new access tokens
    pub refresh_token: String,

    /// Access token expiration timestamp
    pub expires_at: DateTime<Utc>,

    /// Token type (always "Bearer")
    pub token_type: &'static str,
}

impl AuthTokens {
    /// Create a new AuthTokens instance
    pub fn new(access_token: String, refresh_token: String, expires_at: DateTime<Utc>) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_at,
            token_type: "Bearer",
        }
    }
}

/// HTTP request metadata extracted from headers for audit trails
///
/// This struct captures client information from HTTP requests
/// to provide context for sessions and audit logging.
#[derive(Debug, Clone, Default)]
pub struct RequestMetadata {
    /// Client IP address (may be from X-Forwarded-For behind proxy)
    pub ip_address: Option<String>,

    /// Client user agent string
    pub user_agent: Option<String>,
}

impl RequestMetadata {
    /// Create new request metadata with IP and user agent
    pub fn new(ip_address: Option<String>, user_agent: Option<String>) -> Self {
        Self {
            ip_address,
            user_agent,
        }
    }
}

/// Refresh token claims (simpler than access token claims)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshClaims {
    /// Subject (user ID)
    pub sub: Uuid,

    /// Session ID
    pub sid: Uuid,

    /// JWT ID - unique identifier for token rotation
    pub jti: Uuid,

    /// Issued at timestamp
    pub iat: i64,

    /// Expiration timestamp
    pub exp: i64,

    /// Token type identifier
    pub typ: String,

    /// Issuer
    #[serde(default = "default_issuer")]
    pub iss: String,

    /// Audience
    #[serde(default = "default_audience")]
    pub aud: String,
}

impl RefreshClaims {
    /// Create new refresh token claims
    pub fn new(user_id: Uuid, session_id: Uuid, refresh_token_ttl_secs: i64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: user_id,
            sid: session_id,
            jti: Uuid::new_v4(), // Unique ID ensures token rotation produces different tokens
            iat: now,
            exp: now + refresh_token_ttl_secs,
            typ: "refresh".to_string(),
            iss: default_issuer(),
            aud: default_audience(),
        }
    }

    /// Check if the refresh token has expired
    #[allow(dead_code)] // Useful helper for token validation
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_default() {
        assert_eq!(UserRole::default(), UserRole::User);
    }

    #[test]
    fn test_user_preferences_default() {
        let prefs = UserPreferences::default();
        assert_eq!(prefs.theme, "dark");
        assert_eq!(prefs.quality, "high");
        assert_eq!(prefs.crossfade_duration_ms, 0);
        assert!(prefs.gapless_playback);
        assert!(!prefs.normalize_volume);
        assert!(prefs.show_explicit);
        assert!(!prefs.private_session);
        assert!(prefs.discord_rpc);
        assert!(!prefs.listenbrainz_scrobble);
    }

    #[test]
    fn test_claims_is_expired() {
        let mut claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            role: UserRole::User,
            sid: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
            exp: Utc::now().timestamp() + 3600,
            iss: "resonance".to_string(),
            aud: "resonance".to_string(),
        };
        assert!(!claims.is_expired());

        // Expired token
        claims.exp = Utc::now().timestamp() - 1;
        assert!(claims.is_expired());
    }

    #[test]
    fn test_device_type_display() {
        assert_eq!(DeviceType::Desktop.to_string(), "desktop");
        assert_eq!(DeviceType::Mobile.to_string(), "mobile");
        assert_eq!(DeviceType::Tablet.to_string(), "tablet");
        assert_eq!(DeviceType::Web.to_string(), "web");
        assert_eq!(DeviceType::Tv.to_string(), "tv");
    }

    #[test]
    fn test_auth_tokens_new() {
        let tokens = AuthTokens::new("access".to_string(), "refresh".to_string(), Utc::now());
        assert_eq!(tokens.token_type, "Bearer");
        assert_eq!(tokens.access_token, "access");
        assert_eq!(tokens.refresh_token, "refresh");
    }

    #[test]
    fn test_public_user_from_user() {
        let now = Utc::now();
        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            display_name: "Test User".to_string(),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            role: UserRole::User,
            preferences: UserPreferences::default(),
            listenbrainz_token: Some("secret".to_string()),
            discord_user_id: None,
            email_verified: true,
            last_seen_at: None,
            created_at: now,
            updated_at: now,
            password_updated_at: now,
        };

        let public: PublicUser = user.into();
        assert_eq!(public.display_name, "Test User");
        assert!(public.avatar_url.is_some());
    }
}
