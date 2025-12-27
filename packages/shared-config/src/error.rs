//! Configuration error types

use thiserror::Error;

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Missing required environment variable
    #[error("missing required environment variable: {0}")]
    MissingEnvVar(String),

    /// Invalid value for environment variable
    #[error("invalid value for {0}: {1}")]
    InvalidValue(String, String),

    /// Invalid URL format
    #[error("invalid URL format for {0}: {1}")]
    InvalidUrl(String, String),

    /// Configuration validation error
    #[error("configuration validation failed: {0}")]
    ValidationError(String),
}

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;
