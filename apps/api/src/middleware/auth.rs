//! Authentication extractors for Axum handlers
//!
//! This module provides Axum extractors for authentication:
//! - `AuthUser`: Requires valid JWT token, returns 401 if missing/invalid
//! - `MaybeAuthUser`: Optional authentication, returns None if not authenticated
//! - `AdminUser`: Requires admin role, returns 403 if not admin
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::middleware::{AuthUser, MaybeAuthUser, AdminUser};
//!
//! // Require authentication
//! async fn protected_handler(auth: AuthUser) -> impl IntoResponse {
//!     format!("Hello, {}!", auth.user.display_name)
//! }
//!
//! // Optional authentication
//! async fn optional_auth_handler(auth: MaybeAuthUser) -> impl IntoResponse {
//!     match auth.user {
//!         Some(user) => format!("Hello, {}!", user.display_name),
//!         None => "Hello, guest!".to_string(),
//!     }
//! }
//!
//! // Admin only
//! async fn admin_handler(auth: AdminUser) -> impl IntoResponse {
//!     format!("Admin access granted for {}!", auth.user.display_name)
//! }
//! ```

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::error::{ApiError, ErrorResponse};
use crate::models::user::{Claims, User, UserRole};
use crate::repositories::{SessionRepository, UserRepository};
use crate::services::AuthService;

/// Authenticated user extractor - requires valid authentication
///
/// Extracts the authenticated user from the request. Returns 401 Unauthorized
/// if no valid authentication is present.
///
/// # Example
///
/// ```rust,ignore
/// async fn handler(auth: AuthUser) -> impl IntoResponse {
///     format!("User ID: {}", auth.user.id)
/// }
/// ```
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used by handlers that consume this extractor
pub struct AuthUser {
    /// The authenticated user
    pub user: User,
    /// JWT claims from the access token
    pub claims: Claims,
    /// Session ID from the token
    pub session_id: Uuid,
}

/// Optional authentication extractor
///
/// Attempts to extract an authenticated user but doesn't fail if not present.
/// Returns None if no valid authentication is found.
///
/// # Example
///
/// ```rust,ignore
/// async fn handler(auth: MaybeAuthUser) -> impl IntoResponse {
///     if let Some(user) = auth.user {
///         format!("Welcome back, {}!", user.display_name)
///     } else {
///         "Welcome, guest!".to_string()
///     }
/// }
/// ```
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used by handlers that consume this extractor
pub struct MaybeAuthUser {
    /// The authenticated user, if present
    pub user: Option<User>,
    /// JWT claims from the access token, if present
    pub claims: Option<Claims>,
    /// Session ID from the token, if present
    pub session_id: Option<Uuid>,
}

/// Admin-only extractor - requires admin role
///
/// Extracts the authenticated user and verifies they have admin role.
/// Returns 401 if not authenticated, 403 if not admin.
///
/// # Example
///
/// ```rust,ignore
/// async fn admin_handler(auth: AdminUser) -> impl IntoResponse {
///     format!("Admin: {}", auth.user.display_name)
/// }
/// ```
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used by handlers that consume this extractor
pub struct AdminUser {
    /// The authenticated admin user
    pub user: User,
    /// JWT claims from the access token
    pub claims: Claims,
    /// Session ID from the token
    pub session_id: Uuid,
}

/// Authentication rejection types
#[derive(Debug)]
pub enum AuthRejection {
    /// Missing or invalid Authorization header
    MissingToken,
    /// Token is malformed or expired
    InvalidToken(String),
    /// Database error while fetching user
    DatabaseError(String),
    /// User not found in database
    UserNotFound,
    /// User lacks required admin permissions
    InsufficientPermissions,
    /// Missing required services in app state
    MissingServices,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            AuthRejection::MissingToken => {
                tracing::debug!("Authentication rejected: missing token");
                (StatusCode::UNAUTHORIZED, ApiError::Unauthorized)
            }
            AuthRejection::InvalidToken(reason) => {
                tracing::debug!(reason = %reason, "Authentication rejected: invalid token");
                (StatusCode::UNAUTHORIZED, ApiError::InvalidToken(reason))
            }
            AuthRejection::DatabaseError(e) => {
                tracing::error!(error = %e, "Authentication rejected: database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiError::Internal(format!("Failed to fetch user: {}", e)),
                )
            }
            AuthRejection::UserNotFound => {
                tracing::warn!("Authentication rejected: user not found");
                (
                    StatusCode::UNAUTHORIZED,
                    ApiError::InvalidToken("user not found".to_string()),
                )
            }
            AuthRejection::InsufficientPermissions => {
                tracing::warn!("Authentication rejected: insufficient permissions");
                (
                    StatusCode::FORBIDDEN,
                    ApiError::Forbidden("admin access required".to_string()),
                )
            }
            AuthRejection::MissingServices => {
                tracing::error!("Authentication rejected: missing services in app state");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiError::Internal("Authentication services not configured".to_string()),
                )
            }
        };

        let body = Json(ErrorResponse {
            code: error.error_code(),
            message: error.to_string(),
            details: None,
        });

        (status, body).into_response()
    }
}

/// Extract the bearer token from the Authorization header
fn extract_bearer_token(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract bearer token from Authorization header
        let token = extract_bearer_token(parts).ok_or(AuthRejection::MissingToken)?;

        // Get AuthService from request extensions
        let auth_service = parts
            .extensions
            .get::<AuthService>()
            .ok_or(AuthRejection::MissingServices)?;

        // Verify the access token
        let claims = auth_service
            .verify_access_token(token)
            .map_err(|e| AuthRejection::InvalidToken(e.to_string()))?;

        // Get SessionRepository from extensions
        let session_repo = parts
            .extensions
            .get::<SessionRepository>()
            .ok_or(AuthRejection::MissingServices)?;

        // Verify that the session is still active (prevents token reuse after logout)
        let session_active = session_repo
            .is_active(claims.sid, claims.sub)
            .await
            .map_err(|e| AuthRejection::DatabaseError(e.to_string()))?;

        if !session_active {
            return Err(AuthRejection::InvalidToken(
                "session is no longer active".to_string(),
            ));
        }

        // Get UserRepository from extensions
        let user_repo = parts
            .extensions
            .get::<UserRepository>()
            .ok_or(AuthRejection::MissingServices)?;

        // Fetch user from database using repository
        let user = user_repo
            .find_by_id(claims.sub)
            .await
            .map_err(|e| AuthRejection::DatabaseError(e.to_string()))?
            .ok_or(AuthRejection::UserNotFound)?;

        Ok(AuthUser {
            user,
            session_id: claims.sid,
            claims,
        })
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for MaybeAuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract bearer token - if missing, return None (not an error)
        let token = match extract_bearer_token(parts) {
            Some(t) => t,
            None => {
                return Ok(MaybeAuthUser {
                    user: None,
                    claims: None,
                    session_id: None,
                });
            }
        };

        // Get AuthService from request extensions - if missing, return None
        let auth_service = match parts.extensions.get::<AuthService>() {
            Some(s) => s,
            None => {
                tracing::warn!("AuthService not in extensions for MaybeAuthUser");
                return Ok(MaybeAuthUser {
                    user: None,
                    claims: None,
                    session_id: None,
                });
            }
        };

        // Verify the access token - if invalid, return None (not an error)
        let claims = match auth_service.verify_access_token(token) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(error = %e, "Token verification failed in MaybeAuthUser");
                return Ok(MaybeAuthUser {
                    user: None,
                    claims: None,
                    session_id: None,
                });
            }
        };

        // Get SessionRepository from extensions - if missing, return None
        let session_repo = match parts.extensions.get::<SessionRepository>() {
            Some(r) => r,
            None => {
                tracing::warn!("SessionRepository not in extensions for MaybeAuthUser");
                return Ok(MaybeAuthUser {
                    user: None,
                    claims: None,
                    session_id: None,
                });
            }
        };

        // Verify that the session is still active (prevents token reuse after logout)
        match session_repo.is_active(claims.sid, claims.sub).await {
            Ok(true) => {} // Session is active, continue
            Ok(false) => {
                tracing::debug!(session_id = %claims.sid, "Session no longer active in MaybeAuthUser");
                return Ok(MaybeAuthUser {
                    user: None,
                    claims: None,
                    session_id: None,
                });
            }
            Err(e) => {
                tracing::warn!(error = %e, "Database error checking session in MaybeAuthUser");
                return Ok(MaybeAuthUser {
                    user: None,
                    claims: None,
                    session_id: None,
                });
            }
        }

        // Get UserRepository from extensions - if missing, return None
        let user_repo = match parts.extensions.get::<UserRepository>() {
            Some(r) => r,
            None => {
                tracing::warn!("UserRepository not in extensions for MaybeAuthUser");
                return Ok(MaybeAuthUser {
                    user: None,
                    claims: None,
                    session_id: None,
                });
            }
        };

        // Fetch user from database using repository - if not found, return None
        match user_repo.find_by_id(claims.sub).await {
            Ok(Some(user)) => Ok(MaybeAuthUser {
                user: Some(user),
                session_id: Some(claims.sid),
                claims: Some(claims),
            }),
            Ok(None) => {
                tracing::debug!(user_id = %claims.sub, "User not found in MaybeAuthUser");
                Ok(MaybeAuthUser {
                    user: None,
                    claims: None,
                    session_id: None,
                })
            }
            Err(e) => {
                tracing::warn!(error = %e, "Database error in MaybeAuthUser");
                Ok(MaybeAuthUser {
                    user: None,
                    claims: None,
                    session_id: None,
                })
            }
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract bearer token from Authorization header
        let token = extract_bearer_token(parts).ok_or(AuthRejection::MissingToken)?;

        // Get AuthService from request extensions
        let auth_service = parts
            .extensions
            .get::<AuthService>()
            .ok_or(AuthRejection::MissingServices)?;

        // Verify the access token
        let claims = auth_service
            .verify_access_token(token)
            .map_err(|e| AuthRejection::InvalidToken(e.to_string()))?;

        // Check role from claims first for fast rejection
        if claims.role != UserRole::Admin {
            return Err(AuthRejection::InsufficientPermissions);
        }

        // Get SessionRepository from extensions
        let session_repo = parts
            .extensions
            .get::<SessionRepository>()
            .ok_or(AuthRejection::MissingServices)?;

        // Verify that the session is still active (prevents token reuse after logout)
        let session_active = session_repo
            .is_active(claims.sid, claims.sub)
            .await
            .map_err(|e| AuthRejection::DatabaseError(e.to_string()))?;

        if !session_active {
            return Err(AuthRejection::InvalidToken(
                "session is no longer active".to_string(),
            ));
        }

        // Get UserRepository from extensions
        let user_repo = parts
            .extensions
            .get::<UserRepository>()
            .ok_or(AuthRejection::MissingServices)?;

        // Fetch user from database using repository
        let user = user_repo
            .find_by_id(claims.sub)
            .await
            .map_err(|e| AuthRejection::DatabaseError(e.to_string()))?
            .ok_or(AuthRejection::UserNotFound)?;

        // Double-check role from database (in case it was updated after token was issued)
        if user.role != UserRole::Admin {
            return Err(AuthRejection::InsufficientPermissions);
        }

        Ok(AdminUser {
            user,
            session_id: claims.sid,
            claims,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token_valid() {
        use axum::http::{HeaderMap, HeaderValue, Request};

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9"),
        );

        let request = Request::builder()
            .header(AUTHORIZATION, "Bearer test_token_123")
            .body(())
            .unwrap();

        let (parts, _) = request.into_parts();
        let token = extract_bearer_token(&parts);
        assert_eq!(token, Some("test_token_123"));
    }

    #[test]
    fn test_extract_bearer_token_missing() {
        use axum::http::Request;

        let request = Request::builder().body(()).unwrap();
        let (parts, _) = request.into_parts();
        let token = extract_bearer_token(&parts);
        assert_eq!(token, None);
    }

    #[test]
    fn test_extract_bearer_token_invalid_scheme() {
        use axum::http::Request;

        let request = Request::builder()
            .header(AUTHORIZATION, "Basic dXNlcjpwYXNz")
            .body(())
            .unwrap();

        let (parts, _) = request.into_parts();
        let token = extract_bearer_token(&parts);
        assert_eq!(token, None);
    }

    #[test]
    fn test_auth_rejection_responses() {
        // Just verify the rejection types map to correct status codes
        let missing_token = AuthRejection::MissingToken;
        let response = missing_token.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let invalid_token = AuthRejection::InvalidToken("expired".to_string());
        let response = invalid_token.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let insufficient = AuthRejection::InsufficientPermissions;
        let response = insufficient.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let db_error = AuthRejection::DatabaseError("connection failed".to_string());
        let response = db_error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
