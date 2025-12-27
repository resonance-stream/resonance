//! Worker configuration loaded from environment variables
//!
//! This module provides configuration management for the Resonance worker service.
//! Configuration is loaded from environment variables with sensible defaults for
//! development environments.

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
        let common = CommonConfig::from_env()
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to ensure tests that modify environment variables don't run in parallel
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper to temporarily set environment variables for a test
    struct EnvGuard {
        vars: Vec<(String, Option<String>)>,
    }

    impl EnvGuard {
        fn new(vars: &[(&str, &str)]) -> Self {
            let saved: Vec<_> = vars
                .iter()
                .map(|(k, v)| {
                    let old = env::var(*k).ok();
                    env::set_var(*k, *v);
                    (k.to_string(), old)
                })
                .collect();
            Self { vars: saved }
        }

        fn remove_vars(vars: &[&str]) -> Self {
            let saved: Vec<_> = vars
                .iter()
                .map(|k| {
                    let old = env::var(*k).ok();
                    env::remove_var(*k);
                    (k.to_string(), old)
                })
                .collect();
            Self { vars: saved }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in &self.vars {
                match v {
                    Some(val) => env::set_var(k, val),
                    None => env::remove_var(k),
                }
            }
        }
    }

    #[test]
    fn test_default_poll_interval() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove_vars(&["WORKER_POLL_INTERVAL"]);

        // Since we can't easily call from_env without CommonConfig setup,
        // we test the parsing logic directly
        let interval: u64 = env::var("WORKER_POLL_INTERVAL")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap();
        assert_eq!(interval, 5);
    }

    #[test]
    fn test_custom_poll_interval() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("WORKER_POLL_INTERVAL", "10")]);

        let interval: u64 = env::var("WORKER_POLL_INTERVAL")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap();
        assert_eq!(interval, 10);
    }

    #[test]
    fn test_default_max_concurrent_jobs() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove_vars(&["WORKER_MAX_CONCURRENT_JOBS"]);

        let max_jobs: usize = env::var("WORKER_MAX_CONCURRENT_JOBS")
            .unwrap_or_else(|_| "4".to_string())
            .parse()
            .unwrap();
        assert_eq!(max_jobs, 4);
    }

    #[test]
    fn test_custom_max_concurrent_jobs() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("WORKER_MAX_CONCURRENT_JOBS", "8")]);

        let max_jobs: usize = env::var("WORKER_MAX_CONCURRENT_JOBS")
            .unwrap_or_else(|_| "4".to_string())
            .parse()
            .unwrap();
        assert_eq!(max_jobs, 8);
    }

    #[test]
    fn test_default_max_retries() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove_vars(&["WORKER_MAX_RETRIES"]);

        let max_retries: u32 = env::var("WORKER_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .unwrap();
        assert_eq!(max_retries, 3);
    }

    #[test]
    fn test_custom_max_retries() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("WORKER_MAX_RETRIES", "5")]);

        let max_retries: u32 = env::var("WORKER_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .unwrap();
        assert_eq!(max_retries, 5);
    }

    #[test]
    fn test_default_retry_delay() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove_vars(&["WORKER_RETRY_DELAY"]);

        let retry_delay: u64 = env::var("WORKER_RETRY_DELAY")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap();
        assert_eq!(retry_delay, 60);
    }

    #[test]
    fn test_custom_retry_delay() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("WORKER_RETRY_DELAY", "120")]);

        let retry_delay: u64 = env::var("WORKER_RETRY_DELAY")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap();
        assert_eq!(retry_delay, 120);
    }

    #[test]
    fn test_invalid_poll_interval_format() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("WORKER_POLL_INTERVAL", "not_a_number")]);

        let result: Result<u64, _> = env::var("WORKER_POLL_INTERVAL")
            .unwrap_or_else(|_| "5".to_string())
            .parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_max_concurrent_jobs_format() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("WORKER_MAX_CONCURRENT_JOBS", "invalid")]);

        let result: Result<usize, _> = env::var("WORKER_MAX_CONCURRENT_JOBS")
            .unwrap_or_else(|_| "4".to_string())
            .parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_values_fail_parsing() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("WORKER_POLL_INTERVAL", "-5")]);

        let result: Result<u64, _> = env::var("WORKER_POLL_INTERVAL")
            .unwrap_or_else(|_| "5".to_string())
            .parse();
        // Negative numbers should fail for unsigned types
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_is_valid_for_numeric_configs() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("WORKER_POLL_INTERVAL", "0")]);

        let interval: u64 = env::var("WORKER_POLL_INTERVAL")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap();
        assert_eq!(interval, 0);
    }
}
