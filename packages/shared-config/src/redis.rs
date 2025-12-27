//! Redis configuration types

use crate::{get_env_or_default, parse_env, ConfigResult};

/// Redis configuration
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,

    /// Optional password for Redis authentication
    pub password: Option<String>,

    /// Connection pool size
    pub pool_size: u32,

    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
}

impl RedisConfig {
    /// Load Redis configuration from environment variables
    pub fn from_env() -> ConfigResult<Self> {
        Ok(Self {
            url: get_env_or_default("REDIS_URL", "redis://localhost:6379"),
            password: std::env::var("REDIS_PASSWORD").ok().filter(|s| !s.is_empty()),
            pool_size: parse_env("REDIS_POOL_SIZE", 10)?,
            connect_timeout_secs: parse_env("REDIS_CONNECT_TIMEOUT", 5)?,
        })
    }

    /// Create a configuration with a custom URL (useful for testing)
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            password: None,
            pool_size: 10,
            connect_timeout_secs: 5,
        }
    }

    /// Build the full connection URL including password if set
    pub fn connection_url(&self) -> String {
        if let Some(ref password) = self.password {
            // Parse URL and insert password
            if let Some(at_pos) = self.url.find("://") {
                let (scheme, rest) = self.url.split_at(at_pos + 3);
                return format!("{}:{}@{}", scheme.trim_end_matches("://"), password, rest);
            }
        }
        self.url.clone()
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            password: None,
            pool_size: 10,
            connect_timeout_secs: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RedisConfig::default();
        assert_eq!(config.url, "redis://localhost:6379");
        assert!(config.password.is_none());
        assert_eq!(config.pool_size, 10);
    }

    #[test]
    fn test_with_url() {
        let config = RedisConfig::with_url("redis://custom:6380");
        assert_eq!(config.url, "redis://custom:6380");
    }

    #[test]
    fn test_connection_url_no_password() {
        let config = RedisConfig::default();
        assert_eq!(config.connection_url(), "redis://localhost:6379");
    }
}
