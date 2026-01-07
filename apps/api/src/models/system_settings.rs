//! System settings and setup status models for Resonance
//!
//! This module contains database models for:
//! - External service configuration (Ollama, Lidarr, LastFM, etc.)
//! - User library paths
//! - First-run setup status tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Service type enum matching PostgreSQL service_type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "service_type", rename_all = "snake_case")]
pub enum ServiceType {
    Ollama,
    Lidarr,
    Lastfm,
    Meilisearch,
    MusicLibrary,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ollama => write!(f, "ollama"),
            Self::Lidarr => write!(f, "lidarr"),
            Self::Lastfm => write!(f, "lastfm"),
            Self::Meilisearch => write!(f, "meilisearch"),
            Self::MusicLibrary => write!(f, "music_library"),
        }
    }
}

/// System setting for an external service from the system_settings table
#[derive(Debug, Clone, FromRow, Serialize)]
#[allow(dead_code)] // Infrastructure for setup wizard and admin settings
pub struct SystemSetting {
    /// Unique identifier
    pub id: Uuid,

    /// Service type (ollama, lidarr, lastfm, etc.)
    pub service: ServiceType,

    /// Whether this service is enabled
    pub enabled: bool,

    /// Configuration stored as JSON (URLs, ports, non-sensitive options)
    #[sqlx(json)]
    pub config: serde_json::Value,

    /// Encrypted secrets (API keys, tokens) - only accessible to admins
    /// Format: nonce (12 bytes) || ciphertext || auth_tag (16 bytes)
    #[serde(skip_serializing)]
    pub encrypted_secrets: Option<Vec<u8>>,

    /// Last time connection was tested
    pub last_connection_test: Option<DateTime<Utc>>,

    /// Result of last connection test
    pub connection_healthy: Option<bool>,

    /// Error message from last connection test
    pub connection_error: Option<String>,

    /// User who last updated this setting
    pub updated_by: Option<Uuid>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Input for creating/updating a system setting
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // Infrastructure for setup wizard and admin settings
pub struct SystemSettingInput {
    /// Service type to configure
    pub service: ServiceType,

    /// Whether this service is enabled
    pub enabled: bool,

    /// Configuration as JSON (URLs, ports, non-sensitive options)
    pub config: serde_json::Value,

    /// Encrypted secrets (API keys, tokens)
    #[serde(skip_serializing)]
    pub encrypted_secrets: Option<Vec<u8>>,
}

/// User library path from the user_library_paths table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct UserLibraryPath {
    /// Unique identifier
    pub id: Uuid,

    /// User who owns this path
    pub user_id: Uuid,

    /// File system path to the music library
    pub path: String,

    /// User-friendly label (e.g., "NAS Music", "Local Collection")
    pub label: Option<String>,

    /// Whether this is the primary path for the user
    pub is_primary: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Setup status singleton from the setup_status table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SetupStatus {
    /// Always 1 (singleton constraint)
    pub id: i32,

    /// Whether first-run setup has been completed
    pub completed: bool,

    /// When setup was completed
    pub completed_at: Option<DateTime<Utc>>,

    /// User who completed setup
    pub completed_by: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type_display() {
        assert_eq!(ServiceType::Ollama.to_string(), "ollama");
        assert_eq!(ServiceType::Lidarr.to_string(), "lidarr");
        assert_eq!(ServiceType::Lastfm.to_string(), "lastfm");
        assert_eq!(ServiceType::Meilisearch.to_string(), "meilisearch");
        assert_eq!(ServiceType::MusicLibrary.to_string(), "music_library");
    }

    #[test]
    fn test_system_setting_input() {
        let input = SystemSettingInput {
            service: ServiceType::Ollama,
            enabled: true,
            config: serde_json::json!({
                "url": "http://localhost:11434",
                "model": "llama2"
            }),
            encrypted_secrets: None,
        };

        assert_eq!(input.service, ServiceType::Ollama);
        assert!(input.enabled);
        assert_eq!(input.config["url"], "http://localhost:11434");
    }
}
