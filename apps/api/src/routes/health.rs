//! Health check HTTP route handlers
//!
//! Provides endpoints for checking the health of the API and its dependencies:
//! - `GET /health` - Simple liveness check (returns 200 OK)
//! - `GET /health/ready` - Readiness check (verifies all dependencies)
//! - `GET /health/live` - Kubernetes-style liveness probe

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use std::sync::Arc;

use crate::config::Config;
use crate::services::HealthService;

/// Shared application state for health check handlers
#[derive(Clone)]
pub struct HealthState {
    /// Application configuration
    pub config: Arc<Config>,
    /// Health check service
    pub health_service: Arc<HealthService>,
}

impl HealthState {
    /// Create new health state from config
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            health_service: Arc::new(HealthService::new()),
        }
    }
}

/// Create health check router
pub fn health_router(state: HealthState) -> Router {
    Router::new()
        .route("/", get(simple_health))
        .route("/live", get(liveness_probe))
        .route("/ready", get(readiness_probe))
        .with_state(state)
}

/// Simple health check - always returns OK if the server is running
///
/// This is useful for load balancer health checks that just need to verify
/// the server is responding to HTTP requests.
///
/// # Response
/// - 200 OK with body "OK"
async fn simple_health() -> &'static str {
    "OK"
}

/// Liveness probe for Kubernetes
///
/// Returns 200 if the server process is running and can handle requests.
/// This should NOT check external dependencies - that's what readiness is for.
///
/// # Response
/// - 200 OK with JSON body containing status
async fn liveness_probe() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "alive",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Readiness probe - checks all external dependencies
///
/// Verifies connectivity to:
/// - PostgreSQL database
/// - Redis cache
/// - Meilisearch search engine
/// - Ollama AI service
///
/// # Response
/// - 200 OK if all services are healthy
/// - 503 Service Unavailable if any required service is unhealthy
async fn readiness_probe(State(state): State<HealthState>) -> impl IntoResponse {
    let config = &state.config;

    let response = state
        .health_service
        .check_all(
            &config.database().url,
            &config.redis().connection_url(),
            &config.meilisearch_url,
            &config.meilisearch_key,
            &config.ollama().url,
            &config.ollama().model,
        )
        .await;

    let status_code = if response.is_healthy() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_health() {
        let response = simple_health().await;
        assert_eq!(response, "OK");
    }

    #[tokio::test]
    async fn test_liveness_probe() {
        let response = liveness_probe().await;
        let json = response.into_response();
        assert_eq!(json.status(), StatusCode::OK);
    }
}
