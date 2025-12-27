//! API server configuration

use std::env;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use resonance_shared_config::{
    CommonConfig, DatabaseConfig, Environment, LidarrConfig, OllamaConfig, RedisConfig,
};

/// Minimum required length for JWT_SECRET to be considered secure
const MIN_JWT_SECRET_LENGTH: usize = 32;

/// API server configuration loaded from environment variables
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    /// Common configuration shared with other services
    pub common: CommonConfig,

    /// Server port (default: 8080)
    pub port: u16,

    /// Meilisearch URL
    pub meilisearch_url: String,

    /// Meilisearch API key
    pub meilisearch_key: String,

    /// JWT secret for authentication
    pub jwt_secret: String,

    /// JWT access token expiry (default: 15m)
    pub jwt_access_expiry: String,

    /// JWT refresh token expiry (default: 7d)
    pub jwt_refresh_expiry: String,

    /// ListenBrainz API key (optional)
    pub listenbrainz_api_key: Option<String>,

    /// Discord client ID for Rich Presence (optional)
    pub discord_client_id: Option<String>,

    /// CORS allowed origins (optional)
    pub cors_allowed_origins: Option<Vec<String>>,
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// In production mode, this function requires:
    /// - `JWT_SECRET`: Must be set and at least 32 characters long
    /// - `MEILISEARCH_KEY`: Must be explicitly set (no insecure defaults)
    /// - `DATABASE_URL`: Must be explicitly set (no insecure defaults)
    ///
    /// In development/staging mode, sensible defaults are used for convenience.
    pub fn from_env() -> Result<Self> {
        // Determine environment first to know if we need strict validation
        let environment = Environment::from_str(
            &env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        )
        .unwrap_or_default();
        let is_production = environment.is_production();

        // Validate and load security-critical configuration
        let jwt_secret = Self::load_jwt_secret(is_production)?;
        let meilisearch_key = Self::load_meilisearch_key(is_production)?;

        // Validate DATABASE_URL is explicitly set in production
        if is_production {
            Self::validate_database_url()?;
        }

        let common = CommonConfig::from_env()
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

        Ok(Self {
            common,

            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("Invalid PORT value")?,

            meilisearch_url: env::var("MEILISEARCH_URL")
                .unwrap_or_else(|_| "http://localhost:7700".to_string()),

            meilisearch_key,

            jwt_secret,

            jwt_access_expiry: env::var("JWT_ACCESS_EXPIRY").unwrap_or_else(|_| "15m".to_string()),

            jwt_refresh_expiry: env::var("JWT_REFRESH_EXPIRY").unwrap_or_else(|_| "7d".to_string()),

            listenbrainz_api_key: env::var("LISTENBRAINZ_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),

            discord_client_id: env::var("DISCORD_CLIENT_ID").ok().filter(|s| !s.is_empty()),

            cors_allowed_origins: env::var("CORS_ORIGINS").ok().map(|s| {
                s.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }),
        })
    }

    /// Load and validate JWT_SECRET
    ///
    /// In production:
    /// - JWT_SECRET must be explicitly set
    /// - Must be at least MIN_JWT_SECRET_LENGTH characters
    ///
    /// In development: uses a default value with a warning
    fn load_jwt_secret(is_production: bool) -> Result<String> {
        match env::var("JWT_SECRET") {
            Ok(secret) if !secret.is_empty() => {
                if is_production && secret.len() < MIN_JWT_SECRET_LENGTH {
                    bail!(
                        "JWT_SECRET must be at least {} characters in production (got {})",
                        MIN_JWT_SECRET_LENGTH,
                        secret.len()
                    );
                }
                Ok(secret)
            }
            _ if is_production => {
                bail!(
                    "JWT_SECRET environment variable is required in production. \
                     Please set a secure secret of at least {} characters.",
                    MIN_JWT_SECRET_LENGTH
                );
            }
            _ => {
                // Development mode: use default but log a warning
                tracing::warn!(
                    "JWT_SECRET not set, using insecure default. \
                     This is only acceptable in development mode."
                );
                Ok("development-secret-change-in-production".to_string())
            }
        }
    }

    /// Load and validate MEILISEARCH_KEY
    ///
    /// In production: MEILISEARCH_KEY must be explicitly set
    /// In development: uses a default value
    fn load_meilisearch_key(is_production: bool) -> Result<String> {
        match env::var("MEILISEARCH_KEY") {
            Ok(key) if !key.is_empty() => Ok(key),
            _ if is_production => {
                bail!(
                    "MEILISEARCH_KEY environment variable is required in production. \
                     Please set your Meilisearch master key."
                );
            }
            _ => {
                tracing::warn!(
                    "MEILISEARCH_KEY not set, using insecure default. \
                     This is only acceptable in development mode."
                );
                Ok("masterKey".to_string())
            }
        }
    }

    /// Validate that DATABASE_URL is explicitly set in production
    fn validate_database_url() -> Result<()> {
        match env::var("DATABASE_URL") {
            Ok(url) if !url.is_empty() => Ok(()),
            _ => {
                bail!(
                    "DATABASE_URL environment variable is required in production. \
                     Please set your PostgreSQL connection string."
                );
            }
        }
    }

    // Convenience accessors for common config fields

    /// Get database configuration
    #[allow(dead_code)]
    pub fn database(&self) -> &DatabaseConfig {
        &self.common.database
    }

    /// Get Redis configuration
    #[allow(dead_code)]
    pub fn redis(&self) -> &RedisConfig {
        &self.common.redis
    }

    /// Get Ollama configuration
    #[allow(dead_code)]
    pub fn ollama(&self) -> &OllamaConfig {
        &self.common.ollama
    }

    /// Get Lidarr configuration (if configured)
    #[allow(dead_code)]
    pub fn lidarr(&self) -> Option<&LidarrConfig> {
        self.common.lidarr.as_ref()
    }

    /// Get environment mode
    #[allow(dead_code)]
    pub fn environment(&self) -> Environment {
        self.common.environment
    }

    /// Check if Lidarr integration is configured
    #[allow(dead_code)]
    pub fn has_lidarr(&self) -> bool {
        self.common.has_lidarr()
    }

    /// Check if ListenBrainz scrobbling is configured
    #[allow(dead_code)]
    pub fn has_listenbrainz(&self) -> bool {
        self.listenbrainz_api_key.is_some()
    }

    /// Check if Discord Rich Presence is configured
    #[allow(dead_code)]
    pub fn has_discord(&self) -> bool {
        self.discord_client_id.is_some()
    }

    /// Check if running in production
    #[allow(dead_code)]
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
    fn test_jwt_secret_required_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove_vars(&["JWT_SECRET"]);

        let result = Config::load_jwt_secret(true);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("JWT_SECRET"));
        assert!(err.contains("required in production"));
    }

    #[test]
    fn test_jwt_secret_minimum_length_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("JWT_SECRET", "short")]);

        let result = Config::load_jwt_secret(true);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("at least 32 characters"));
    }

    #[test]
    fn test_jwt_secret_valid_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let secret = "a".repeat(MIN_JWT_SECRET_LENGTH);
        let _guard = EnvGuard::new(&[("JWT_SECRET", &secret)]);

        let result = Config::load_jwt_secret(true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), secret);
    }

    #[test]
    fn test_jwt_secret_uses_default_in_development() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove_vars(&["JWT_SECRET"]);

        let result = Config::load_jwt_secret(false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "development-secret-change-in-production");
    }

    #[test]
    fn test_meilisearch_key_required_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove_vars(&["MEILISEARCH_KEY"]);

        let result = Config::load_meilisearch_key(true);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("MEILISEARCH_KEY"));
        assert!(err.contains("required in production"));
    }

    #[test]
    fn test_meilisearch_key_valid_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("MEILISEARCH_KEY", "my-secure-key")]);

        let result = Config::load_meilisearch_key(true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "my-secure-key");
    }

    #[test]
    fn test_meilisearch_key_uses_default_in_development() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove_vars(&["MEILISEARCH_KEY"]);

        let result = Config::load_meilisearch_key(false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "masterKey");
    }

    #[test]
    fn test_database_url_required_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove_vars(&["DATABASE_URL"]);

        let result = Config::validate_database_url();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("DATABASE_URL"));
        assert!(err.contains("required in production"));
    }

    #[test]
    fn test_database_url_valid_when_set() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("DATABASE_URL", "postgres://user:pass@host/db")]);

        let result = Config::validate_database_url();
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_jwt_secret_fails_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("JWT_SECRET", "")]);

        let result = Config::load_jwt_secret(true);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_meilisearch_key_fails_in_production() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("MEILISEARCH_KEY", "")]);

        let result = Config::load_meilisearch_key(true);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_database_url_fails() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(&[("DATABASE_URL", "")]);

        let result = Config::validate_database_url();
        assert!(result.is_err());
    }
}
