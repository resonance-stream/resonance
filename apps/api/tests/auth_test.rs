//! Integration tests for authentication flow
//!
//! Tests the complete auth lifecycle:
//! - Registration (valid, duplicate email, invalid email, weak password)
//! - Login (valid credentials, invalid credentials)
//! - Token refresh (valid token, invalid token, expired token)
//! - Logout (authenticated, unauthenticated)
//!
//! # Requirements
//!
//! These tests require a PostgreSQL database to be running. Set the `DATABASE_URL`
//! environment variable or have a local database at `postgres://resonance:resonance@localhost:5432/resonance_test`.
//!
//! To run the tests:
//! ```bash
//! # Start the test database (from project root)
//! docker compose up -d postgres
//!
//! # Run the tests
//! DATABASE_URL="postgres://resonance:resonance@localhost:5432/resonance" cargo test --test auth_test -p resonance-api
//! ```
//!
//! If the database is not available, tests will be skipped automatically.

mod common;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

use resonance_api::{
    routes::auth::{auth_router, AuthState},
    services::{AuthConfig, AuthService},
};

// ========== Test Request/Response Types ==========

#[derive(Debug, Serialize)]
struct RegisterRequest {
    email: String,
    password: String,
    display_name: String,
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    user: UserResponse,
    message: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    user: UserResponse,
    tokens: TokensResponse,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    tokens: TokensResponse,
}

#[derive(Debug, Deserialize)]
struct UserResponse {
    id: Uuid,
    email: String,
    display_name: String,
    role: String,
    email_verified: bool,
}

#[derive(Debug, Deserialize)]
struct TokensResponse {
    access_token: String,
    refresh_token: String,
    expires_at: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    code: String,
    message: String,
}

// ========== Test Fixtures ==========

/// JWT secret for testing (must be at least 32 characters)
const TEST_JWT_SECRET: &str = "test-jwt-secret-for-integration-tests-minimum-32-chars";

/// Create a test database pool connected to test database.
/// Returns None if the database is not available, allowing tests to be skipped.
async fn try_create_test_pool() -> Option<PgPool> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://resonance:resonance@localhost:5432/resonance_test".to_string()
    });

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .ok()
}

/// Macro to skip tests if the database is not available
macro_rules! require_db {
    ($pool_var:ident) => {
        let $pool_var = match try_create_test_pool().await {
            Some(p) => p,
            None => {
                eprintln!("Skipping test: database not available");
                return;
            }
        };
    };
}

/// Create a test auth service with the given pool
fn create_auth_service(pool: PgPool) -> AuthService {
    let config = AuthConfig::new(TEST_JWT_SECRET.to_string());
    AuthService::new(pool, config)
}

/// Create a test router with auth routes
fn create_test_router(auth_service: AuthService) -> Router {
    let state = AuthState::new(auth_service);
    auth_router(state)
}

/// Generate a unique email for testing to avoid conflicts
fn unique_email() -> String {
    format!("test_{}@example.com", Uuid::new_v4())
}

/// Clean up test user by email
async fn cleanup_user(pool: &PgPool, email: &str) {
    // Delete sessions first (foreign key constraint)
    let _ = sqlx::query(
        r#"
        DELETE FROM sessions WHERE user_id IN (
            SELECT id FROM users WHERE email = $1
        )
        "#,
    )
    .bind(email.to_lowercase())
    .execute(pool)
    .await;

    // Delete user
    let _ = sqlx::query("DELETE FROM users WHERE email = $1")
        .bind(email.to_lowercase())
        .execute(pool)
        .await;
}

/// Helper to make JSON POST request
fn json_post_request(uri: &str, body: &impl Serialize) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap()
}

/// Helper to make authenticated DELETE request
fn auth_delete_request(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap()
}

/// Parse response body as JSON
async fn parse_body<T: for<'de> Deserialize<'de>>(response: axum::response::Response) -> T {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// Parse response body as generic JSON Value
async fn parse_body_value(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

// ========== Registration Tests ==========

#[tokio::test]
async fn test_register_success() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();
    let request = RegisterRequest {
        email: email.clone(),
        password: "secure_password_123".to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .oneshot(json_post_request("/register", &request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body: RegisterResponse = parse_body(response).await;
    assert_eq!(body.user.email, email.to_lowercase());
    assert_eq!(body.user.display_name, "Test User");
    assert_eq!(body.user.role, "user");
    assert!(!body.user.email_verified);
    assert_eq!(body.message, "Registration successful");

    // Cleanup
    cleanup_user(&pool, &email).await;
}

#[tokio::test]
async fn test_register_duplicate_email() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();
    let request = RegisterRequest {
        email: email.clone(),
        password: "secure_password_123".to_string(),
        display_name: "Test User".to_string(),
    };

    // First registration should succeed
    let response = app
        .clone()
        .oneshot(json_post_request("/register", &request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Second registration with same email should fail
    let response = app
        .oneshot(json_post_request("/register", &request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body: ErrorResponse = parse_body(response).await;
    assert_eq!(body.code, "CONFLICT");
    assert!(body.message.contains("already exists"));

    // Cleanup
    cleanup_user(&pool, &email).await;
}

#[tokio::test]
async fn test_register_invalid_email() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let test_cases = vec![
        "invalid-email",
        "@missing-local.com",
        "missing-domain@",
        "no-domain-dots@example",
        "",
    ];

    for invalid_email in test_cases {
        let request = RegisterRequest {
            email: invalid_email.to_string(),
            password: "secure_password_123".to_string(),
            display_name: "Test User".to_string(),
        };

        let response = app
            .clone()
            .oneshot(json_post_request("/register", &request))
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Expected BAD_REQUEST for invalid email: '{}'",
            invalid_email
        );

        let body: ErrorResponse = parse_body(response).await;
        assert_eq!(body.code, "VALIDATION_ERROR");
    }
}

#[tokio::test]
async fn test_register_weak_password() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();
    let request = RegisterRequest {
        email,
        password: "short".to_string(), // Less than 8 characters
        display_name: "Test User".to_string(),
    };

    let response = app
        .oneshot(json_post_request("/register", &request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: ErrorResponse = parse_body(response).await;
    assert_eq!(body.code, "VALIDATION_ERROR");
    assert!(body.message.contains("8 characters"));
}

#[tokio::test]
async fn test_register_email_case_insensitive() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let base_email = format!("TEST_{}@EXAMPLE.COM", Uuid::new_v4());

    // Register with uppercase email
    let request = RegisterRequest {
        email: base_email.clone(),
        password: "secure_password_123".to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Email should be stored lowercase
    let body: RegisterResponse = parse_body(response).await;
    assert_eq!(body.user.email, base_email.to_lowercase());

    // Try to register again with lowercase version - should fail
    let request = RegisterRequest {
        email: base_email.to_lowercase(),
        password: "another_password_123".to_string(),
        display_name: "Another User".to_string(),
    };

    let response = app
        .oneshot(json_post_request("/register", &request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    // Cleanup
    cleanup_user(&pool, &base_email).await;
}

// ========== Login Tests ==========

#[tokio::test]
async fn test_login_success() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();
    let password = "secure_password_123";

    // First, register a user
    let register_request = RegisterRequest {
        email: email.clone(),
        password: password.to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &register_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Now login
    let login_request = LoginRequest {
        email: email.clone(),
        password: password.to_string(),
    };

    let response = app
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: LoginResponse = parse_body(response).await;
    assert_eq!(body.user.email, email.to_lowercase());
    assert!(!body.tokens.access_token.is_empty());
    assert!(!body.tokens.refresh_token.is_empty());
    assert_eq!(body.tokens.token_type, "Bearer");

    // Cleanup
    cleanup_user(&pool, &email).await;
}

#[tokio::test]
async fn test_login_invalid_password() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();

    // Register a user
    let register_request = RegisterRequest {
        email: email.clone(),
        password: "correct_password_123".to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &register_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Try to login with wrong password
    let login_request = LoginRequest {
        email: email.clone(),
        password: "wrong_password_123".to_string(),
    };

    let response = app
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: ErrorResponse = parse_body(response).await;
    assert_eq!(body.code, "UNAUTHORIZED");

    // Cleanup
    cleanup_user(&pool, &email).await;
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let login_request = LoginRequest {
        email: format!("nonexistent_{}@example.com", Uuid::new_v4()),
        password: "any_password_123".to_string(),
    };

    let response = app
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: ErrorResponse = parse_body(response).await;
    assert_eq!(body.code, "UNAUTHORIZED");
}

#[tokio::test]
async fn test_login_email_case_insensitive() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = format!("test_{}@example.com", Uuid::new_v4());
    let password = "secure_password_123";

    // Register with lowercase email
    let register_request = RegisterRequest {
        email: email.clone(),
        password: password.to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &register_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Login with uppercase email
    let login_request = LoginRequest {
        email: email.to_uppercase(),
        password: password.to_string(),
    };

    let response = app
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: LoginResponse = parse_body(response).await;
    assert_eq!(body.user.email, email.to_lowercase());

    // Cleanup
    cleanup_user(&pool, &email).await;
}

// ========== Token Refresh Tests ==========

#[tokio::test]
async fn test_refresh_token_success() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();
    let password = "secure_password_123";

    // Register
    let register_request = RegisterRequest {
        email: email.clone(),
        password: password.to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &register_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Login to get tokens
    let login_request = LoginRequest {
        email: email.clone(),
        password: password.to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let login_body: LoginResponse = parse_body(response).await;
    let original_refresh_token = login_body.tokens.refresh_token;

    // Use refresh token to get new tokens
    let refresh_request = RefreshRequest {
        refresh_token: original_refresh_token.clone(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/refresh", &refresh_request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let refresh_body: RefreshResponse = parse_body(response).await;
    assert!(!refresh_body.tokens.access_token.is_empty());
    assert!(!refresh_body.tokens.refresh_token.is_empty());
    assert_eq!(refresh_body.tokens.token_type, "Bearer");

    // New refresh token should be different (token rotation)
    assert_ne!(refresh_body.tokens.refresh_token, original_refresh_token);

    // Cleanup
    cleanup_user(&pool, &email).await;
}

#[tokio::test]
async fn test_refresh_token_invalid() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let refresh_request = RefreshRequest {
        refresh_token: "invalid_token_that_does_not_exist".to_string(),
    };

    let response = app
        .oneshot(json_post_request("/refresh", &refresh_request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: ErrorResponse = parse_body(response).await;
    assert_eq!(body.code, "INVALID_TOKEN");
}

#[tokio::test]
async fn test_refresh_token_rotation_prevents_reuse() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();
    let password = "secure_password_123";

    // Register and login
    let register_request = RegisterRequest {
        email: email.clone(),
        password: password.to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &register_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let login_request = LoginRequest {
        email: email.clone(),
        password: password.to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let login_body: LoginResponse = parse_body(response).await;
    let original_refresh_token = login_body.tokens.refresh_token;

    // First refresh - should succeed
    let refresh_request = RefreshRequest {
        refresh_token: original_refresh_token.clone(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/refresh", &refresh_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Second refresh with the same (old) token - should fail due to token rotation
    let refresh_request = RefreshRequest {
        refresh_token: original_refresh_token,
    };

    let response = app
        .oneshot(json_post_request("/refresh", &refresh_request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: ErrorResponse = parse_body(response).await;
    assert_eq!(body.code, "INVALID_TOKEN");

    // Cleanup
    cleanup_user(&pool, &email).await;
}

// ========== Logout Tests ==========

#[tokio::test]
async fn test_logout_success() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());

    // We need to inject services into extensions for the auth extractor to work
    // For this test, we'll create a custom app setup
    use axum::{middleware, Extension};

    let auth_svc = auth_service.clone();
    let app = Router::new()
        .merge(auth_router(AuthState::new(auth_service.clone())))
        .layer(Extension(auth_svc))
        .layer(Extension(pool.clone()));

    let email = unique_email();
    let password = "secure_password_123";

    // Register
    let register_request = RegisterRequest {
        email: email.clone(),
        password: password.to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &register_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Login to get access token
    let login_request = LoginRequest {
        email: email.clone(),
        password: password.to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let login_body: LoginResponse = parse_body(response).await;
    let access_token = login_body.tokens.access_token;

    // Logout
    let response = app
        .clone()
        .oneshot(auth_delete_request("/logout", &access_token))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = parse_body_value(response).await;
    assert!(body["message"].as_str().unwrap().contains("success"));

    // Cleanup
    cleanup_user(&pool, &email).await;
}

#[tokio::test]
async fn test_logout_unauthenticated() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());

    use axum::Extension;

    let auth_svc = auth_service.clone();
    let app = Router::new()
        .merge(auth_router(AuthState::new(auth_service)))
        .layer(Extension(auth_svc))
        .layer(Extension(pool.clone()));

    // Try to logout without auth token
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: ErrorResponse = parse_body(response).await;
    assert_eq!(body.code, "UNAUTHORIZED");
}

#[tokio::test]
async fn test_logout_invalid_token() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());

    use axum::Extension;

    let auth_svc = auth_service.clone();
    let app = Router::new()
        .merge(auth_router(AuthState::new(auth_service)))
        .layer(Extension(auth_svc))
        .layer(Extension(pool.clone()));

    // Try to logout with invalid token
    let response = app
        .oneshot(auth_delete_request("/logout", "invalid_token"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: ErrorResponse = parse_body(response).await;
    assert_eq!(body.code, "INVALID_TOKEN");
}

#[tokio::test]
async fn test_logout_prevents_refresh_token_usage() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());

    use axum::Extension;

    let auth_svc = auth_service.clone();
    let app = Router::new()
        .merge(auth_router(AuthState::new(auth_service.clone())))
        .layer(Extension(auth_svc))
        .layer(Extension(pool.clone()));

    let email = unique_email();
    let password = "secure_password_123";

    // Register
    let register_request = RegisterRequest {
        email: email.clone(),
        password: password.to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &register_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Login
    let login_request = LoginRequest {
        email: email.clone(),
        password: password.to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let login_body: LoginResponse = parse_body(response).await;
    let access_token = login_body.tokens.access_token;
    let refresh_token = login_body.tokens.refresh_token;

    // Logout
    let response = app
        .clone()
        .oneshot(auth_delete_request("/logout", &access_token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Try to use refresh token after logout - should fail
    let refresh_request = RefreshRequest { refresh_token };

    let response = app
        .oneshot(json_post_request("/refresh", &refresh_request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: ErrorResponse = parse_body(response).await;
    assert_eq!(body.code, "INVALID_TOKEN");

    // Cleanup
    cleanup_user(&pool, &email).await;
}

// ========== Multiple Sessions Tests ==========

#[tokio::test]
async fn test_multiple_login_sessions() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();
    let password = "secure_password_123";

    // Register
    let register_request = RegisterRequest {
        email: email.clone(),
        password: password.to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &register_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Login from "device 1"
    let login_request = LoginRequest {
        email: email.clone(),
        password: password.to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let session1: LoginResponse = parse_body(response).await;

    // Login from "device 2"
    let response = app
        .clone()
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let session2: LoginResponse = parse_body(response).await;

    // Both sessions should have different tokens
    assert_ne!(session1.tokens.access_token, session2.tokens.access_token);
    assert_ne!(session1.tokens.refresh_token, session2.tokens.refresh_token);

    // Both refresh tokens should work
    let refresh_request = RefreshRequest {
        refresh_token: session1.tokens.refresh_token,
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/refresh", &refresh_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let refresh_request = RefreshRequest {
        refresh_token: session2.tokens.refresh_token,
    };

    let response = app
        .oneshot(json_post_request("/refresh", &refresh_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Cleanup
    cleanup_user(&pool, &email).await;
}

// ========== Edge Case Tests ==========

#[tokio::test]
async fn test_register_with_whitespace_email() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let base_email = unique_email();
    let email_with_spaces = format!("  {}  ", base_email);

    let request = RegisterRequest {
        email: email_with_spaces,
        password: "secure_password_123".to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .oneshot(json_post_request("/register", &request))
        .await
        .unwrap();

    // Should either trim and succeed, or fail validation
    // The current implementation should fail because email validation
    // happens before any trimming
    if response.status() == StatusCode::CREATED {
        let body: RegisterResponse = parse_body(response).await;
        // If it succeeded, the email should be normalized
        assert_eq!(body.user.email, base_email.to_lowercase());
        cleanup_user(&pool, &base_email).await;
    } else {
        // If it failed, that's also acceptable behavior
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn test_login_with_empty_password() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();

    // Register first with a valid password
    let register_request = RegisterRequest {
        email: email.clone(),
        password: "secure_password_123".to_string(),
        display_name: "Test User".to_string(),
    };

    let response = app
        .clone()
        .oneshot(json_post_request("/register", &register_request))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Try to login with empty password
    let login_request = LoginRequest {
        email: email.clone(),
        password: "".to_string(),
    };

    let response = app
        .oneshot(json_post_request("/login", &login_request))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Cleanup
    cleanup_user(&pool, &email).await;
}

#[tokio::test]
async fn test_register_very_long_display_name() {
    require_db!(pool);
    let auth_service = create_auth_service(pool.clone());
    let app = create_test_router(auth_service);

    let email = unique_email();
    let long_name = "A".repeat(500); // Very long display name

    let request = RegisterRequest {
        email: email.clone(),
        password: "secure_password_123".to_string(),
        display_name: long_name.clone(),
    };

    let response = app
        .oneshot(json_post_request("/register", &request))
        .await
        .unwrap();

    // This could either succeed (if no validation) or fail (if validation exists)
    // Both are valid behaviors - document whichever happens
    if response.status() == StatusCode::CREATED {
        let body: RegisterResponse = parse_body(response).await;
        // If it succeeded, display name should be stored (possibly truncated)
        assert!(!body.user.display_name.is_empty());
        cleanup_user(&pool, &email).await;
    } else {
        // If it failed with validation error, that's also acceptable
        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}
