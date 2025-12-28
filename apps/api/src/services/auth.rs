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
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::user::{
    AuthTokens, Claims, DeviceInfo, RefreshClaims, User, UserPreferences, UserRole,
};
use crate::repositories::{SessionRepository, UserRepository};

// =============================================================================
// Security Constants
// =============================================================================

/// Minimum length for JWT secret (256 bits for HS256)
pub const MIN_JWT_SECRET_LENGTH: usize = 32;

/// Minimum password length
pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Maximum password length (prevent DoS via extremely long passwords)
pub const MAX_PASSWORD_LENGTH: usize = 128;

/// Maximum display name length
#[allow(dead_code)] // Used for display name validation
pub const MAX_DISPLAY_NAME_LENGTH: usize = 100;

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
    ///
    /// # Panics
    /// Panics if jwt_secret is shorter than MIN_JWT_SECRET_LENGTH (32 bytes)
    #[allow(dead_code)] // Available for simpler initialization without expiry strings
    pub fn new(jwt_secret: String) -> Self {
        Self::validate_jwt_secret(&jwt_secret);
        Self {
            jwt_secret,
            access_token_ttl_secs: 15 * 60,        // 15 minutes
            refresh_token_ttl_secs: 7 * 24 * 3600, // 7 days
            issuer: "resonance".to_string(),
            audience: "resonance".to_string(),
        }
    }

    /// Create AuthConfig from expiry strings (e.g., "15m", "7d")
    ///
    /// # Panics
    /// Panics if jwt_secret is shorter than MIN_JWT_SECRET_LENGTH (32 bytes)
    pub fn with_expiry_strings(
        jwt_secret: String,
        access_expiry: &str,
        refresh_expiry: &str,
    ) -> Self {
        Self::validate_jwt_secret(&jwt_secret);
        Self {
            jwt_secret,
            access_token_ttl_secs: parse_duration_string(access_expiry).unwrap_or(15 * 60),
            refresh_token_ttl_secs: parse_duration_string(refresh_expiry).unwrap_or(7 * 24 * 3600),
            issuer: "resonance".to_string(),
            audience: "resonance".to_string(),
        }
    }

    /// Validate JWT secret meets minimum security requirements
    fn validate_jwt_secret(secret: &str) {
        if secret.len() < MIN_JWT_SECRET_LENGTH {
            panic!(
                "JWT_SECRET must be at least {} characters for security (got {}). \
                 Generate a secure secret with: openssl rand -base64 48",
                MIN_JWT_SECRET_LENGTH,
                secret.len()
            );
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

/// Authentication service providing registration, login, and token management
#[derive(Clone)]
pub struct AuthService {
    config: AuthConfig,
    argon2: Argon2<'static>,
    /// Pre-computed dummy hash for timing attack prevention.
    /// We verify against this hash when a user is not found to ensure
    /// consistent response times regardless of whether the email exists.
    dummy_password_hash: String,
    /// User repository for centralized database operations
    user_repo: UserRepository,
    /// Session repository for centralized session database operations
    session_repo: SessionRepository,
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

        // Create repositories for centralized database operations
        let user_repo = UserRepository::new(pool.clone());
        let session_repo = SessionRepository::new(pool);

        Self {
            config,
            argon2,
            dummy_password_hash,
            user_repo,
            session_repo,
        }
    }

    /// Register a new user account
    ///
    /// # Arguments
    /// * `email` - User's email address (must be unique)
    /// * `password` - User's plaintext password (will be hashed with Argon2id)
    /// * `display_name` - User's display name (1-100 characters)
    ///
    /// # Returns
    /// The newly created User on success
    ///
    /// # Errors
    /// - `ApiError::Conflict` if email already exists
    /// - `ApiError::ValidationError` if email, password, or display_name is invalid
    pub async fn register(
        &self,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> ApiResult<User> {
        // Normalize email for consistent storage and lookup
        let email = normalize_email(email);

        // Validate email format
        if !is_valid_email(&email) {
            return Err(ApiError::ValidationError(
                "invalid email format".to_string(),
            ));
        }

        // Validate password complexity
        let password_validation = validate_password_complexity(password);
        if !password_validation.is_valid {
            return Err(ApiError::ValidationError(
                password_validation.errors.join("; "),
            ));
        }

        // Validate display_name length (1-100 characters)
        if !is_valid_display_name(display_name) {
            return Err(ApiError::ValidationError(
                "display_name must be between 1 and 100 characters".to_string(),
            ));
        }

        // Check if email already exists using repository
        let existing = self.user_repo.email_exists(&email).await?;

        if existing {
            return Err(ApiError::Conflict {
                resource_type: "user",
                id: email.clone(),
            });
        }

        // Hash password with Argon2id
        let password_hash = self.hash_password(password)?;

        // Create user with default preferences using repository
        let preferences_json = serde_json::to_value(UserPreferences::default())?;

        let user = self
            .user_repo
            .create(
                &email,
                &password_hash,
                display_name,
                UserRole::User,
                &preferences_json,
            )
            .await
            .map_err(|e| match &e {
                sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                    ApiError::Conflict {
                        resource_type: "user",
                        id: email.clone(),
                    }
                }
                _ => ApiError::Database(e),
            })?;

        tracing::info!(user_id = %user.id, email = %user.email, "User registered successfully");

        Ok(user)
    }

    /// Register a new user and immediately create a session with tokens
    ///
    /// This optimized method combines registration and session creation to avoid
    /// the need for a separate login call after registration. Since we just hashed
    /// the password during registration, we can directly create tokens without
    /// re-hashing (which would happen if login() were called separately).
    ///
    /// # Arguments
    /// * `email` - User's email address (must be unique)
    /// * `password` - User's plaintext password (will be hashed with Argon2id)
    /// * `display_name` - User's display name
    /// * `device_info` - Optional device information for the session
    /// * `ip_address` - Optional client IP address
    /// * `user_agent` - Optional client user agent
    ///
    /// # Returns
    /// Tuple of (User, AuthTokens) on success - user is logged in immediately
    ///
    /// # Errors
    /// - `ApiError::Conflict` if email already exists
    /// - `ApiError::ValidationError` if email or password is invalid
    pub async fn register_with_session(
        &self,
        email: &str,
        password: &str,
        display_name: &str,
        device_info: Option<DeviceInfo>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> ApiResult<(User, AuthTokens)> {
        // Register the user (password is hashed here)
        let user = self.register(email, password, display_name).await?;

        // Create session directly - no need to verify password again since we just
        // created the user with that password. This saves an expensive Argon2id
        // verification operation.
        let tokens = self
            .create_session(&user, device_info, ip_address, user_agent)
            .await?;

        // Update last seen using repository
        self.user_repo.update_last_seen(user.id).await?;

        tracing::info!(
            user_id = %user.id,
            email = %user.email,
            "User registered and session created successfully"
        );

        Ok((user, tokens))
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
        // Normalize email for consistent lookup
        let email = normalize_email(email);

        // Find user by email using repository
        let user = self.user_repo.find_by_email(&email).await?;

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

        // Update last seen using repository
        self.user_repo.update_last_seen(user.id).await?;

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

        // Find the session and verify refresh token hash matches using repository
        let refresh_token_hash = hash_token(refresh_token);

        let session = self
            .session_repo
            .find_active_by_refresh_token(claims.sid, &refresh_token_hash)
            .await?
            .ok_or_else(|| ApiError::InvalidToken("session not found or inactive".to_string()))?;

        // Check if session has expired
        if session.expires_at < Utc::now() {
            // Deactivate expired session using repository
            let _ = self.session_repo.deactivate(session.id).await?;
            return Err(ApiError::InvalidToken("session expired".to_string()));
        }

        // Get user for new token generation using repository
        let user = self
            .user_repo
            .find_by_id(session.user_id)
            .await?
            .ok_or_else(|| ApiError::InvalidToken("user not found".to_string()))?;

        // Generate new tokens
        let (access_token, new_refresh_token) = self.generate_token_pair(&user, session.id)?;

        // Calculate expiration timestamps
        let access_expires_at = Utc::now() + Duration::seconds(self.config.access_token_ttl_secs);
        let session_expires_at = Utc::now() + Duration::seconds(self.config.refresh_token_ttl_secs);

        // Update session with new token hashes using repository
        let access_token_hash = hash_token(&access_token);
        let new_refresh_token_hash = hash_token(&new_refresh_token);

        self.session_repo
            .update_tokens(
                session.id,
                &access_token_hash,
                &new_refresh_token_hash,
                session_expires_at,
            )
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
        let deactivated = self.session_repo.deactivate(session_id).await?;

        if !deactivated {
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
        let count = self.session_repo.deactivate_all_for_user(user_id).await?;

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

        // Create session record using repository
        self.session_repo
            .create(
                session_id,
                user.id,
                &access_token_hash,
                &refresh_token_hash,
                device_name.as_deref(),
                device_type.as_deref(),
                device_id.as_deref(),
                ip_address,
                user_agent,
                session_expires_at,
            )
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

/// Password complexity validation result
#[derive(Debug, Clone, PartialEq)]
pub struct PasswordValidation {
    pub is_valid: bool,
    pub has_min_length: bool,
    pub has_max_length: bool,
    pub has_uppercase: bool,
    pub has_lowercase: bool,
    pub has_number: bool,
    pub errors: Vec<String>,
}

impl PasswordValidation {
    fn new() -> Self {
        Self {
            is_valid: false,
            has_min_length: false,
            has_max_length: true, // Assume valid until proven otherwise
            has_uppercase: false,
            has_lowercase: false,
            has_number: false,
            errors: Vec::new(),
        }
    }
}

/// Validate password complexity
///
/// Password must meet the following requirements:
/// - At least 8 characters long
/// - Contains at least one uppercase letter (A-Z)
/// - Contains at least one lowercase letter (a-z)
/// - Contains at least one number (0-9)
///
/// # Arguments
/// * `password` - The password to validate
///
/// # Returns
/// A `PasswordValidation` struct containing validation results and any errors
pub fn validate_password_complexity(password: &str) -> PasswordValidation {
    let mut result = PasswordValidation::new();

    // Check minimum length
    result.has_min_length = password.len() >= MIN_PASSWORD_LENGTH;
    if !result.has_min_length {
        result.errors.push(format!(
            "Password must be at least {} characters",
            MIN_PASSWORD_LENGTH
        ));
    }

    // Check maximum length (prevent DoS via extremely long passwords)
    result.has_max_length = password.len() <= MAX_PASSWORD_LENGTH;
    if !result.has_max_length {
        result.errors.push(format!(
            "Password must be at most {} characters",
            MAX_PASSWORD_LENGTH
        ));
    }

    // Check for at least one uppercase letter
    result.has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    if !result.has_uppercase {
        result
            .errors
            .push("Password must contain at least one uppercase letter".to_string());
    }

    // Check for at least one lowercase letter
    result.has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    if !result.has_lowercase {
        result
            .errors
            .push("Password must contain at least one lowercase letter".to_string());
    }

    // Check for at least one number
    result.has_number = password.chars().any(|c| c.is_ascii_digit());
    if !result.has_number {
        result
            .errors
            .push("Password must contain at least one number".to_string());
    }

    // Password is valid if all requirements are met
    result.is_valid = result.has_min_length
        && result.has_max_length
        && result.has_uppercase
        && result.has_lowercase
        && result.has_number;

    result
}

/// Validate display_name length (1-100 characters)
///
/// Display name must:
/// - Not be empty (after trimming whitespace)
/// - Not exceed 100 characters
///
/// # Arguments
/// * `display_name` - The display name to validate
///
/// # Returns
/// `true` if the display name is valid, `false` otherwise
fn is_valid_display_name(display_name: &str) -> bool {
    let trimmed = display_name.trim();
    !trimmed.is_empty() && trimmed.len() <= 100
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

/// Normalize email address for consistent storage and lookup
///
/// Normalization includes:
/// - Trimming leading/trailing whitespace
/// - Converting to lowercase (email addresses are case-insensitive per RFC 5321)
///
/// # Arguments
/// * `email` - The email address to normalize
///
/// # Returns
/// The normalized email address
fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
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
    fn test_is_valid_display_name() {
        // Valid display names
        assert!(is_valid_display_name("John Doe"));
        assert!(is_valid_display_name("A")); // minimum length (1 char)
        assert!(is_valid_display_name("DJ Music Lover 123"));
        assert!(is_valid_display_name("用户名")); // Unicode characters

        // Exactly 100 characters (should be valid)
        let exactly_100 = "a".repeat(100);
        assert!(is_valid_display_name(&exactly_100));

        // Invalid: empty string
        assert!(!is_valid_display_name(""));

        // Invalid: only whitespace
        assert!(!is_valid_display_name("   "));
        assert!(!is_valid_display_name("\t\n"));

        // Invalid: exceeds 100 characters
        let too_long = "a".repeat(101);
        assert!(!is_valid_display_name(&too_long));

        // Edge case: whitespace padding should be trimmed
        // " A " should be valid (trimmed to "A")
        assert!(is_valid_display_name(" A "));

        // Edge case: name with only leading/trailing whitespace but empty content
        assert!(!is_valid_display_name("     "));
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

    /// Test secret that meets the 32 character minimum requirement
    const TEST_JWT_SECRET: &str = "test-jwt-secret-that-is-at-least-32-characters-long";

    #[test]
    fn test_auth_config_new() {
        let config = AuthConfig::new(TEST_JWT_SECRET.to_string());
        assert_eq!(config.access_token_ttl_secs, 15 * 60);
        assert_eq!(config.refresh_token_ttl_secs, 7 * 24 * 3600);
        assert_eq!(config.issuer, "resonance");
        assert_eq!(config.audience, "resonance");
    }

    #[test]
    fn test_auth_config_with_expiry_strings() {
        let config = AuthConfig::with_expiry_strings(TEST_JWT_SECRET.to_string(), "30m", "14d");
        assert_eq!(config.access_token_ttl_secs, 30 * 60);
        assert_eq!(config.refresh_token_ttl_secs, 14 * 24 * 3600);
    }

    #[test]
    fn test_auth_config_invalid_expiry_uses_default() {
        let config =
            AuthConfig::with_expiry_strings(TEST_JWT_SECRET.to_string(), "invalid", "also_invalid");
        assert_eq!(config.access_token_ttl_secs, 15 * 60);
        assert_eq!(config.refresh_token_ttl_secs, 7 * 24 * 3600);
    }

    #[test]
    fn test_validate_password_complexity_valid_password() {
        let result = validate_password_complexity("ValidPass1");
        assert!(result.is_valid);
        assert!(result.has_min_length);
        assert!(result.has_uppercase);
        assert!(result.has_lowercase);
        assert!(result.has_number);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_password_complexity_too_short() {
        let result = validate_password_complexity("Pass1");
        assert!(!result.is_valid);
        assert!(!result.has_min_length);
        assert!(result.has_uppercase);
        assert!(result.has_lowercase);
        assert!(result.has_number);
        assert!(result
            .errors
            .contains(&"Password must be at least 8 characters".to_string()));
    }

    #[test]
    fn test_validate_password_complexity_missing_uppercase() {
        let result = validate_password_complexity("password1");
        assert!(!result.is_valid);
        assert!(result.has_min_length);
        assert!(!result.has_uppercase);
        assert!(result.has_lowercase);
        assert!(result.has_number);
        assert!(result
            .errors
            .contains(&"Password must contain at least one uppercase letter".to_string()));
    }

    #[test]
    fn test_validate_password_complexity_missing_lowercase() {
        let result = validate_password_complexity("PASSWORD1");
        assert!(!result.is_valid);
        assert!(result.has_min_length);
        assert!(result.has_uppercase);
        assert!(!result.has_lowercase);
        assert!(result.has_number);
        assert!(result
            .errors
            .contains(&"Password must contain at least one lowercase letter".to_string()));
    }

    #[test]
    fn test_validate_password_complexity_missing_number() {
        let result = validate_password_complexity("Password");
        assert!(!result.is_valid);
        assert!(result.has_min_length);
        assert!(result.has_uppercase);
        assert!(result.has_lowercase);
        assert!(!result.has_number);
        assert!(result
            .errors
            .contains(&"Password must contain at least one number".to_string()));
    }

    #[test]
    fn test_validate_password_complexity_all_numbers() {
        let result = validate_password_complexity("12345678");
        assert!(!result.is_valid);
        assert!(result.has_min_length);
        assert!(!result.has_uppercase);
        assert!(!result.has_lowercase);
        assert!(result.has_number);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_validate_password_complexity_empty() {
        let result = validate_password_complexity("");
        assert!(!result.is_valid);
        assert!(!result.has_min_length);
        assert!(!result.has_uppercase);
        assert!(!result.has_lowercase);
        assert!(!result.has_number);
        assert_eq!(result.errors.len(), 4);
    }

    #[test]
    fn test_validate_password_complexity_edge_cases() {
        // Exactly 8 characters with all requirements
        let result = validate_password_complexity("Abcdef1!");
        assert!(result.is_valid);

        // Just at the boundary
        let result = validate_password_complexity("Abcde12");
        assert!(!result.is_valid);
        assert!(!result.has_min_length);
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
