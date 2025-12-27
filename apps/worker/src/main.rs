//! Resonance Background Worker
//!
//! This service handles background job processing including:
//! - Library scanning and metadata updates
//! - Audio feature extraction
//! - AI embedding generation
//! - Weekly playlist generation
//! - Lidarr integration sync
//! - Smart prefetch for autoplay

use std::sync::Arc;

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use url::Url;
use tokio::signal;
use tokio::sync::broadcast;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod jobs;

use config::Config;
use jobs::JobRunner;

pub use error::{ErrorSeverity, JobResult, WorkerError, WorkerResult};

/// Application state shared across job handlers
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool
    pub db: sqlx::PgPool,

    /// Redis connection
    pub redis: redis::Client,

    /// HTTP client for external API calls
    pub http_client: reqwest::Client,

    /// Application configuration
    pub config: Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "resonance_worker=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    tracing::info!("Starting Resonance worker");

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Loaded configuration");
    tracing::debug!("Database URL: {}", redact_url_password(config.database_url()));
    tracing::debug!("Redis URL: {}", redact_url_password(config.redis_url()));
    tracing::debug!("Music library path: {:?}", config.music_library_path());

    // Initialize database connection pool
    let db = PgPoolOptions::new()
        .max_connections(config.max_concurrent_jobs as u32 + 2)
        .connect(config.database_url())
        .await?;
    tracing::info!("Connected to PostgreSQL");

    // Initialize Redis client
    let redis = redis::Client::open(config.redis_url())?;
    // Test Redis connection
    let mut conn = redis.get_multiplexed_async_connection().await?;
    let _: String = redis::cmd("PING")
        .query_async(&mut conn)
        .await?;
    tracing::info!("Connected to Redis");

    // Initialize HTTP client for external API calls
    let http_client = reqwest::Client::builder()
        .user_agent("Resonance-Worker/0.1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // Create application state
    let state = Arc::new(AppState {
        db,
        redis,
        http_client,
        config: config.clone(),
    });

    // Create shutdown signal channel
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    // Create job runner
    let job_runner = JobRunner::new(state.clone(), shutdown_tx.subscribe());

    // Start job processing in background task
    let runner_handle = tokio::spawn(async move {
        job_runner.run().await
    });

    tracing::info!("Worker is running. Press Ctrl+C to shutdown.");

    // Wait for shutdown signal
    shutdown_signal().await;

    tracing::info!("Shutdown signal received, stopping worker...");

    // Signal all tasks to shutdown
    let _ = shutdown_tx.send(());

    // Wait for job runner to complete
    if let Err(e) = runner_handle.await {
        tracing::error!("Job runner task error: {}", e);
    }

    tracing::info!("Worker shutdown complete");

    Ok(())
}

/// Redact password from a URL for safe logging
///
/// Parses the URL and masks the password portion with asterisks.
/// If the URL cannot be parsed, returns a generic redacted message.
fn redact_url_password(url: &str) -> String {
    match Url::parse(url) {
        Ok(mut parsed) => {
            if parsed.password().is_some() {
                // Set password to redacted value
                let _ = parsed.set_password(Some("****"));
            }
            parsed.to_string()
        }
        Err(_) => {
            // If we can't parse, be safe and don't expose anything
            "[URL parse error - redacted]".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_url_password_with_password() {
        let url = "postgres://user:secretpassword@localhost:5432/dbname";
        let redacted = redact_url_password(url);
        assert!(redacted.contains("****"));
        assert!(!redacted.contains("secretpassword"));
        assert!(redacted.contains("user"));
        assert!(redacted.contains("localhost"));
    }

    #[test]
    fn test_redact_url_password_without_password() {
        let url = "postgres://localhost:5432/dbname";
        let redacted = redact_url_password(url);
        assert_eq!(redacted, "postgres://localhost:5432/dbname");
    }

    #[test]
    fn test_redact_url_password_redis() {
        let url = "redis://:myredispassword@localhost:6379";
        let redacted = redact_url_password(url);
        assert!(redacted.contains("****"));
        assert!(!redacted.contains("myredispassword"));
    }

    #[test]
    fn test_redact_url_password_invalid_url() {
        let url = "not a valid url";
        let redacted = redact_url_password(url);
        assert_eq!(redacted, "[URL parse error - redacted]");
    }
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
