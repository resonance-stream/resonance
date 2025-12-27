//! Authentication service for Resonance
//!
//! This module provides comprehensive authentication functionality:
//! - User registration with Argon2id password hashing
//! - Login with JWT access/refresh token generation
//! - Token refresh and verification
//! - Session management and logout

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::user::{
    AuthTokens, Claims, DeviceInfo, RefreshClaims, User, UserPreferences, UserRole,
};

/// Authentication service configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT signing secret
    pub jwt_secret: String,
    /// Access token TTL in seconds (default: 15 minutes)
    pub access_token_ttl_secs: i64,
    /// Refresh token TTL in seconds (default: 7 days)
    pub refresh_token_ttl_secs: i64,
    /// JWT issuer
    pub issuer: String,
    /// JWT audience
    pub audience: String,
}

impl AuthConfig {
    /// Create a new AuthConfig with default TTLs
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_secret,
            access_token_ttl_secs: 15 * 60,        // 15 minutes
            refresh_token_ttl_secs: 7 * 24 * 3600, // 7 days
            issuer: "resonance".to_string(),
            audience: "resonance".to_string(),
        }
    }

    /// Create AuthConfig from expiry strings (e.g., "15m", "7d")
    pub fn with_expiry_strings(
        jwt_secret: String,
        access_expiry: &str,
        refresh_expiry: &str,
    ) -> Self {
        Self {
            jwt_secret,
            access_token_ttl_secs: parse_duration_string(access_expiry).unwrap_or(15 * 60),
            refresh_token_ttl_secs: parse_duration_string(refresh_expiry).unwrap_or(7 * 24 * 3600),
            issuer: "resonance".to_string(),
            audience: "resonance".to_string(),
        }
    }
}

/// Parse duration strings like "15m", "7d", "24h" to seconds
fn parse_duration_string(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: i64 = num_str.parse().ok()?;

    match unit {
        "s" => Some(num),
        "m" => Some(num * 60),
        "h" => Some(num * 3600),
        "d" => Some(num * 24 * 3600),
        "w" => Some(num * 7 * 24 * 3600),
        _ => None,
    }
}

/// Session row from database query
#[derive(Debug, FromRow)]
struct SessionRow {
    id: Uuid,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
}

/// Authentication service providing registration, login, and token management
#[derive(Clone)]
pub struct AuthService {
    pool: PgPool,
    config: AuthConfig,
    argon2: Argon2<'static>,
    /// Pre-computed dummy hash for timing attack prevention.
    /// We verify against this hash when a user is not found to ensure
    /// consistent response times regardless of whether the email exists.
    dummy_password_hash: String,
}

impl AuthService {
    /// Create a new AuthService instance
    pub fn new(pool: PgPool, config: AuthConfig) -> Self {
        let argon2 = Argon2::default();

        // Pre-compute a dummy password hash for timing attack prevention.
        // This hash is used when a user lookup fails, ensuring that the
        // password verification step takes the same amount of time whether
        // or not the user exists, preventing user enumeration attacks.
        let dummy_salt = SaltString::generate(&mut OsRng);
        let dummy_password_hash = argon2
            .hash_password(b"dummy_password_for_timing_attack_prevention", &dummy_salt)
            .expect("dummy password hashing should not fail")
            .to_string();

        Self {
            pool,
            config,
            argon2,
            dummy_password_hash,
        }
    }

    /// Register a new user account
    ///
    /// # Arguments
    /// * `email` - User's email address (must be unique)
    /// * `password` - User's plaintext password (will be hashed with Argon2id)
    /// * `display_name` - User's display name
    ///
    /// # Returns
    /// The newly created User on success
    ///
    /// # Errors
    /// - `ApiError::Conflict` if email already exists
    /// - `ApiError::ValidationError` if email or password is invalid
    pub async fn register(
        &self,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> ApiResult<User> {
        // Validate email format
        if !is_valid_email(email) {
            return Err(ApiError::ValidationError(
                "invalid email format".to_string(),
            ));
        }

        // Validate password strength
        if password.len() < 8 {
            return Err(ApiError::ValidationError(
                "password must be at least 8 characters".to_string(),
            ));
        }

        // Check if email already exists
        let existing: bool =
            sqlx::query_scalar(r#"SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)"#)
                .bind(email.to_lowercase())
                .fetch_one(&self.pool)
                .await?;

        if existing {
            return Err(ApiError::Conflict {
                resource_type: "user",
                id: email.to_string(),
            });
        }

        // Hash password with Argon2id
        let password_hash = self.hash_password(password)?;

        // Create user with default preferences
        let preferences_json = serde_json::to_value(UserPreferences::default())?;

        let user: User = sqlx::query_as(
            r#"
            INSERT INTO users (email, password_hash, display_name, role, preferences)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id,
                email,
                password_hash,
                display_name,
                avatar_url,
                role,
                preferences,
                listenbrainz_token,
                discord_user_id,
                email_verified,
                last_seen_at,
                created_at,
                updated_at
            "#,
        )
        .bind(email.to_lowercase())
        .bind(&password_hash)
        .bind(display_name)
        .bind(UserRole::User)
        .bind(&preferences_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => ApiError::Conflict {
                resource_type: "user",
                id: email.to_string(),
            },
            _ => ApiError::Database(e),
        })?;

        tracing::info!(user_id = %user.id, email = %user.email, "User registered successfully");

        Ok(user)
    }

    /// Authenticate a user and create a new session
    ///
    /// # Arguments
    /// * `email` - User's email address
    /// * `password` - User's plaintext password
    /// * `device_info` - Optional device information for the session
    /// * `ip_address` - Optional client IP address
    /// * `user_agent` - Optional client user agent
    ///
    /// # Returns
    /// Tuple of (User, AuthTokens) on successful authentication
    ///
    /// # Errors
    /// - `ApiError::Unauthorized` if credentials are invalid
    pub async fn login(
        &self,
        email: &str,
        password: &str,
        device_info: Option<DeviceInfo>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> ApiResult<(User, AuthTokens)> {
        // Find user by email
        let user: Option<User> = sqlx::query_as(
            r#"
            SELECT
                id,
                email,
                password_hash,
                display_name,
                avatar_url,
                role,
                preferences,
                listenbrainz_token,
                discord_user_id,
                email_verified,
                last_seen_at,
                created_at,
                updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email.to_lowercase())
        .fetch_optional(&self.pool)
        .await?;

        // SECURITY: Timing attack prevention for user enumeration
        // We must verify a password hash regardless of whether the user exists.
        // This ensures the response time is consistent, preventing attackers
        // from determining if an email address is registered by measuring
        // how long the login request takes.
        let (user, password_valid) = match user {
            Some(u) => {
                // User exists - verify their actual password
                let valid = self.verify_password(password, &u.password_hash)?;
                (Some(u), valid)
            }
            None => {
                // User doesn't exist - still perform password verification
                // against a dummy hash to prevent timing-based user enumeration.
                // The result is ignored but the timing remains consistent.
                let _ = self.verify_password(password, &self.dummy_password_hash);
                (None, false)
            }
        };

        // Check authentication result
        let user = match (user, password_valid) {
            (Some(u), true) => u,
            (Some(_), false) => {
                tracing::warn!(email = %email, "Login failed: invalid password");
                return Err(ApiError::Unauthorized);
            }
            (None, _) => {
                tracing::warn!(email = %email, "Login failed: user not found");
                return Err(ApiError::Unauthorized);
            }
        };

        // Generate tokens and create session
        let tokens = self
            .create_session(&user, device_info, ip_address, user_agent)
            .await?;

        // Update last seen
        sqlx::query("UPDATE users SET last_seen_at = NOW() WHERE id = $1")
            .bind(user.id)
            .execute(&self.pool)
            .await?;

        tracing::info!(user_id = %user.id, email = %user.email, "User logged in successfully");

        Ok((user, tokens))
    }

    /// Refresh authentication tokens using a valid refresh token
    ///
    /// # Arguments
    /// * `refresh_token` - The refresh token from a previous login/refresh
    ///
    /// # Returns
    /// New AuthTokens (both access and refresh tokens are rotated)
    ///
    /// # Errors
    /// - `ApiError::InvalidToken` if refresh token is invalid or expired
    /// - `ApiError::Unauthorized` if session is no longer valid
    pub async fn refresh_token(&self, refresh_token: &str) -> ApiResult<AuthTokens> {
        // Decode and validate refresh token
        let claims = self.verify_refresh_token(refresh_token)?;

        // Find the session and verify refresh token hash matches
        let refresh_token_hash = hash_token(refresh_token);

        let session: Option<SessionRow> = sqlx::query_as(
            r#"
            SELECT id, user_id, expires_at
            FROM sessions
            WHERE id = $1 AND refresh_token_hash = $2 AND is_active = true
            "#,
        )
        .bind(claims.sid)
        .bind(&refresh_token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let session = session
            .ok_or_else(|| ApiError::InvalidToken("session not found or inactive".to_string()))?;

        // Check if session has expired
        if session.expires_at < Utc::now() {
            // Deactivate expired session
            sqlx::query("UPDATE sessions SET is_active = false WHERE id = $1")
                .bind(session.id)
                .execute(&self.pool)
                .await?;
            return Err(ApiError::InvalidToken("session expired".to_string()));
        }

        // Get user for new token generation
        let user: User = sqlx::query_as(
            r#"
            SELECT
                id,
                email,
                password_hash,
                display_name,
                avatar_url,
                role,
                preferences,
                listenbrainz_token,
                discord_user_id,
                email_verified,
                last_seen_at,
                created_at,
                updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(session.user_id)
        .fetch_one(&self.pool)
        .await?;

        // Generate new tokens
        let (access_token, new_refresh_token) = self.generate_token_pair(&user, session.id)?;

        // Calculate expiration timestamps
        let access_expires_at = Utc::now() + Duration::seconds(self.config.access_token_ttl_secs);
        let session_expires_at = Utc::now() + Duration::seconds(self.config.refresh_token_ttl_secs);

        // Update session with new token hashes
        let access_token_hash = hash_token(&access_token);
        let new_refresh_token_hash = hash_token(&new_refresh_token);

        sqlx::query(
            r#"
            UPDATE sessions
            SET token_hash = $1,
                refresh_token_hash = $2,
                last_active_at = NOW(),
                expires_at = $3
            WHERE id = $4
            "#,
        )
        .bind(&access_token_hash)
        .bind(&new_refresh_token_hash)
        .bind(session_expires_at)
        .bind(session.id)
        .execute(&self.pool)
        .await?;

        tracing::debug!(session_id = %session.id, user_id = %user.id, "Token refreshed successfully");

        Ok(AuthTokens::new(
            access_token,
            new_refresh_token,
            access_expires_at,
        ))
    }

    /// Logout a specific session
    ///
    /// # Arguments
    /// * `session_id` - The session ID to invalidate
    ///
    /// # Errors
    /// - `ApiError::NotFound` if session doesn't exist
    pub async fn logout(&self, session_id: Uuid) -> ApiResult<()> {
        let result = sqlx::query("UPDATE sessions SET is_active = false WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound {
                resource_type: "session",
                id: session_id.to_string(),
            });
        }

        tracing::info!(session_id = %session_id, "Session logged out");

        Ok(())
    }

    /// Logout all sessions for a user
    ///
    /// # Arguments
    /// * `user_id` - The user whose sessions to invalidate
    pub async fn logout_all(&self, user_id: Uuid) -> ApiResult<u64> {
        let result = sqlx::query(
            "UPDATE sessions SET is_active = false WHERE user_id = $1 AND is_active = true",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        let count = result.rows_affected();
        tracing::info!(user_id = %user_id, sessions_invalidated = count, "All sessions logged out");

        Ok(count)
    }

    /// Verify an access token and return its claims
    ///
    /// # Arguments
    /// * `token` - The JWT access token to verify
    ///
    /// # Returns
    /// The decoded Claims on success
    ///
    /// # Errors
    /// - `ApiError::InvalidToken` if token is invalid, expired, or malformed
    pub fn verify_access_token(&self, token: &str) -> ApiResult<Claims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| {
            tracing::debug!(error = %e, "Access token verification failed");
            ApiError::InvalidToken(e.to_string())
        })?;

        Ok(token_data.claims)
    }

    /// Verify a refresh token and return its claims
    fn verify_refresh_token(&self, token: &str) -> ApiResult<RefreshClaims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        let token_data = decode::<RefreshClaims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| {
            tracing::debug!(error = %e, "Refresh token verification failed");
            ApiError::InvalidToken(e.to_string())
        })?;

        // Verify it's a refresh token
        if token_data.claims.typ != "refresh" {
            return Err(ApiError::InvalidToken("expected refresh token".to_string()));
        }

        Ok(token_data.claims)
    }

    /// Create a new session for a user
    async fn create_session(
        &self,
        user: &User,
        device_info: Option<DeviceInfo>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> ApiResult<AuthTokens> {
        let session_id = Uuid::new_v4();

        // Generate token pair
        let (access_token, refresh_token) = self.generate_token_pair(user, session_id)?;

        // Calculate expiration timestamps
        let access_expires_at = Utc::now() + Duration::seconds(self.config.access_token_ttl_secs);
        let session_expires_at = Utc::now() + Duration::seconds(self.config.refresh_token_ttl_secs);

        // Hash tokens for storage
        let access_token_hash = hash_token(&access_token);
        let refresh_token_hash = hash_token(&refresh_token);

        // Extract device info
        let (device_name, device_type, device_id) = device_info
            .map(|d| {
                (
                    d.device_name,
                    d.device_type.map(|t| t.to_string()),
                    d.device_id,
                )
            })
            .unwrap_or((None, None, None));

        // Create session record
        sqlx::query(
            r#"
            INSERT INTO sessions (
                id, user_id, token_hash, refresh_token_hash,
                device_name, device_type, device_id,
                ip_address, user_agent, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9, $10)
            "#,
        )
        .bind(session_id)
        .bind(user.id)
        .bind(&access_token_hash)
        .bind(&refresh_token_hash)
        .bind(&device_name)
        .bind(&device_type)
        .bind(&device_id)
        .bind(ip_address)
        .bind(user_agent)
        .bind(session_expires_at)
        .execute(&self.pool)
        .await?;

        Ok(AuthTokens::new(
            access_token,
            refresh_token,
            access_expires_at,
        ))
    }

    /// Generate a pair of access and refresh tokens
    fn generate_token_pair(&self, user: &User, session_id: Uuid) -> ApiResult<(String, String)> {
        // Create access token claims
        let access_claims = Claims::new(user, session_id, self.config.access_token_ttl_secs);

        // Create refresh token claims
        let refresh_claims =
            RefreshClaims::new(user.id, session_id, self.config.refresh_token_ttl_secs);

        // Encode access token
        let access_token = encode(
            &Header::default(),
            &access_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )?;

        // Encode refresh token
        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )?;

        Ok((access_token, refresh_token))
    }

    /// Hash a password with Argon2id
    fn hash_password(&self, password: &str) -> ApiResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| ApiError::Internal(format!("Password hashing failed: {}", e)))?;
        Ok(hash.to_string())
    }

    /// Verify a password against an Argon2id hash
    fn verify_password(&self, password: &str, hash: &str) -> ApiResult<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| ApiError::Internal(format!("Invalid password hash format: {}", e)))?;

        Ok(self
            .argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

/// Hash a token using SHA-256 for secure storage
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Simple email validation
fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.is_empty() || email.len() > 254 {
        return false;
    }

    // Must have exactly one @ symbol
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let (local, domain) = (parts[0], parts[1]);

    // Local part must not be empty and not too long
    if local.is_empty() || local.len() > 64 {
        return false;
    }

    // Domain must have at least one dot and not be empty
    if domain.is_empty() || !domain.contains('.') {
        return false;
    }

    // Domain parts must not be empty
    domain.split('.').all(|part| !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_string() {
        assert_eq!(parse_duration_string("15m"), Some(900));
        assert_eq!(parse_duration_string("7d"), Some(604800));
        assert_eq!(parse_duration_string("24h"), Some(86400));
        assert_eq!(parse_duration_string("30s"), Some(30));
        assert_eq!(parse_duration_string("1w"), Some(604800));
        assert_eq!(parse_duration_string(""), None);
        assert_eq!(parse_duration_string("invalid"), None);
        assert_eq!(parse_duration_string("15x"), None);
    }

    #[test]
    fn test_is_valid_email() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("test.user@domain.co.uk"));
        assert!(is_valid_email("user123@test.org"));
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("missing@domain"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user@@domain.com"));
    }

    #[test]
    fn test_hash_token() {
        let token = "test_token_123";
        let hash = hash_token(token);
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Same token should produce same hash
        assert_eq!(hash, hash_token(token));

        // Different tokens should produce different hashes
        assert_ne!(hash, hash_token("different_token"));
    }

    #[test]
    fn test_auth_config_new() {
        let config = AuthConfig::new("secret".to_string());
        assert_eq!(config.access_token_ttl_secs, 15 * 60);
        assert_eq!(config.refresh_token_ttl_secs, 7 * 24 * 3600);
        assert_eq!(config.issuer, "resonance");
        assert_eq!(config.audience, "resonance");
    }

    #[test]
    fn test_auth_config_with_expiry_strings() {
        let config = AuthConfig::with_expiry_strings("secret".to_string(), "30m", "14d");
        assert_eq!(config.access_token_ttl_secs, 30 * 60);
        assert_eq!(config.refresh_token_ttl_secs, 14 * 24 * 3600);
    }

    #[test]
    fn test_auth_config_invalid_expiry_uses_default() {
        let config =
            AuthConfig::with_expiry_strings("secret".to_string(), "invalid", "also_invalid");
        assert_eq!(config.access_token_ttl_secs, 15 * 60);
        assert_eq!(config.refresh_token_ttl_secs, 7 * 24 * 3600);
    }

    #[test]
    fn test_dummy_password_hash_for_timing_attack_prevention() {
        // Verify that the dummy password hash mechanism works correctly:
        // 1. A dummy hash can be created
        // 2. Verifying against it (with any password) takes similar time as real verification
        // 3. The verification always fails (as expected)
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let dummy_hash = argon2
            .hash_password(b"dummy_password_for_timing_attack_prevention", &salt)
            .expect("dummy password hashing should not fail")
            .to_string();

        // The hash should be a valid Argon2 hash format
        let parsed = PasswordHash::new(&dummy_hash);
        assert!(parsed.is_ok(), "Dummy hash should be parseable as Argon2");

        // Verifying with an incorrect password should fail (but not panic)
        // This is the key behavior for timing attack prevention:
        // when a user doesn't exist, we still verify against this dummy hash
        let verify_result = argon2.verify_password(b"attacker_password", &parsed.unwrap());
        assert!(
            verify_result.is_err(),
            "Verification with wrong password should fail"
        );
    }
}
