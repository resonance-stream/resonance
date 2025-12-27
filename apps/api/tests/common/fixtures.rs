//! Test fixtures for API integration tests
//!
//! Provides reusable test data and configuration builders.

#![allow(dead_code)]

use std::collections::HashMap;

/// Test environment variables builder
///
/// Builds a HashMap of environment variables for testing configuration loading.
#[derive(Debug, Default)]
pub struct TestEnvBuilder {
    vars: HashMap<String, String>,
}

impl TestEnvBuilder {
    /// Create a new test environment builder with minimal required variables
    pub fn new() -> Self {
        let mut builder = Self::default();
        // Set minimal defaults for development environment
        builder
            .set("ENVIRONMENT", "development")
            .set(
                "DATABASE_URL",
                "postgres://test:test@localhost:5432/resonance_test",
            )
            .set("REDIS_URL", "redis://localhost:6379")
            .set("MUSIC_LIBRARY_PATH", "/tmp/test-music");
        builder
    }

    /// Create a production-like environment
    pub fn production() -> Self {
        let mut builder = Self::default();
        builder
            .set("ENVIRONMENT", "production")
            .set(
                "DATABASE_URL",
                "postgres://prod:secret@prod-host:5432/resonance",
            )
            .set("REDIS_URL", "redis://prod-redis:6379")
            .set("MUSIC_LIBRARY_PATH", "/music")
            .set(
                "JWT_SECRET",
                "a-very-secure-secret-that-is-at-least-32-characters-long",
            )
            .set("MEILISEARCH_KEY", "production-meilisearch-key");
        builder
    }

    /// Set an environment variable
    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }

    /// Remove an environment variable
    pub fn remove(&mut self, key: &str) -> &mut Self {
        self.vars.remove(key);
        self
    }

    /// Get the environment variables as a HashMap
    pub fn build(&self) -> HashMap<String, String> {
        self.vars.clone()
    }

    /// Get the environment variables as tuples for temp_env
    pub fn as_tuples(&self) -> Vec<(String, String)> {
        self.vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

/// Test configuration for API config
#[derive(Debug, Clone)]
pub struct TestApiConfig {
    pub port: u16,
    pub jwt_secret: String,
    pub meilisearch_url: String,
    pub meilisearch_key: String,
}

impl Default for TestApiConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            jwt_secret: "test-secret-that-is-long-enough-for-testing-purposes".to_string(),
            meilisearch_url: "http://localhost:7700".to_string(),
            meilisearch_key: "test-key".to_string(),
        }
    }
}

/// Test configuration for worker config
#[derive(Debug, Clone)]
pub struct TestWorkerConfig {
    pub poll_interval_secs: u64,
    pub max_concurrent_jobs: usize,
    pub max_retries: u32,
    pub retry_delay_secs: u64,
}

impl Default for TestWorkerConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 1,
            max_concurrent_jobs: 2,
            max_retries: 1,
            retry_delay_secs: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_builder_new_has_defaults() {
        let builder = TestEnvBuilder::new();
        let vars = builder.build();

        assert!(vars.contains_key("ENVIRONMENT"));
        assert!(vars.contains_key("DATABASE_URL"));
        assert!(vars.contains_key("REDIS_URL"));
        assert_eq!(vars.get("ENVIRONMENT").unwrap(), "development");
    }

    #[test]
    fn test_env_builder_production() {
        let builder = TestEnvBuilder::production();
        let vars = builder.build();

        assert_eq!(vars.get("ENVIRONMENT").unwrap(), "production");
        assert!(vars.contains_key("JWT_SECRET"));
        assert!(vars.contains_key("MEILISEARCH_KEY"));
    }

    #[test]
    fn test_env_builder_set_and_remove() {
        let mut builder = TestEnvBuilder::new();
        builder.set("CUSTOM_VAR", "custom_value");
        assert_eq!(builder.build().get("CUSTOM_VAR").unwrap(), "custom_value");

        builder.remove("CUSTOM_VAR");
        assert!(!builder.build().contains_key("CUSTOM_VAR"));
    }

    #[test]
    fn test_default_api_config() {
        let config = TestApiConfig::default();
        assert_eq!(config.port, 8080);
        assert!(config.jwt_secret.len() >= 32);
    }

    #[test]
    fn test_default_worker_config() {
        let config = TestWorkerConfig::default();
        assert_eq!(config.poll_interval_secs, 1);
        assert_eq!(config.max_concurrent_jobs, 2);
    }
}
