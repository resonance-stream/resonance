use std::env;

use anyhow::{Context, Result};

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    /// Server port (default: 8080)
    pub port: u16,

    /// Database connection URL
    pub database_url: String,

    /// Redis connection URL
    pub redis_url: String,

    /// Meilisearch URL
    pub meilisearch_url: String,

    /// Meilisearch API key
    pub meilisearch_key: String,

    /// JWT secret for authentication
    pub jwt_secret: String,

    /// Path to music library
    pub music_library_path: String,

    /// Lidarr API URL (optional)
    pub lidarr_url: Option<String>,

    /// Lidarr API key (optional)
    pub lidarr_api_key: Option<String>,

    /// Ollama URL for AI features
    pub ollama_url: String,

    /// Ollama model to use
    pub ollama_model: String,

    /// ListenBrainz API key (optional)
    pub listenbrainz_api_key: Option<String>,

    /// Discord client ID for Rich Presence (optional)
    pub discord_client_id: Option<String>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("Invalid PORT value")?,

            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://resonance:resonance@localhost:5432/resonance".to_string()),

            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),

            meilisearch_url: env::var("MEILISEARCH_URL")
                .unwrap_or_else(|_| "http://localhost:7700".to_string()),

            meilisearch_key: env::var("MEILISEARCH_KEY")
                .unwrap_or_else(|_| "masterKey".to_string()),

            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "development-secret-change-in-production".to_string()),

            music_library_path: env::var("MUSIC_LIBRARY_PATH")
                .unwrap_or_else(|_| "/music".to_string()),

            lidarr_url: env::var("LIDARR_URL").ok(),
            lidarr_api_key: env::var("LIDARR_API_KEY").ok(),

            ollama_url: env::var("OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),

            ollama_model: env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "mistral".to_string()),

            listenbrainz_api_key: env::var("LISTENBRAINZ_API_KEY").ok(),
            discord_client_id: env::var("DISCORD_CLIENT_ID").ok(),
        })
    }

    /// Check if Lidarr integration is configured
    pub fn has_lidarr(&self) -> bool {
        self.lidarr_url.is_some() && self.lidarr_api_key.is_some()
    }

    /// Check if ListenBrainz scrobbling is configured
    pub fn has_listenbrainz(&self) -> bool {
        self.listenbrainz_api_key.is_some()
    }

    /// Check if Discord Rich Presence is configured
    pub fn has_discord(&self) -> bool {
        self.discord_client_id.is_some()
    }
}
