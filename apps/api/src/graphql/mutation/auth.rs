//! Authentication mutations for Resonance GraphQL API
//!
//! This module provides mutations for user authentication:
//! - register: Create a new user account (rate limited: 3/hour)
//! - login: Authenticate and get tokens (rate limited: 5/minute)
//! - refreshToken: Get new tokens using refresh token (rate limited: 10/minute)
//! - logout: Invalidate the current session

use async_graphql::{Context, InputObject, Object, Result};

use crate::graphql::guards::{RateLimitGuard, RateLimitType};
use crate::graphql::types::{AuthPayload, RefreshPayload};
use crate::models::user::{DeviceInfo, DeviceType, RequestMetadata};
use crate::services::auth::AuthService;

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
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

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
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

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
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

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
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

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
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count as i32)
    }
}
