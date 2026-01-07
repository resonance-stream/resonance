//! System settings and setup status GraphQL types
//!
//! This module defines the GraphQL types for system configuration,
//! setup status, and service health monitoring.

use async_graphql::{Enum, InputObject, SimpleObject, ID};
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::models::system_settings::{
    ServiceType as DbServiceType, SystemSetting as DbSystemSetting,
    UserLibraryPath as DbUserLibraryPath,
};

/// External service type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum ServiceType {
    /// Ollama AI service for embeddings and recommendations
    Ollama,
    /// Lidarr for automatic music library management
    Lidarr,
    /// Last.fm for scrobbling and similar artists
    Lastfm,
    /// Meilisearch for full-text search
    Meilisearch,
    /// Music library path configuration
    MusicLibrary,
}

impl From<DbServiceType> for ServiceType {
    fn from(db: DbServiceType) -> Self {
        match db {
            DbServiceType::Ollama => Self::Ollama,
            DbServiceType::Lidarr => Self::Lidarr,
            DbServiceType::Lastfm => Self::Lastfm,
            DbServiceType::Meilisearch => Self::Meilisearch,
            DbServiceType::MusicLibrary => Self::MusicLibrary,
        }
    }
}

impl From<ServiceType> for DbServiceType {
    fn from(gql: ServiceType) -> Self {
        match gql {
            ServiceType::Ollama => Self::Ollama,
            ServiceType::Lidarr => Self::Lidarr,
            ServiceType::Lastfm => Self::Lastfm,
            ServiceType::Meilisearch => Self::Meilisearch,
            ServiceType::MusicLibrary => Self::MusicLibrary,
        }
    }
}

/// First-run setup status for the setup wizard
#[derive(Debug, Clone, SimpleObject)]
pub struct SetupStatus {
    /// Whether the first-run setup has been completed
    pub is_complete: bool,
    /// Whether at least one admin user exists
    pub has_admin: bool,
    /// List of services that have been configured
    pub configured_services: Vec<ServiceType>,
}

/// System setting information (safe for admin viewing)
///
/// Note: This type never exposes actual secrets - only indicates whether
/// secrets have been configured via `has_secret`.
#[derive(Debug, Clone, SimpleObject)]
pub struct SystemSettingInfo {
    /// The service type this setting configures
    pub service: ServiceType,
    /// Whether this service is enabled
    pub enabled: bool,
    /// Non-sensitive configuration (URLs, ports, options)
    /// Serialized as JSON
    pub config: JsonValue,
    /// Whether encrypted secrets are configured (never exposes actual secrets)
    pub has_secret: bool,
    /// Last time a connection test was performed
    pub last_connection_test: Option<DateTime<Utc>>,
    /// Result of the last connection test
    pub connection_healthy: Option<bool>,
    /// Error message from the last connection test (if failed)
    pub connection_error: Option<String>,
}

impl From<DbSystemSetting> for SystemSettingInfo {
    fn from(setting: DbSystemSetting) -> Self {
        Self {
            service: setting.service.into(),
            enabled: setting.enabled,
            config: setting.config,
            has_secret: setting.encrypted_secrets.is_some(),
            last_connection_test: setting.last_connection_test,
            connection_healthy: setting.connection_healthy,
            connection_error: setting.connection_error,
        }
    }
}

// =============================================================================
// Mutation Input Types
// =============================================================================

/// Input for creating the initial admin user during setup
#[derive(Debug, Clone, InputObject)]
pub struct CreateAdminInput {
    /// Admin username (for display)
    pub username: String,
    /// Admin email address
    pub email: String,
    /// Admin password (minimum 8 characters)
    pub password: String,
}

/// Input for updating a system setting
#[derive(Debug, Clone, InputObject)]
pub struct UpdateSystemSettingInput {
    /// The service to update
    pub service: ServiceType,
    /// Whether to enable or disable the service
    pub enabled: Option<bool>,
    /// Non-sensitive configuration as JSON string
    /// Example: {"url": "http://localhost:11434", "model": "mistral"}
    pub config: Option<String>,
    /// Secret value (API key, password, etc.) - will be encrypted before storage
    pub secret: Option<String>,
}

// =============================================================================
// Mutation Output Types
// =============================================================================

/// Result of testing a service connection
#[derive(Debug, Clone, SimpleObject)]
pub struct ConnectionTestResult {
    /// Whether the connection test was successful
    pub success: bool,
    /// Response time in milliseconds (if successful)
    pub response_time_ms: Option<i64>,
    /// Version of the service (if available)
    pub version: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// User library path configuration
#[derive(Debug, Clone, SimpleObject)]
pub struct UserLibraryPath {
    /// Unique identifier for this path
    pub id: ID,
    /// The file system path
    pub path: String,
    /// User-friendly label (e.g., "NAS Music", "Local Collection")
    pub label: Option<String>,
    /// Whether this is the user's primary library path
    pub is_primary: bool,
    /// When this path was added
    pub created_at: DateTime<Utc>,
}

impl From<DbUserLibraryPath> for UserLibraryPath {
    fn from(path: DbUserLibraryPath) -> Self {
        Self {
            id: ID(path.id.to_string()),
            path: path.path,
            label: path.label,
            is_primary: path.is_primary,
            created_at: path.created_at,
        }
    }
}

/// Helper to parse a GraphQL ID into a UUID
impl UserLibraryPath {
    /// Parse an ID string into a UUID
    pub fn parse_id(id: &ID) -> Result<Uuid, async_graphql::Error> {
        id.parse::<Uuid>()
            .map_err(|_| async_graphql::Error::new("Invalid library path ID"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type_conversion() {
        // DB to GraphQL
        assert_eq!(
            ServiceType::from(DbServiceType::Ollama),
            ServiceType::Ollama
        );
        assert_eq!(
            ServiceType::from(DbServiceType::Lidarr),
            ServiceType::Lidarr
        );
        assert_eq!(
            ServiceType::from(DbServiceType::Lastfm),
            ServiceType::Lastfm
        );
        assert_eq!(
            ServiceType::from(DbServiceType::Meilisearch),
            ServiceType::Meilisearch
        );
        assert_eq!(
            ServiceType::from(DbServiceType::MusicLibrary),
            ServiceType::MusicLibrary
        );

        // GraphQL to DB
        assert_eq!(
            DbServiceType::from(ServiceType::Ollama),
            DbServiceType::Ollama
        );
        assert_eq!(
            DbServiceType::from(ServiceType::Lidarr),
            DbServiceType::Lidarr
        );
    }

    #[test]
    fn test_system_setting_info_has_secret() {
        use serde_json::json;
        use uuid::Uuid;

        // Setting without secrets
        let setting_no_secret = DbSystemSetting {
            id: Uuid::new_v4(),
            service: DbServiceType::Ollama,
            enabled: true,
            config: json!({"url": "http://localhost:11434"}),
            encrypted_secrets: None,
            last_connection_test: None,
            connection_healthy: None,
            connection_error: None,
            updated_by: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let info: SystemSettingInfo = setting_no_secret.into();
        assert!(!info.has_secret);

        // Setting with secrets
        let setting_with_secret = DbSystemSetting {
            id: Uuid::new_v4(),
            service: DbServiceType::Lastfm,
            enabled: true,
            config: json!({}),
            encrypted_secrets: Some(vec![1, 2, 3, 4]),
            last_connection_test: None,
            connection_healthy: None,
            connection_error: None,
            updated_by: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let info: SystemSettingInfo = setting_with_secret.into();
        assert!(info.has_secret);
    }
}
