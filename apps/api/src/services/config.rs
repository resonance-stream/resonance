//! Configuration service for runtime config loading
//!
//! This service provides centralized configuration loading with a priority system:
//! 1. Database (admin-configured settings)
//! 2. Environment variables
//! 3. Default values
//!
//! Configs are cached to reduce database queries, with automatic expiration.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

use crate::models::system_settings::ServiceType;
use crate::repositories::SystemSettingsRepository;
use crate::services::encryption::{EncryptionError, EncryptionService};

use resonance_shared_config::{LidarrConfig, OllamaConfig};

/// Cache TTL - how long configs are cached before re-fetching from DB
const CACHE_TTL: Duration = Duration::from_secs(60);

/// Errors that can occur during configuration operations
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Database error
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Encryption error when decrypting secrets
    #[error("encryption error: {0}")]
    Encryption(#[from] EncryptionError),

    /// JSON parsing error for config data
    #[error("config parsing error: {0}")]
    Parse(#[from] serde_json::Error),

    /// Service not configured (no DB, env, or defaults available)
    #[error("service not configured: {0}")]
    #[allow(dead_code)] // Part of public error API, used in tests
    NotConfigured(String),
}

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Cached configuration entry
#[derive(Debug, Clone)]
struct CachedConfig {
    /// The configuration as JSON value
    config: serde_json::Value,
    /// Decrypted secret (e.g., API key)
    secret: Option<String>,
    /// When this entry was cached
    cached_at: Instant,
}

impl CachedConfig {
    /// Check if this cache entry has expired
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > CACHE_TTL
    }
}

/// Last.fm configuration loaded from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmConfig {
    /// Last.fm API key
    pub api_key: String,
}

/// Music library configuration from database
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)] // Part of public config API, re-exported in mod.rs
pub struct MusicLibraryConfig {
    /// Path to the music library
    pub path: String,
}

/// Service for loading runtime configuration with DB -> Env -> Defaults priority
///
/// This service centralizes all configuration access, providing:
/// - Priority-based config loading (DB overrides env, env overrides defaults)
/// - Caching to reduce database queries
/// - Automatic secret decryption
/// - Cache invalidation for admin updates
#[derive(Clone)]
pub struct ConfigService {
    repo: SystemSettingsRepository,
    encryption: EncryptionService,
    cache: Arc<RwLock<HashMap<ServiceType, CachedConfig>>>,
}

impl ConfigService {
    /// Create a new ConfigService
    ///
    /// # Arguments
    /// * `repo` - Repository for database access
    /// * `encryption` - Service for decrypting secrets
    pub fn new(repo: SystemSettingsRepository, encryption: EncryptionService) -> Self {
        Self {
            repo,
            encryption,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get cached config if available and not expired
    async fn get_cached(&self, service: ServiceType) -> Option<CachedConfig> {
        let cache = self.cache.read().await;
        cache.get(&service).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.clone())
            }
        })
    }

    /// Store config in cache
    async fn set_cached(&self, service: ServiceType, entry: CachedConfig) {
        let mut cache = self.cache.write().await;
        cache.insert(service, entry);
    }

    /// Load config from database for a service, caching the result
    #[instrument(skip(self))]
    async fn load_from_db(&self, service: ServiceType) -> ConfigResult<Option<CachedConfig>> {
        // Check cache first
        if let Some(cached) = self.get_cached(service).await {
            debug!("Using cached config for {}", service);
            return Ok(Some(cached));
        }

        // Load from database
        let setting = self.repo.get_by_service(service).await?;

        match setting {
            Some(s) if s.enabled => {
                // Decrypt secrets if present
                let secret = if let Some(encrypted) = &s.encrypted_secrets {
                    match self.encryption.decrypt(encrypted) {
                        Ok(decrypted) => Some(decrypted),
                        Err(e) => {
                            warn!("Failed to decrypt secrets for {}: {}", service, e);
                            None
                        }
                    }
                } else {
                    None
                };

                let entry = CachedConfig {
                    config: s.config.clone(),
                    secret,
                    cached_at: Instant::now(),
                };

                // Cache the result
                self.set_cached(service, entry.clone()).await;
                debug!("Loaded and cached config for {} from database", service);

                Ok(Some(entry))
            }
            Some(_) => {
                debug!("Service {} is disabled in database", service);
                Ok(None)
            }
            None => {
                debug!("No database config for {}", service);
                Ok(None)
            }
        }
    }

    /// Get Ollama configuration
    ///
    /// Priority: DB -> Environment -> Defaults
    ///
    /// Returns the Ollama configuration, always providing at least default values.
    #[instrument(skip(self))]
    pub async fn get_ollama_config(&self) -> OllamaConfig {
        // Try database first
        if let Ok(Some(cached)) = self.load_from_db(ServiceType::Ollama).await {
            if let Ok(db_config) = self.parse_ollama_from_db(&cached) {
                debug!("Using Ollama config from database");
                return db_config;
            }
        }

        // Fall back to environment variables with defaults
        debug!("Using Ollama config from environment/defaults");
        OllamaConfig::from_env().unwrap_or_default()
    }

    /// Parse Ollama config from cached database entry
    fn parse_ollama_from_db(&self, cached: &CachedConfig) -> ConfigResult<OllamaConfig> {
        // Database config structure:
        // {
        //   "url": "http://ollama:11434",
        //   "model": "mistral",
        //   "embedding_model": "nomic-embed-text",
        //   "timeout_secs": 60,
        //   "max_tokens": 2048,
        //   "temperature": 0.7
        // }

        let defaults = OllamaConfig::default();

        let url = cached
            .config
            .get("url")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or(defaults.url);

        let model = cached
            .config
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or(defaults.model);

        let embedding_model = cached
            .config
            .get("embedding_model")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or(defaults.embedding_model);

        let timeout_secs = cached
            .config
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(defaults.timeout_secs);

        let max_tokens = cached
            .config
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(defaults.max_tokens);

        let temperature = cached
            .config
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(defaults.temperature);

        Ok(OllamaConfig {
            url,
            model,
            embedding_model,
            timeout_secs,
            max_tokens,
            temperature,
        })
    }

    /// Get Lidarr configuration
    ///
    /// Priority: DB -> Environment -> None
    ///
    /// Returns `None` if Lidarr is not configured (it's an optional integration).
    #[instrument(skip(self))]
    pub async fn get_lidarr_config(&self) -> Option<LidarrConfig> {
        // Try database first
        if let Ok(Some(cached)) = self.load_from_db(ServiceType::Lidarr).await {
            if let Some(config) = self.parse_lidarr_from_db(&cached) {
                debug!("Using Lidarr config from database");
                return Some(config);
            }
        }

        // Fall back to environment variables
        debug!("Checking Lidarr config from environment");
        LidarrConfig::from_env().ok()
    }

    /// Parse Lidarr config from cached database entry
    fn parse_lidarr_from_db(&self, cached: &CachedConfig) -> Option<LidarrConfig> {
        // Database config structure:
        // {
        //   "url": "http://lidarr:8686",
        //   "sync_interval_secs": 3600,
        //   "timeout_secs": 30
        // }
        // Secrets: API key stored in encrypted_secrets

        let url = cached
            .config
            .get("url")
            .and_then(|v| v.as_str())
            .map(String::from)?;

        // API key must come from decrypted secrets
        let api_key = cached.secret.clone()?;

        if url.trim().is_empty() || api_key.trim().is_empty() {
            return None;
        }

        let sync_interval_secs = cached
            .config
            .get("sync_interval_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        let timeout_secs = cached
            .config
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        Some(LidarrConfig {
            url,
            api_key,
            sync_interval_secs,
            timeout_secs,
        })
    }

    /// Get Last.fm configuration
    ///
    /// Priority: DB -> Environment -> None
    ///
    /// Returns `None` if Last.fm is not configured (it's an optional integration).
    #[instrument(skip(self))]
    pub async fn get_lastfm_config(&self) -> Option<LastFmConfig> {
        // Try database first
        if let Ok(Some(cached)) = self.load_from_db(ServiceType::Lastfm).await {
            if let Some(config) = self.parse_lastfm_from_db(&cached) {
                debug!("Using Last.fm config from database");
                return Some(config);
            }
        }

        // Fall back to environment variable
        debug!("Checking Last.fm config from environment");
        std::env::var("LASTFM_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
            .map(|api_key| LastFmConfig { api_key })
    }

    /// Parse Last.fm config from cached database entry
    fn parse_lastfm_from_db(&self, cached: &CachedConfig) -> Option<LastFmConfig> {
        // Last.fm only needs an API key, stored in encrypted_secrets
        let api_key = cached.secret.clone()?;

        if api_key.trim().is_empty() {
            return None;
        }

        Some(LastFmConfig { api_key })
    }

    /// Get the music library path
    ///
    /// Priority: DB -> Environment -> Default ("/music")
    ///
    /// Returns the configured music library path.
    #[instrument(skip(self))]
    pub async fn get_music_library_path(&self) -> PathBuf {
        // Try database first
        if let Ok(Some(cached)) = self.load_from_db(ServiceType::MusicLibrary).await {
            if let Some(path) = self.parse_music_library_from_db(&cached) {
                debug!("Using music library path from database: {:?}", path);
                return path;
            }
        }

        // Fall back to environment variable with default
        let path = std::env::var("MUSIC_LIBRARY_PATH").unwrap_or_else(|_| "/music".to_string());

        debug!(
            "Using music library path from environment/default: {}",
            path
        );
        PathBuf::from(path)
    }

    /// Parse music library path from cached database entry
    fn parse_music_library_from_db(&self, cached: &CachedConfig) -> Option<PathBuf> {
        // Database config structure:
        // {
        //   "path": "/path/to/music"
        // }

        let path = cached
            .config
            .get("path")
            .and_then(|v| v.as_str())
            .map(String::from)?;

        if path.trim().is_empty() {
            return None;
        }

        Some(PathBuf::from(path))
    }

    /// Invalidate cache for a specific service
    ///
    /// Call this after updating a service's configuration in the database.
    #[instrument(skip(self))]
    pub async fn invalidate_cache(&self, service: ServiceType) {
        let mut cache = self.cache.write().await;
        if cache.remove(&service).is_some() {
            debug!("Invalidated cache for {}", service);
        }
    }

    /// Invalidate all cached configurations
    ///
    /// Call this after bulk updates or when reloading all configs.
    #[allow(dead_code)] // Public API for future use by admin endpoints
    #[instrument(skip(self))]
    pub async fn invalidate_all(&self) {
        let mut cache = self.cache.write().await;
        let count = cache.len();
        cache.clear();
        debug!("Invalidated {} cached configs", count);
    }

    /// Check if a service is configured (either in DB or environment)
    #[instrument(skip(self))]
    pub async fn is_service_configured(&self, service: ServiceType) -> bool {
        match service {
            ServiceType::Ollama => true, // Always has defaults
            ServiceType::Lidarr => self.get_lidarr_config().await.is_some(),
            ServiceType::Lastfm => self.get_lastfm_config().await.is_some(),
            ServiceType::Meilisearch => {
                // Meilisearch is always required, check env
                std::env::var("MEILISEARCH_URL").is_ok()
                    || std::env::var("MEILISEARCH_HOST").is_ok()
            }
            ServiceType::MusicLibrary => true, // Always has default
        }
    }
}

impl std::fmt::Debug for ConfigService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigService")
            .field("repo", &"[SystemSettingsRepository]")
            .field("encryption", &"[EncryptionService]")
            .field("cache", &"[RwLock<HashMap>]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_expiration() {
        let entry = CachedConfig {
            config: serde_json::json!({}),
            secret: None,
            cached_at: Instant::now() - Duration::from_secs(120), // 2 minutes ago
        };

        assert!(entry.is_expired());

        let fresh_entry = CachedConfig {
            config: serde_json::json!({}),
            secret: None,
            cached_at: Instant::now(),
        };

        assert!(!fresh_entry.is_expired());
    }

    #[test]
    fn test_lastfm_config_serialization() {
        let config = LastFmConfig {
            api_key: "test-api-key".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-api-key"));

        let parsed: LastFmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.api_key, "test-api-key");
    }

    #[test]
    fn test_music_library_config_default() {
        let config = MusicLibraryConfig::default();
        assert!(config.path.is_empty());
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::NotConfigured("lidarr".to_string());
        assert_eq!(err.to_string(), "service not configured: lidarr");
    }
}
