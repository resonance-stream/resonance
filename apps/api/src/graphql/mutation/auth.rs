//! Authentication mutations for Resonance GraphQL API
//!
//! This module provides mutations for user authentication:
//! - register: Create a new user account (rate limited: 3/hour)
//! - login: Authenticate and get tokens (rate limited: 5/minute)
//! - refreshToken: Get new tokens using refresh token (rate limited: 10/minute)
//! - logout: Invalidate the current session
//!
//! Account settings mutations:
//! - changePassword: Change the user's password (invalidates other sessions)
//! - updateEmail: Change the user's email (requires password verification)
//! - updateProfile: Update display name and/or avatar URL
//! - deleteAccount: Permanently delete the user's account (requires password verification)

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::error::ApiError;
use crate::graphql::guards::{RateLimitGuard, RateLimitType};
use crate::graphql::types::{AuthPayload, RefreshPayload, User};
use crate::models::user::{DeviceInfo, DeviceType, RequestMetadata};
use crate::repositories::UserRepository;
use crate::services::auth::AuthService;

// =============================================================================
// Error Sanitization
// =============================================================================

/// Sanitize auth errors to prevent information disclosure
///
/// Maps internal error variants to generic user-facing messages while
/// logging the full error details server-side. This prevents leaking
/// sensitive information like "password hash invalid" or database errors.
fn sanitize_auth_error(error: &ApiError) -> async_graphql::Error {
    match error {
        // Expected user-facing errors with safe messages
        ApiError::Unauthorized => {
            tracing::debug!("Auth unauthorized error");
            async_graphql::Error::new("Invalid credentials")
        }
        ApiError::InvalidToken(msg) => {
            tracing::debug!(error = %msg, "Invalid token error");
            async_graphql::Error::new("Invalid or expired token")
        }
        ApiError::Forbidden(msg) => {
            tracing::debug!(error = %msg, "Auth forbidden error");
            async_graphql::Error::new("Access denied")
        }
        ApiError::NotFound { .. } => {
            // Don't reveal whether a user exists
            async_graphql::Error::new("Invalid credentials")
        }
        ApiError::Conflict { resource_type, .. } => {
            // Email already exists - this is safe to reveal
            if resource_type.to_lowercase().contains("email")
                || resource_type.to_lowercase().contains("user")
            {
                async_graphql::Error::new("Email already registered")
            } else {
                async_graphql::Error::new("Resource conflict")
            }
        }
        ApiError::ValidationError(msg) => {
            // Validation errors are generally safe to reveal
            async_graphql::Error::new(msg.clone())
        }
        ApiError::RateLimited { retry_after } => async_graphql::Error::new(format!(
            "Too many requests. Try again in {} seconds",
            retry_after
        )),
        // All other errors (internal, database, etc.) should not be exposed
        _ => {
            tracing::error!(error = %error, "Internal auth error");
            async_graphql::Error::new("An unexpected error occurred")
        }
    }
}

/// Input for user registration
#[derive(Debug, InputObject)]
pub struct RegisterInput {
    /// User's email address (must be unique)
    pub email: String,
    /// Password (minimum 8 characters)
    pub password: String,
    /// Display name shown in the UI
    pub display_name: String,
}

/// Input for user login
#[derive(Debug, InputObject)]
pub struct LoginInput {
    /// User's email address
    pub email: String,
    /// User's password
    pub password: String,
    /// Optional device information for the session
    pub device: Option<DeviceInput>,
}

/// Device information input for session tracking
#[derive(Debug, InputObject)]
pub struct DeviceInput {
    /// Human-readable device name (e.g., "iPhone 15 Pro")
    pub device_name: Option<String>,
    /// Device type category
    pub device_type: Option<DeviceTypeInput>,
    /// Unique device identifier (for device limits)
    pub device_id: Option<String>,
}

/// Device type categories for GraphQL input
#[derive(Debug, Clone, Copy, PartialEq, Eq, async_graphql::Enum)]
pub enum DeviceTypeInput {
    Desktop,
    Mobile,
    Tablet,
    Web,
    Tv,
}

impl From<DeviceTypeInput> for DeviceType {
    fn from(input: DeviceTypeInput) -> Self {
        match input {
            DeviceTypeInput::Desktop => DeviceType::Desktop,
            DeviceTypeInput::Mobile => DeviceType::Mobile,
            DeviceTypeInput::Tablet => DeviceType::Tablet,
            DeviceTypeInput::Web => DeviceType::Web,
            DeviceTypeInput::Tv => DeviceType::Tv,
        }
    }
}

impl From<DeviceInput> for DeviceInfo {
    fn from(input: DeviceInput) -> Self {
        DeviceInfo {
            device_name: input.device_name,
            device_type: input.device_type.map(|t| t.into()),
            device_id: input.device_id,
        }
    }
}

// =============================================================================
// Request Context Helpers
// =============================================================================

/// Extracted request context for auth operations
///
/// Contains IP address and user agent extracted from GraphQL context,
/// used for session audit trails and security logging.
struct RequestContext {
    ip_address: Option<String>,
    user_agent: Option<String>,
}

impl RequestContext {
    /// Extract request context from GraphQL context
    ///
    /// Reads RequestMetadata from context data (set by middleware) and
    /// extracts relevant fields for auth operations.
    fn from_graphql_context(ctx: &Context<'_>) -> Self {
        let metadata = ctx.data_opt::<RequestMetadata>();
        Self {
            ip_address: metadata.and_then(|m| m.ip_address.clone()),
            user_agent: metadata.and_then(|m| m.user_agent.clone()),
        }
    }

    /// Get IP address as a str reference
    fn ip_address(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }

    /// Get user agent as a str reference
    fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }
}

// =============================================================================
// Input Types
// =============================================================================

/// Input for token refresh
#[derive(Debug, InputObject)]
pub struct RefreshTokenInput {
    /// The refresh token from a previous login or refresh
    pub refresh_token: String,
}

// =============================================================================
// Account Settings Input Types
// =============================================================================

/// Input for changing password
#[derive(Debug, InputObject)]
pub struct ChangePasswordInput {
    /// Current password for verification
    pub current_password: String,
    /// New password (minimum 8 characters, must include uppercase, lowercase, number)
    pub new_password: String,
}

/// Input for updating email
#[derive(Debug, InputObject)]
pub struct UpdateEmailInput {
    /// New email address
    pub new_email: String,
    /// Current password for verification (required for security)
    pub current_password: String,
}

/// Input for updating profile
///
/// At least one field must be provided. To perform an update, include
/// only the fields you want to change.
#[derive(Debug, InputObject)]
pub struct UpdateProfileInput {
    /// New display name (1-100 characters, optional)
    pub display_name: Option<String>,
    /// New avatar URL (must be http/https, optional)
    ///
    /// Behavior:
    /// - `null` or omitted: No change to avatar
    /// - `""` (empty string): Clear the avatar (set to null)
    /// - `"https://..."`: Set to the provided URL
    pub avatar_url: Option<String>,
}

/// Input for deleting account
#[derive(Debug, InputObject)]
pub struct DeleteAccountInput {
    /// Current password for verification (required for security)
    pub password: String,
}

/// Result of changing password
#[derive(Debug, SimpleObject)]
pub struct ChangePasswordResult {
    /// Whether the password was changed successfully
    pub success: bool,
    /// Number of other sessions that were invalidated
    pub sessions_invalidated: i32,
}

/// Authentication mutations
#[derive(Default)]
pub struct AuthMutation;

#[Object]
impl AuthMutation {
    /// Register a new user account
    ///
    /// Creates a new user with the provided email, password, and display name.
    /// Returns the user data along with authentication tokens.
    ///
    /// Rate limited to 3 attempts per hour per IP address.
    ///
    /// # Errors
    /// - Returns error if email is already registered
    /// - Returns error if email format is invalid
    /// - Returns error if password is less than 8 characters
    /// - Returns error if rate limit is exceeded
    #[graphql(guard = "RateLimitGuard::new(RateLimitType::Register)")]
    async fn register(&self, ctx: &Context<'_>, input: RegisterInput) -> Result<AuthPayload> {
        let auth_service = ctx.data::<AuthService>()?;
        let req_ctx = RequestContext::from_graphql_context(ctx);

        // Register and create session in one call to avoid redundant password hashing
        let (user, tokens) = auth_service
            .register_with_session(
                &input.email,
                &input.password,
                &input.display_name,
                None, // device_info - could be added to RegisterInput in future
                req_ctx.ip_address(),
                req_ctx.user_agent(),
            )
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        Ok(AuthPayload::new(user, tokens))
    }

    /// Authenticate a user and get access tokens
    ///
    /// Validates the user's credentials and creates a new session.
    /// Returns the user data along with authentication tokens.
    ///
    /// Rate limited to 5 attempts per minute per IP address.
    ///
    /// # Errors
    /// - Returns error if credentials are invalid
    /// - Returns error if rate limit is exceeded
    #[graphql(guard = "RateLimitGuard::new(RateLimitType::Login)")]
    async fn login(&self, ctx: &Context<'_>, input: LoginInput) -> Result<AuthPayload> {
        let auth_service = ctx.data::<AuthService>()?;
        let req_ctx = RequestContext::from_graphql_context(ctx);
        let device_info = input.device.map(DeviceInfo::from);

        let (user, tokens) = auth_service
            .login(
                &input.email,
                &input.password,
                device_info,
                req_ctx.ip_address(),
                req_ctx.user_agent(),
            )
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        Ok(AuthPayload::new(user, tokens))
    }

    /// Refresh authentication tokens
    ///
    /// Uses a valid refresh token to obtain a new access token and refresh token.
    /// The old refresh token is invalidated (token rotation).
    ///
    /// Rate limited to 10 attempts per minute per IP address.
    ///
    /// # Errors
    /// - Returns error if refresh token is invalid or expired
    /// - Returns error if session is no longer active
    /// - Returns error if rate limit is exceeded
    #[graphql(guard = "RateLimitGuard::new(RateLimitType::RefreshToken)")]
    async fn refresh_token(
        &self,
        ctx: &Context<'_>,
        input: RefreshTokenInput,
    ) -> Result<RefreshPayload> {
        let auth_service = ctx.data::<AuthService>()?;

        let tokens = auth_service
            .refresh_token(&input.refresh_token)
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        Ok(RefreshPayload::from(tokens))
    }

    /// Logout the current session
    ///
    /// Invalidates the current session. The user must be authenticated.
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if session not found
    async fn logout(&self, ctx: &Context<'_>) -> Result<bool> {
        let auth_service = ctx.data::<AuthService>()?;

        // Get the current session from context (set by auth middleware)
        let claims = ctx
            .data_opt::<crate::models::user::Claims>()
            .ok_or_else(|| async_graphql::Error::new("authentication required"))?;

        auth_service
            .logout(claims.sid)
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        Ok(true)
    }

    /// Logout all sessions for the current user
    ///
    /// Invalidates all active sessions for the authenticated user.
    ///
    /// # Returns
    /// The number of sessions that were invalidated
    ///
    /// # Errors
    /// - Returns error if not authenticated
    async fn logout_all(&self, ctx: &Context<'_>) -> Result<i32> {
        let auth_service = ctx.data::<AuthService>()?;

        // Get the current session from context
        let claims = ctx
            .data_opt::<crate::models::user::Claims>()
            .ok_or_else(|| async_graphql::Error::new("authentication required"))?;

        let count = auth_service
            .logout_all(claims.sub)
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        Ok(count as i32)
    }

    // =========================================================================
    // Account Settings Mutations
    // =========================================================================

    /// Change the current user's password
    ///
    /// Requires the current password for verification. After successful change,
    /// all other sessions will be invalidated for security (user stays logged in
    /// on current device but must re-login on other devices).
    ///
    /// Rate limited to 5 attempts per 15 minutes per IP address.
    ///
    /// # Arguments
    /// * `input` - Contains current_password and new_password
    ///
    /// # Returns
    /// Result containing success status and count of invalidated sessions
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if current password is incorrect
    /// - Returns error if new password doesn't meet complexity requirements
    /// - Returns error if rate limit is exceeded
    #[graphql(guard = "RateLimitGuard::new(RateLimitType::ChangePassword)")]
    async fn change_password(
        &self,
        ctx: &Context<'_>,
        input: ChangePasswordInput,
    ) -> Result<ChangePasswordResult> {
        let auth_service = ctx.data::<AuthService>()?;

        // Get the current session from context
        let claims = ctx
            .data_opt::<crate::models::user::Claims>()
            .ok_or_else(|| async_graphql::Error::new("authentication required"))?;

        let sessions_invalidated = auth_service
            .change_password(
                claims.sub,
                claims.sid,
                &input.current_password,
                &input.new_password,
            )
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        Ok(ChangePasswordResult {
            success: true,
            sessions_invalidated: sessions_invalidated as i32,
        })
    }

    /// Update the current user's email address
    ///
    /// Requires password verification for security. After successful update,
    /// the email_verified flag will be reset to false.
    ///
    /// Rate limited to 5 attempts per 15 minutes per IP address (same as password change).
    ///
    /// # Arguments
    /// * `input` - Contains new_email and current_password
    ///
    /// # Returns
    /// The updated user
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if password is incorrect
    /// - Returns error if email format is invalid
    /// - Returns error if email is already in use
    /// - Returns error if rate limit is exceeded
    #[graphql(guard = "RateLimitGuard::new(RateLimitType::ChangePassword)")]
    async fn update_email(&self, ctx: &Context<'_>, input: UpdateEmailInput) -> Result<User> {
        let auth_service = ctx.data::<AuthService>()?;
        let user_repo = ctx.data::<UserRepository>()?;

        // Get the current session from context
        let claims = ctx
            .data_opt::<crate::models::user::Claims>()
            .ok_or_else(|| async_graphql::Error::new("authentication required"))?;

        // Update the email
        auth_service
            .update_email(claims.sub, &input.new_email, &input.current_password)
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        // Fetch and return the updated user
        let user = user_repo.find_by_id(claims.sub).await.map_err(|e| {
            tracing::error!(error = %e, user_id = %claims.sub, "Failed to fetch updated user");
            async_graphql::Error::new("Failed to fetch updated user")
        })?;

        let user = user.ok_or_else(|| {
            tracing::error!(user_id = %claims.sub, "User not found after email update");
            async_graphql::Error::new("User not found")
        })?;

        Ok(User::from(user))
    }

    /// Update the current user's profile (display name and/or avatar)
    ///
    /// This is a non-sensitive operation that doesn't require password verification.
    /// At least one field must be provided.
    ///
    /// # Arguments
    /// * `input` - Contains optional display_name and avatar_url
    ///
    /// # Returns
    /// The updated user
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if no fields are provided (no-op not allowed)
    /// - Returns error if display name is invalid (empty or > 100 chars)
    /// - Returns error if avatar URL is invalid
    async fn update_profile(&self, ctx: &Context<'_>, input: UpdateProfileInput) -> Result<User> {
        // Validate that at least one field is provided
        if input.display_name.is_none() && input.avatar_url.is_none() {
            return Err(async_graphql::Error::new(
                "At least one field (display_name or avatar_url) must be provided",
            ));
        }

        let auth_service = ctx.data::<AuthService>()?;
        let user_repo = ctx.data::<UserRepository>()?;

        // Get the current session from context
        let claims = ctx
            .data_opt::<crate::models::user::Claims>()
            .ok_or_else(|| async_graphql::Error::new("authentication required"))?;

        // Convert UpdateProfileInput to the service method signature
        // For avatar_url: None = don't update, Some(None) = clear, Some(Some(url)) = set
        // The input has: None = don't update, Some(empty or value)
        let avatar_url: Option<Option<&str>> = match &input.avatar_url {
            None => None,                              // Don't update
            Some(url) if url.is_empty() => Some(None), // Clear (empty string = clear)
            Some(url) => Some(Some(url.as_str())),     // Set to value
        };

        // Update the profile
        auth_service
            .update_profile(claims.sub, input.display_name.as_deref(), avatar_url)
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        // Fetch and return the updated user
        let user = user_repo.find_by_id(claims.sub).await.map_err(|e| {
            tracing::error!(error = %e, user_id = %claims.sub, "Failed to fetch updated user");
            async_graphql::Error::new("Failed to fetch updated user")
        })?;

        let user = user.ok_or_else(|| {
            tracing::error!(user_id = %claims.sub, "User not found after profile update");
            async_graphql::Error::new("User not found")
        })?;

        Ok(User::from(user))
    }

    /// Delete the current user's account
    ///
    /// This is a destructive operation that permanently deletes the user's account
    /// and all associated data. Requires password verification for security.
    /// All sessions will be invalidated before deletion.
    ///
    /// Rate limited to 5 attempts per 15 minutes per IP address (same as password change).
    ///
    /// # Arguments
    /// * `input` - Contains the password for verification
    ///
    /// # Returns
    /// True if the account was deleted successfully
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if password is incorrect
    /// - Returns error if rate limit is exceeded
    #[graphql(guard = "RateLimitGuard::new(RateLimitType::ChangePassword)")]
    async fn delete_account(&self, ctx: &Context<'_>, input: DeleteAccountInput) -> Result<bool> {
        let auth_service = ctx.data::<AuthService>()?;

        // Get the current session from context
        let claims = ctx
            .data_opt::<crate::models::user::Claims>()
            .ok_or_else(|| async_graphql::Error::new("authentication required"))?;

        auth_service
            .delete_account(claims.sub, &input.password)
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        Ok(true)
    }
}
