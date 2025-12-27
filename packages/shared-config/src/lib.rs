//! Shared configuration types for Resonance services
//!
//! This crate provides common configuration types used by both the API
//! and worker services, ensuring consistency across the application.

mod database;
mod error;
mod lidarr;
mod ollama;
mod redis;

pub use database::DatabaseConfig;
pub use error::{ConfigError, ConfigResult};
pub use lidarr::LidarrConfig;
pub use ollama::OllamaConfig;
pub use redis::RedisConfig;

use std::env;
use std::path::PathBuf;

/// Common configuration shared between all services
#[derive(Debug, Clone)]
pub struct CommonConfig {
    /// Database configuration
    pub database: DatabaseConfig,

    /// Redis configuration
    pub redis: RedisConfig,

    /// Path to music library
    pub music_library_path: PathBuf,

    /// Lidarr integration configuration (optional)
    pub lidarr: Option<LidarrConfig>,

    /// Ollama AI configuration
    pub ollama: OllamaConfig,

    /// Environment mode (development, staging, production)
    pub environment: Environment,

    /// Log level (from RUST_LOG or LOG_LEVEL)
    pub log_level: String,
}

/// Application environment mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Environment {
    #[default]
    Development,
    Staging,
    Production,
}

impl Environment {
    /// Parse environment from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "production" | "prod" => Self::Production,
            "staging" | "stage" => Self::Staging,
            _ => Self::Development,
        }
    }

    /// Check if this is a production environment
    pub fn is_production(&self) -> bool {
        matches!(self, Self::Production)
    }

    /// Check if this is a development environment
    pub fn is_development(&self) -> bool {
        matches!(self, Self::Development)
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Staging => write!(f, "staging"),
            Self::Production => write!(f, "production"),
        }
    }
}

impl CommonConfig {
    /// Load common configuration from environment variables
    pub fn from_env() -> ConfigResult<Self> {
        Ok(Self {
            database: DatabaseConfig::from_env()?,
            redis: RedisConfig::from_env()?,
            music_library_path: PathBuf::from(
                env::var("MUSIC_LIBRARY_PATH").unwrap_or_else(|_| "/music".to_string()),
            ),
            lidarr: LidarrConfig::from_env().ok(),
            ollama: OllamaConfig::from_env()?,
            environment: Environment::from_str(
                &env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            ),
            log_level: env::var("RUST_LOG")
                .or_else(|_| env::var("LOG_LEVEL"))
                .unwrap_or_else(|_| "info".to_string()),
        })
    }

    /// Check if Lidarr integration is configured
    pub fn has_lidarr(&self) -> bool {
        self.lidarr.is_some()
    }
}

/// Helper function to get a required environment variable
pub fn get_required_env(name: &str) -> ConfigResult<String> {
    env::var(name).map_err(|_| ConfigError::MissingEnvVar(name.to_string()))
}

/// Helper function to get an optional environment variable with a default
pub fn get_env_or_default(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

/// Helper function to parse an environment variable into a specific type
pub fn parse_env<T>(name: &str, default: T) -> ConfigResult<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(val) => val
            .parse()
            .map_err(|e| ConfigError::InvalidValue(name.to_string(), format!("{}", e))),
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_parsing() {
        assert_eq!(Environment::from_str("production"), Environment::Production);
        assert_eq!(Environment::from_str("prod"), Environment::Production);
        assert_eq!(Environment::from_str("staging"), Environment::Staging);
        assert_eq!(Environment::from_str("stage"), Environment::Staging);
        assert_eq!(Environment::from_str("development"), Environment::Development);
        assert_eq!(Environment::from_str("dev"), Environment::Development);
        assert_eq!(Environment::from_str("anything"), Environment::Development);
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(format!("{}", Environment::Production), "production");
        assert_eq!(format!("{}", Environment::Staging), "staging");
        assert_eq!(format!("{}", Environment::Development), "development");
    }

    #[test]
    fn test_environment_checks() {
        assert!(Environment::Production.is_production());
        assert!(!Environment::Production.is_development());
        assert!(Environment::Development.is_development());
        assert!(!Environment::Development.is_production());
    }
}
