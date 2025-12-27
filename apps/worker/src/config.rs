//! Worker configuration loaded from environment variables

use std::env;

use anyhow::{Context, Result};

/// Worker configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Database connection URL
    pub database_url: String,

    /// Redis connection URL
    pub redis_url: String,

    /// Path to music library
    pub music_library_path: String,

    /// Lidarr API URL (optional)
    pub lidarr_url: Option<String>,

    /// Lidarr API key (optional)
    pub lidarr_api_key: Option<String>,

    /// Ollama URL for AI features
    pub ollama_url: String,

    /// Ollama model to use
    pub ollama_model: String,

    /// Job polling interval in seconds
    pub poll_interval_secs: u64,

    /// Maximum concurrent jobs
    pub max_concurrent_jobs: usize,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://resonance:resonance@localhost:5432/resonance".to_string()),

            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),

            music_library_path: env::var("MUSIC_LIBRARY_PATH")
                .unwrap_or_else(|_| "/music".to_string()),

            lidarr_url: env::var("LIDARR_URL").ok(),
            lidarr_api_key: env::var("LIDARR_API_KEY").ok(),

            ollama_url: env::var("OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),

            ollama_model: env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "mistral".to_string()),

            poll_interval_secs: env::var("WORKER_POLL_INTERVAL")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .context("Invalid WORKER_POLL_INTERVAL value")?,

            max_concurrent_jobs: env::var("WORKER_MAX_CONCURRENT_JOBS")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .context("Invalid WORKER_MAX_CONCURRENT_JOBS value")?,
        })
    }

    /// Check if Lidarr integration is configured
    pub fn has_lidarr(&self) -> bool {
        self.lidarr_url.is_some() && self.lidarr_api_key.is_some()
    }
}
