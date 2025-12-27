//! Database configuration types

use crate::{get_env_or_default, parse_env, ConfigResult};

/// PostgreSQL database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Full connection URL (e.g., postgres://user:pass@host:port/db)
    pub url: String,

    /// Maximum number of connections in the pool
    pub max_connections: u32,

    /// Minimum number of connections to maintain
    pub min_connections: u32,

    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,

    /// Idle timeout for connections in seconds
    pub idle_timeout_secs: u64,
}

impl DatabaseConfig {
    /// Load database configuration from environment variables
    pub fn from_env() -> ConfigResult<Self> {
        Ok(Self {
            url: get_env_or_default(
                "DATABASE_URL",
                "postgres://resonance:resonance@localhost:5432/resonance",
            ),
            max_connections: parse_env("DATABASE_MAX_CONNECTIONS", 10)?,
            min_connections: parse_env("DATABASE_MIN_CONNECTIONS", 2)?,
            connect_timeout_secs: parse_env("DATABASE_CONNECT_TIMEOUT", 30)?,
            idle_timeout_secs: parse_env("DATABASE_IDLE_TIMEOUT", 600)?,
        })
    }

    /// Create a configuration with a custom URL (useful for testing)
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            max_connections: 10,
            min_connections: 2,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://resonance:resonance@localhost:5432/resonance".to_string(),
            max_connections: 10,
            min_connections: 2,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DatabaseConfig::default();
        assert!(config.url.contains("resonance"));
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
    }

    #[test]
    fn test_with_url() {
        let config = DatabaseConfig::with_url("postgres://test:test@localhost/test");
        assert_eq!(config.url, "postgres://test:test@localhost/test");
    }
}
