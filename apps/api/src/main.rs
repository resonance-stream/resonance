use axum::{
    http::{header, Method},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod graphql;
mod middleware;
mod models;
mod routes;
mod services;
mod websocket;

pub use error::{ApiError, ApiResult, ErrorResponse};

use routes::{health_router, HealthState};

/// Build the CORS layer based on configuration.
///
/// In production mode:
/// - If `CORS_ORIGINS` is set, only those origins are allowed
/// - If `CORS_ORIGINS` is not set, CORS requests are rejected (no origins allowed)
///
/// In development mode:
/// - If `CORS_ORIGINS` is set, those origins are used
/// - If `CORS_ORIGINS` is not set, permissive CORS is used for convenience
fn build_cors_layer(config: &config::Config) -> CorsLayer {
    let is_production = config.is_production();

    match &config.cors_allowed_origins {
        Some(origins) if !origins.is_empty() => {
            // Parse configured origins
            let allowed_origins: Vec<_> = origins
                .iter()
                .filter_map(|origin| {
                    origin.parse().ok().or_else(|| {
                        tracing::warn!("Invalid CORS origin '{}', skipping", origin);
                        None
                    })
                })
                .collect();

            if allowed_origins.is_empty() {
                tracing::error!("No valid CORS origins configured, CORS requests will be rejected");
                CorsLayer::new()
            } else {
                tracing::info!(
                    "CORS configured with {} allowed origin(s): {:?}",
                    allowed_origins.len(),
                    origins
                );
                CorsLayer::new()
                    .allow_origin(allowed_origins)
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::PUT,
                        Method::PATCH,
                        Method::DELETE,
                        Method::OPTIONS,
                    ])
                    .allow_headers([
                        header::AUTHORIZATION,
                        header::CONTENT_TYPE,
                        header::ACCEPT,
                        header::ORIGIN,
                    ])
                    .allow_credentials(true)
                    .max_age(std::time::Duration::from_secs(3600))
            }
        }
        _ if is_production => {
            // Production without configured origins: strict CORS (no origins allowed)
            tracing::warn!(
                "CORS_ORIGINS not configured in production mode. \
                 CORS requests will be rejected. Set CORS_ORIGINS to allow cross-origin requests."
            );
            CorsLayer::new()
        }
        _ => {
            // Development without configured origins: permissive for convenience
            tracing::warn!(
                "Using permissive CORS in development mode. \
                 Set CORS_ORIGINS for production-like behavior."
            );
            CorsLayer::permissive()
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "resonance_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration
    let config = config::Config::from_env()?;

    tracing::info!("Starting Resonance API server on port {}", config.port);

    // Create health check state
    let health_state = HealthState::new(config.clone());

    // Build the CORS layer from configuration
    let cors_layer = build_cors_layer(&config);

    // Build the router
    let app = Router::new()
        .route("/", get(root))
        // Nested health routes: /health, /health/live, /health/ready
        .nest("/health", health_router(health_state))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer);

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> &'static str {
    "Welcome to Resonance - Self-hosted Music Streaming"
}
