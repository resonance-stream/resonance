//! Authentication REST route handlers for Resonance
//!
//! Provides endpoints for user authentication:
//! - `POST /auth/register` - Create a new user account (rate limited: 3/hour per IP)
//! - `POST /auth/login` - Authenticate and get tokens (rate limited: 5/minute per IP)
//! - `POST /auth/refresh` - Refresh access token
//! - `DELETE /auth/logout` - Invalidate current session

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{delete, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::middleware::{login_rate_limit, register_rate_limit, AuthRateLimitState, AuthUser};
use crate::models::user::{AuthTokens, DeviceInfo, User};
use crate::services::AuthService;

/// Shared application state for auth handlers
#[derive(Clone)]
pub struct AuthState {
    /// Authentication service
    pub auth_service: Arc<AuthService>,
}

impl AuthState {
    /// Create new auth state
    pub fn new(auth_service: AuthService) -> Self {
        Self {
            auth_service: Arc::new(auth_service),
        }
    }
}

/// Create authentication router without rate limiting
///
/// Use `auth_router_with_rate_limiting` for production deployments.
pub fn auth_router(state: AuthState) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", delete(logout))
        .with_state(state)
}

/// Create authentication router with Redis-based rate limiting
///
/// # Rate Limits
/// - `/auth/register`: 3 requests per hour per IP
/// - `/auth/login`: 5 requests per minute per IP
///
/// # Arguments
/// * `auth_state` - Authentication service state
/// * `rate_limit_state` - Rate limiting state with Redis client
pub fn auth_router_with_rate_limiting(
    auth_state: AuthState,
    rate_limit_state: AuthRateLimitState,
) -> Router {
    // Routes with rate limiting applied
    let register_route = Router::new()
        .route("/register", post(register))
        .route_layer(middleware::from_fn_with_state(
            rate_limit_state.clone(),
            register_rate_limit,
        ))
        .with_state(auth_state.clone());

    let login_route = Router::new()
        .route("/login", post(login))
        .route_layer(middleware::from_fn_with_state(
            rate_limit_state,
            login_rate_limit,
        ))
        .with_state(auth_state.clone());

    // Routes without rate limiting
    let other_routes = Router::new()
        .route("/refresh", post(refresh))
        .route("/logout", delete(logout))
        .with_state(auth_state);

    // Merge all routes
    Router::new()
        .merge(register_route)
        .merge(login_route)
        .merge(other_routes)
}

// ========== Request/Response Types ==========

/// Registration request body
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// User's email address
    pub email: String,
    /// User's password (min 8 characters)
    pub password: String,
    /// Display name for the user
    pub display_name: String,
    /// Optional device information for immediate session creation
    #[serde(default)]
    pub device_info: Option<DeviceInfo>,
}

/// Login request body
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// User's email address
    pub email: String,
    /// User's password
    pub password: String,
    /// Optional device information
    #[serde(default)]
    pub device_info: Option<DeviceInfo>,
}

/// Refresh token request body
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    /// The refresh token from a previous login/refresh
    pub refresh_token: String,
}

/// User response (safe to return to client)
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub email_verified: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            role: format!("{:?}", user.role).to_lowercase(),
            email_verified: user.email_verified,
        }
    }
}

/// Registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user: UserResponse,
    pub tokens: AuthTokens,
    pub message: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub tokens: AuthTokens,
}

/// Refresh response
#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub tokens: AuthTokens,
}

/// Logout response
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

// ========== Route Handlers ==========

/// Register a new user account
///
/// # Request
/// - Method: POST
/// - Path: /auth/register
/// - Body: JSON with email, password, display_name, optional device_info
///
/// # Response
/// - 201 Created: User registered and logged in successfully with tokens
/// - 400 Bad Request: Invalid input (weak password, invalid email)
/// - 409 Conflict: Email already exists
///
/// # Performance Optimization
/// This endpoint creates a session immediately after registration, avoiding
/// the need to call login separately. This saves an expensive Argon2id
/// password verification operation since we just hashed the password.
async fn register(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    // Extract client info from headers (same as login)
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());

    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());

    let (user, tokens) = state
        .auth_service
        .register_with_session(
            &request.email,
            &request.password,
            &request.display_name,
            request.device_info,
            ip_address,
            user_agent,
        )
        .await?;

    let response = RegisterResponse {
        user: user.into(),
        tokens,
        message: "Registration successful".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Login with email and password
///
/// # Request
/// - Method: POST
/// - Path: /auth/login
/// - Body: JSON with email, password, optional device_info
///
/// # Response
/// - 200 OK: Login successful with tokens
/// - 401 Unauthorized: Invalid credentials
async fn login(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    // Extract client info from headers
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());

    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());

    let (user, tokens) = state
        .auth_service
        .login(
            &request.email,
            &request.password,
            request.device_info,
            ip_address,
            user_agent,
        )
        .await?;

    let response = LoginResponse {
        user: user.into(),
        tokens,
    };

    Ok(Json(response))
}

/// Refresh access token using refresh token
///
/// # Request
/// - Method: POST
/// - Path: /auth/refresh
/// - Body: JSON with refresh_token
///
/// # Response
/// - 200 OK: New tokens issued
/// - 401 Unauthorized: Invalid or expired refresh token
async fn refresh(
    State(state): State<AuthState>,
    Json(request): Json<RefreshRequest>,
) -> ApiResult<impl IntoResponse> {
    let tokens = state
        .auth_service
        .refresh_token(&request.refresh_token)
        .await?;

    let response = RefreshResponse { tokens };

    Ok(Json(response))
}

/// Logout and invalidate current session
///
/// # Request
/// - Method: DELETE
/// - Path: /auth/logout
/// - Headers: Authorization: Bearer <access_token>
///
/// # Response
/// - 200 OK: Session invalidated
/// - 401 Unauthorized: Missing or invalid token
///
/// # Security
/// This endpoint extracts the session ID from the authenticated user's JWT claims,
/// ensuring users can only invalidate their own sessions.
async fn logout(State(state): State<AuthState>, auth: AuthUser) -> ApiResult<impl IntoResponse> {
    state.auth_service.logout(auth.session_id).await?;

    let response = LogoutResponse {
        message: "Logged out successfully".to_string(),
    };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_response_from_user() {
        use crate::models::user::{UserPreferences, UserRole};
        use chrono::Utc;

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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: UserResponse = user.into();
        assert_eq!(response.email, "test@example.com");
        assert_eq!(response.display_name, "Test User");
        assert_eq!(response.role, "user");
        assert!(response.email_verified);
        assert!(response.avatar_url.is_some());
    }
}
