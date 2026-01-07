//! System settings and setup status GraphQL types
//!
//! This module defines the GraphQL types for system configuration,
//! setup status, and service health monitoring.

use async_graphql::{Enum, SimpleObject};
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

use crate::models::system_settings::{ServiceType as DbServiceType, SystemSetting as DbSystemSetting};

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
