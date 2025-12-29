//! Device presence models for cross-device synchronization
//!
//! This module provides database models for tracking device presence,
//! enabling features like cross-device playback sync and "Spotify Connect"-style
//! device switching.

// Allow unused code - these models are prepared for future integration
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use thiserror::Error;
use uuid::Uuid;

/// Maximum length for device_id (matches database constraint)
pub const MAX_DEVICE_ID_LEN: usize = 128;
/// Maximum length for device_name (matches database constraint)
pub const MAX_DEVICE_NAME_LEN: usize = 255;
/// Maximum length for device_type (matches database constraint)
pub const MAX_DEVICE_TYPE_LEN: usize = 32;
/// Maximum length for user_agent (prevent unbounded strings)
pub const MAX_USER_AGENT_LEN: usize = 512;

/// Valid device types for validation
pub const VALID_DEVICE_TYPES: &[&str] =
    &["web", "mobile", "tablet", "desktop", "speaker", "unknown"];

/// Errors that can occur during device validation
#[derive(Debug, Error)]
pub enum DeviceValidationError {
    #[error("device_id cannot be empty")]
    EmptyDeviceId,
    #[error("device_id exceeds maximum length of {MAX_DEVICE_ID_LEN} (got {0})")]
    DeviceIdTooLong(usize),
    #[error("device_name exceeds maximum length of {MAX_DEVICE_NAME_LEN} (got {0})")]
    DeviceNameTooLong(usize),
    #[error("device_type exceeds maximum length of {MAX_DEVICE_TYPE_LEN} (got {0})")]
    DeviceTypeTooLong(usize),
    #[error("invalid device_type '{0}', must be one of: {VALID_DEVICE_TYPES:?}")]
    InvalidDeviceType(String),
}

/// Device presence record from the database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DevicePresence {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
    pub user_agent: Option<String>,
    pub is_connected: bool,
    pub is_active: bool,
    pub last_track_id: Option<String>,
    pub last_position_ms: Option<i64>,
    pub last_is_playing: Option<bool>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub connected_at: Option<DateTime<Utc>>,
    pub disconnected_at: Option<DateTime<Utc>>,
}

/// Data for creating or updating a device presence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertDevicePresence {
    pub user_id: Uuid,
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
    pub user_agent: Option<String>,
}

impl UpsertDevicePresence {
    pub fn new(user_id: Uuid, device_id: String, device_name: String, device_type: String) -> Self {
        Self {
            user_id,
            device_id,
            device_name,
            device_type,
            user_agent: None,
        }
    }

    pub fn with_user_agent(mut self, user_agent: Option<String>) -> Self {
        // Truncate user_agent to prevent unbounded strings
        self.user_agent = user_agent.map(|ua| {
            if ua.len() > MAX_USER_AGENT_LEN {
                ua[..MAX_USER_AGENT_LEN].to_string()
            } else {
                ua
            }
        });
        self
    }

    /// Validate the device presence data before database operations.
    ///
    /// This catches validation errors early (before hitting the database)
    /// and provides clear error messages for debugging.
    pub fn validate(&self) -> Result<(), DeviceValidationError> {
        // Validate device_id
        if self.device_id.is_empty() {
            return Err(DeviceValidationError::EmptyDeviceId);
        }
        if self.device_id.len() > MAX_DEVICE_ID_LEN {
            return Err(DeviceValidationError::DeviceIdTooLong(self.device_id.len()));
        }

        // Validate device_name
        if self.device_name.len() > MAX_DEVICE_NAME_LEN {
            return Err(DeviceValidationError::DeviceNameTooLong(
                self.device_name.len(),
            ));
        }

        // Validate device_type
        if self.device_type.len() > MAX_DEVICE_TYPE_LEN {
            return Err(DeviceValidationError::DeviceTypeTooLong(
                self.device_type.len(),
            ));
        }
        if !VALID_DEVICE_TYPES.contains(&self.device_type.as_str()) {
            return Err(DeviceValidationError::InvalidDeviceType(
                self.device_type.clone(),
            ));
        }

        Ok(())
    }
}

/// Data for updating playback state on a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDevicePlaybackState {
    pub last_track_id: Option<String>,
    pub last_position_ms: i64,
    pub last_is_playing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upsert_device_presence() {
        let user_id = Uuid::new_v4();
        let upsert = UpsertDevicePresence::new(
            user_id,
            "device-123".to_string(),
            "My Phone".to_string(),
            "mobile".to_string(),
        )
        .with_user_agent(Some("Mozilla/5.0".to_string()));

        assert_eq!(upsert.user_id, user_id);
        assert_eq!(upsert.device_id, "device-123");
        assert_eq!(upsert.device_name, "My Phone");
        assert_eq!(upsert.device_type, "mobile");
        assert_eq!(upsert.user_agent, Some("Mozilla/5.0".to_string()));
    }

    #[test]
    fn test_validation_valid_device() {
        let upsert = UpsertDevicePresence::new(
            Uuid::new_v4(),
            "device-123".to_string(),
            "My Phone".to_string(),
            "mobile".to_string(),
        );
        assert!(upsert.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_device_id() {
        let upsert = UpsertDevicePresence::new(
            Uuid::new_v4(),
            "".to_string(),
            "My Phone".to_string(),
            "mobile".to_string(),
        );
        assert!(matches!(
            upsert.validate(),
            Err(DeviceValidationError::EmptyDeviceId)
        ));
    }

    #[test]
    fn test_validation_device_id_too_long() {
        let long_id = "a".repeat(MAX_DEVICE_ID_LEN + 1);
        let upsert = UpsertDevicePresence::new(
            Uuid::new_v4(),
            long_id,
            "My Phone".to_string(),
            "mobile".to_string(),
        );
        assert!(matches!(
            upsert.validate(),
            Err(DeviceValidationError::DeviceIdTooLong(_))
        ));

        // Max length should be valid
        let max_id = "a".repeat(MAX_DEVICE_ID_LEN);
        let upsert_max = UpsertDevicePresence::new(
            Uuid::new_v4(),
            max_id,
            "My Phone".to_string(),
            "mobile".to_string(),
        );
        assert!(upsert_max.validate().is_ok());
    }

    #[test]
    fn test_validation_invalid_device_type() {
        let upsert = UpsertDevicePresence::new(
            Uuid::new_v4(),
            "device-123".to_string(),
            "My Phone".to_string(),
            "invalid_type".to_string(),
        );
        assert!(matches!(
            upsert.validate(),
            Err(DeviceValidationError::InvalidDeviceType(_))
        ));
    }

    #[test]
    fn test_all_valid_device_types() {
        for device_type in VALID_DEVICE_TYPES {
            let upsert = UpsertDevicePresence::new(
                Uuid::new_v4(),
                "device-123".to_string(),
                "Test Device".to_string(),
                device_type.to_string(),
            );
            assert!(
                upsert.validate().is_ok(),
                "device_type '{}' should be valid",
                device_type
            );
        }
    }

    #[test]
    fn test_user_agent_truncation() {
        let long_ua = "x".repeat(MAX_USER_AGENT_LEN + 100);
        let upsert = UpsertDevicePresence::new(
            Uuid::new_v4(),
            "device-123".to_string(),
            "My Phone".to_string(),
            "mobile".to_string(),
        )
        .with_user_agent(Some(long_ua));

        assert_eq!(
            upsert.user_agent.as_ref().unwrap().len(),
            MAX_USER_AGENT_LEN
        );
    }
}
