//! Lidarr integration configuration types

use crate::{get_required_env, parse_env, ConfigError, ConfigResult};
use std::env;

/// Lidarr music library manager configuration
#[derive(Debug, Clone)]
pub struct LidarrConfig {
    /// Lidarr server URL
    pub url: String,

    /// Lidarr API key
    pub api_key: String,

    /// Sync interval in seconds (how often to check for new music)
    pub sync_interval_secs: u64,

    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl LidarrConfig {
    /// Load Lidarr configuration from environment variables
    ///
    /// Returns an error if the required variables (URL and API key) are not set.
    /// This allows consumers to call `.ok()` to get `Option<LidarrConfig>`.
    pub fn from_env() -> ConfigResult<Self> {
        let url = get_required_env("LIDARR_URL")?;
        let api_key = get_required_env("LIDARR_API_KEY")?;

        // Validate that URL is not empty
        if url.trim().is_empty() {
            return Err(ConfigError::InvalidValue(
                "LIDARR_URL".to_string(),
                "URL cannot be empty".to_string(),
            ));
        }

        // Validate that API key is not empty
        if api_key.trim().is_empty() {
            return Err(ConfigError::InvalidValue(
                "LIDARR_API_KEY".to_string(),
                "API key cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            url,
            api_key,
            sync_interval_secs: parse_env("LIDARR_SYNC_INTERVAL", 3600)?, // Default: 1 hour
            timeout_secs: parse_env("LIDARR_TIMEOUT", 30)?,
        })
    }

    /// Check if Lidarr is configured (both URL and API key are set)
    pub fn is_configured() -> bool {
        env::var("LIDARR_URL").is_ok() && env::var("LIDARR_API_KEY").is_ok()
    }

    /// Create a configuration with custom URL and API key (useful for testing)
    pub fn new(url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            api_key: api_key.into(),
            sync_interval_secs: 3600,
            timeout_secs: 30,
        }
    }

    /// Get the full URL for the API endpoint
    pub fn api_url(&self, path: &str) -> String {
        let base = self.url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/api/v1/{}", base, path)
    }

    /// Get headers required for Lidarr API requests
    pub fn api_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("X-Api-Key", self.api_key.clone()),
            ("Content-Type", "application/json".to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_config() {
        let config = LidarrConfig::new("http://lidarr:8686", "test-api-key");
        assert_eq!(config.url, "http://lidarr:8686");
        assert_eq!(config.api_key, "test-api-key");
        assert_eq!(config.sync_interval_secs, 3600);
    }

    #[test]
    fn test_api_url() {
        let config = LidarrConfig::new("http://lidarr:8686", "key");
        assert_eq!(config.api_url("artist"), "http://lidarr:8686/api/v1/artist");
        assert_eq!(config.api_url("/album"), "http://lidarr:8686/api/v1/album");
    }

    #[test]
    fn test_api_url_with_trailing_slash() {
        let config = LidarrConfig::new("http://lidarr:8686/", "key");
        assert_eq!(config.api_url("artist"), "http://lidarr:8686/api/v1/artist");
    }

    #[test]
    fn test_api_headers() {
        let config = LidarrConfig::new("http://lidarr:8686", "test-key");
        let headers = config.api_headers();
        assert_eq!(headers.len(), 2);
        assert!(headers
            .iter()
            .any(|(k, v)| *k == "X-Api-Key" && v == "test-key"));
    }
}
