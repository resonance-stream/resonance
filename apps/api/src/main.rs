use axum::{routing::get, Router};
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod graphql;
mod models;
mod routes;
mod services;
mod websocket;

pub use error::{ApiError, ApiResult, ErrorResponse};

use routes::{health_router, HealthState};

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

    // Build the router
    let app = Router::new()
        .route("/", get(root))
        // Nested health routes: /health, /health/live, /health/ready
        .nest("/health", health_router(health_state))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

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
