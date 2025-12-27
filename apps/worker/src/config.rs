//! Worker configuration loaded from environment variables

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use resonance_shared_config::{
    CommonConfig, DatabaseConfig, Environment, LidarrConfig, OllamaConfig, RedisConfig,
};

/// Worker configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Common configuration shared with other services
    pub common: CommonConfig,

    /// Job polling interval in seconds
    pub poll_interval_secs: u64,

    /// Maximum concurrent jobs
    pub max_concurrent_jobs: usize,

    /// Maximum retry attempts for failed jobs
    pub max_retries: u32,

    /// Retry delay base in seconds (exponential backoff)
    pub retry_delay_secs: u64,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let common =
            CommonConfig::from_env().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

        Ok(Self {
            common,

            poll_interval_secs: env::var("WORKER_POLL_INTERVAL")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .context("Invalid WORKER_POLL_INTERVAL value")?,

            max_concurrent_jobs: env::var("WORKER_MAX_CONCURRENT_JOBS")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .context("Invalid WORKER_MAX_CONCURRENT_JOBS value")?,

            max_retries: env::var("WORKER_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .context("Invalid WORKER_MAX_RETRIES value")?,

            retry_delay_secs: env::var("WORKER_RETRY_DELAY")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("Invalid WORKER_RETRY_DELAY value")?,
        })
    }

    // Convenience accessors for common config fields

    /// Get database URL (for backward compatibility)
    pub fn database_url(&self) -> &str {
        &self.common.database.url
    }

    /// Get Redis URL (for backward compatibility)
    pub fn redis_url(&self) -> &str {
        &self.common.redis.url
    }

    /// Get music library path
    pub fn music_library_path(&self) -> &PathBuf {
        &self.common.music_library_path
    }

    /// Get database configuration
    pub fn database(&self) -> &DatabaseConfig {
        &self.common.database
    }

    /// Get Redis configuration
    pub fn redis(&self) -> &RedisConfig {
        &self.common.redis
    }

    /// Get Ollama configuration
    pub fn ollama(&self) -> &OllamaConfig {
        &self.common.ollama
    }

    /// Get Lidarr configuration (if configured)
    pub fn lidarr(&self) -> Option<&LidarrConfig> {
        self.common.lidarr.as_ref()
    }

    /// Get environment mode
    pub fn environment(&self) -> Environment {
        self.common.environment
    }

    /// Check if Lidarr integration is configured
    pub fn has_lidarr(&self) -> bool {
        self.common.has_lidarr()
    }

    /// Check if running in production
    pub fn is_production(&self) -> bool {
        self.common.environment.is_production()
    }
}
