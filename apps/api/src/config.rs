//! API server configuration

use std::env;

use anyhow::{Context, Result};
use resonance_shared_config::{
    CommonConfig, DatabaseConfig, Environment, LidarrConfig, OllamaConfig, RedisConfig,
};

/// API server configuration loaded from environment variables
#[derive(Debug, Clone)]
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
    pub fn from_env() -> Result<Self> {
        let common =
            CommonConfig::from_env().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

        Ok(Self {
            common,

            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("Invalid PORT value")?,

            meilisearch_url: env::var("MEILISEARCH_URL")
                .unwrap_or_else(|_| "http://localhost:7700".to_string()),

            meilisearch_key: env::var("MEILISEARCH_KEY")
                .unwrap_or_else(|_| "masterKey".to_string()),

            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "development-secret-change-in-production".to_string()),

            jwt_access_expiry: env::var("JWT_ACCESS_EXPIRY")
                .unwrap_or_else(|_| "15m".to_string()),

            jwt_refresh_expiry: env::var("JWT_REFRESH_EXPIRY")
                .unwrap_or_else(|_| "7d".to_string()),

            listenbrainz_api_key: env::var("LISTENBRAINZ_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),

            discord_client_id: env::var("DISCORD_CLIENT_ID").ok().filter(|s| !s.is_empty()),

            cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS").ok().map(|s| {
                s.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }),
        })
    }

    // Convenience accessors for common config fields

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

    /// Check if ListenBrainz scrobbling is configured
    pub fn has_listenbrainz(&self) -> bool {
        self.listenbrainz_api_key.is_some()
    }

    /// Check if Discord Rich Presence is configured
    pub fn has_discord(&self) -> bool {
        self.discord_client_id.is_some()
    }

    /// Check if running in production
    pub fn is_production(&self) -> bool {
        self.common.environment.is_production()
    }
}
