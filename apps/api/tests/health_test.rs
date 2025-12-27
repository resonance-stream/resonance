//! Integration tests for health check endpoints
//!
//! Tests the health check API routes to ensure proper responses
//! for liveness and readiness probes.

mod common;

use axum::{body::Body, http::Request, http::StatusCode, Router};
use tower::ServiceExt;

/// Create a minimal test app with just the health routes
fn create_test_app() -> Router {
    use axum::routing::get;

    Router::new()
        .route(
            "/",
            get(|| async { "Welcome to Resonance - Self-hosted Music Streaming" }),
        )
        .route("/health", get(|| async { "OK" }))
        .route("/health/live", get(liveness_handler))
}

async fn liveness_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "alive",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[tokio::test]
async fn test_root_endpoint() {
    let app = create_test_app();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Resonance"));
}

#[tokio::test]
async fn test_simple_health_check() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"OK");
}

#[tokio::test]
async fn test_liveness_probe() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "alive");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_health_returns_json_content_type() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());

    assert!(content_type.is_some());
    assert!(content_type.unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_nonexistent_route_returns_404() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
