//! Device presence tracking
//!
//! This module handles device presence information including:
//! - Tracking which devices are online
//! - Current playback state per device
//! - Last activity timestamps for stale connection cleanup

use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::messages::{DevicePresence, DeviceType, PlaybackState, TrackSummary};

/// Presence information for a single device
#[derive(Debug, Clone)]
pub struct DevicePresenceInfo {
    /// Unique device identifier
    pub device_id: String,
    /// Human-readable device name
    pub device_name: String,
    /// Type of device
    pub device_type: DeviceType,
    /// Whether this device is currently controlling playback
    pub is_active: bool,
    /// Current playback state (if any)
    pub playback_state: Option<PlaybackState>,
    /// Last activity timestamp
    pub last_seen: DateTime<Utc>,
    /// Device user agent (for device type detection)
    pub user_agent: Option<String>,
}

impl DevicePresenceInfo {
    /// Create a new device presence
    pub fn new(
        device_id: String,
        device_name: String,
        device_type: DeviceType,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            device_id,
            device_name,
            device_type,
            is_active: false,
            playback_state: None,
            last_seen: Utc::now(),
            user_agent,
        }
    }

    /// Update last seen timestamp to now
    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }

    /// Check if the device is stale (no activity for given duration)
    pub fn is_stale(&self, max_age: chrono::Duration) -> bool {
        Utc::now() - self.last_seen > max_age
    }

    /// Convert to DevicePresence message type
    pub fn to_presence(&self) -> DevicePresence {
        let current_track = self.playback_state.as_ref().and_then(|state| {
            state.track_id.as_ref().map(|id| TrackSummary {
                id: id.clone(),
                title: String::new(), // Would need to be populated from track data
                artist: String::new(),
            })
        });

        DevicePresence {
            device_id: self.device_id.clone(),
            device_name: self.device_name.clone(),
            device_type: self.device_type,
            is_active: self.is_active,
            current_track,
            volume: self
                .playback_state
                .as_ref()
                .map(|s| s.volume)
                .unwrap_or(1.0),
            last_seen: self.last_seen.timestamp_millis(),
        }
    }
}

/// Presence store for all devices of a user
#[derive(Debug)]
pub struct UserPresenceStore {
    devices: Arc<RwLock<Vec<DevicePresenceInfo>>>,
    active_device_id: Arc<RwLock<Option<String>>>,
}

impl UserPresenceStore {
    /// Create a new user presence store
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(Vec::new())),
            active_device_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Add or update a device
    pub async fn upsert_device(&self, info: DevicePresenceInfo) {
        let mut devices = self.devices.write().await;
        if let Some(existing) = devices.iter_mut().find(|d| d.device_id == info.device_id) {
            *existing = info;
        } else {
            devices.push(info);
        }
    }

    /// Remove a device
    pub async fn remove_device(&self, device_id: &str) {
        let mut devices = self.devices.write().await;
        devices.retain(|d| d.device_id != device_id);

        // Clear active device if it was removed
        let mut active = self.active_device_id.write().await;
        if active.as_deref() == Some(device_id) {
            *active = None;
        }
    }

    /// Get all devices
    pub async fn get_devices(&self) -> Vec<DevicePresenceInfo> {
        self.devices.read().await.clone()
    }

    /// Get a specific device
    pub async fn get_device(&self, device_id: &str) -> Option<DevicePresenceInfo> {
        self.devices
            .read()
            .await
            .iter()
            .find(|d| d.device_id == device_id)
            .cloned()
    }

    /// Check if a device exists
    pub async fn device_exists(&self, device_id: &str) -> bool {
        self.devices
            .read()
            .await
            .iter()
            .any(|d| d.device_id == device_id)
    }

    /// Update device's last seen time
    pub async fn touch_device(&self, device_id: &str) {
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.iter_mut().find(|d| d.device_id == device_id) {
            device.touch();
        }
    }

    /// Set the active device
    pub async fn set_active_device(&self, device_id: &str) {
        let mut devices = self.devices.write().await;
        let mut active = self.active_device_id.write().await;

        // Update is_active flag on all devices
        for device in devices.iter_mut() {
            device.is_active = device.device_id == device_id;
        }

        *active = Some(device_id.to_string());
    }

    /// Clear the active device
    pub async fn clear_active_device(&self) {
        let mut devices = self.devices.write().await;
        let mut active = self.active_device_id.write().await;

        for device in devices.iter_mut() {
            device.is_active = false;
        }

        *active = None;
    }

    /// Get the active device ID
    pub async fn get_active_device(&self) -> Option<String> {
        self.active_device_id.read().await.clone()
    }

    /// Update playback state for a device
    pub async fn update_playback_state(&self, device_id: &str, state: PlaybackState) {
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.iter_mut().find(|d| d.device_id == device_id) {
            device.playback_state = Some(state);
            device.touch();
        }
    }

    /// Get the current playback state (from active device)
    pub async fn get_playback_state(&self) -> Option<PlaybackState> {
        let active = self.active_device_id.read().await;
        let active_id = active.as_ref()?;

        let devices = self.devices.read().await;
        devices
            .iter()
            .find(|d| d.device_id == *active_id)
            .and_then(|d| d.playback_state.clone())
    }

    /// Remove stale devices
    pub async fn cleanup_stale(&self, max_age: chrono::Duration) -> Vec<String> {
        let mut devices = self.devices.write().await;
        let stale_ids: Vec<String> = devices
            .iter()
            .filter(|d| d.is_stale(max_age))
            .map(|d| d.device_id.clone())
            .collect();

        devices.retain(|d| !d.is_stale(max_age));

        // Clear active device if it was stale
        let mut active = self.active_device_id.write().await;
        if let Some(active_id) = active.as_ref() {
            if stale_ids.contains(active_id) {
                *active = None;
            }
        }

        stale_ids
    }

    /// Convert all devices to DevicePresence list
    pub async fn to_presence_list(&self) -> Vec<DevicePresence> {
        self.devices
            .read()
            .await
            .iter()
            .map(|d| d.to_presence())
            .collect()
    }
}

impl Default for UserPresenceStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect device type from user agent string
pub fn detect_device_type(user_agent: Option<&str>) -> DeviceType {
    let ua = match user_agent {
        Some(ua) => ua.to_lowercase(),
        None => return DeviceType::Unknown,
    };

    if ua.contains("mobile") || ua.contains("android") || ua.contains("iphone") {
        DeviceType::Mobile
    } else if ua.contains("ipad") || ua.contains("tablet") {
        DeviceType::Tablet
    } else if ua.contains("electron") || ua.contains("desktop") {
        DeviceType::Desktop
    } else if ua.contains("speaker") || ua.contains("cast") || ua.contains("sonos") {
        DeviceType::Speaker
    } else if ua.contains("mozilla") || ua.contains("chrome") || ua.contains("safari") {
        DeviceType::Web
    } else {
        DeviceType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_presence_store() {
        let store = UserPresenceStore::new();

        // Add a device
        let device = DevicePresenceInfo::new(
            "device-1".to_string(),
            "My Phone".to_string(),
            DeviceType::Mobile,
            None,
        );
        store.upsert_device(device).await;

        // Should exist
        assert!(store.device_exists("device-1").await);

        // Get device
        let retrieved = store.get_device("device-1").await.unwrap();
        assert_eq!(retrieved.device_name, "My Phone");

        // Remove device
        store.remove_device("device-1").await;
        assert!(!store.device_exists("device-1").await);
    }

    #[tokio::test]
    async fn test_active_device() {
        let store = UserPresenceStore::new();

        // Add devices
        store
            .upsert_device(DevicePresenceInfo::new(
                "device-1".to_string(),
                "Phone".to_string(),
                DeviceType::Mobile,
                None,
            ))
            .await;
        store
            .upsert_device(DevicePresenceInfo::new(
                "device-2".to_string(),
                "Desktop".to_string(),
                DeviceType::Desktop,
                None,
            ))
            .await;

        // No active device initially
        assert!(store.get_active_device().await.is_none());

        // Set active device
        store.set_active_device("device-1").await;
        assert_eq!(
            store.get_active_device().await,
            Some("device-1".to_string())
        );

        // Check is_active flags
        let devices = store.get_devices().await;
        let device_1 = devices.iter().find(|d| d.device_id == "device-1").unwrap();
        let device_2 = devices.iter().find(|d| d.device_id == "device-2").unwrap();
        assert!(device_1.is_active);
        assert!(!device_2.is_active);

        // Clear active device
        store.clear_active_device().await;
        assert!(store.get_active_device().await.is_none());
    }

    #[test]
    fn test_detect_device_type() {
        assert_eq!(
            detect_device_type(Some(
                "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X)"
            )),
            DeviceType::Mobile
        );
        assert_eq!(
            detect_device_type(Some("Mozilla/5.0 (iPad; CPU OS 14_0 like Mac OS X)")),
            DeviceType::Tablet
        );
        assert_eq!(
            detect_device_type(Some(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/91.0"
            )),
            DeviceType::Web
        );
        assert_eq!(
            detect_device_type(Some("Resonance Desktop/1.0 Electron/13.0")),
            DeviceType::Desktop
        );
        assert_eq!(detect_device_type(None), DeviceType::Unknown);
    }

    #[test]
    fn test_device_staleness() {
        let mut device = DevicePresenceInfo::new(
            "device-1".to_string(),
            "Test".to_string(),
            DeviceType::Web,
            None,
        );

        // Set last_seen to 2 hours ago
        device.last_seen = Utc::now() - chrono::Duration::hours(2);

        // Should be stale with 1 hour max age
        assert!(device.is_stale(chrono::Duration::hours(1)));

        // Should not be stale with 3 hour max age
        assert!(!device.is_stale(chrono::Duration::hours(3)));
    }
}
