//! Device presence tracking
//!
//! This module handles device presence information including:
//! - Tracking which devices are online
//! - Current playback state per device
//! - Last activity timestamps for stale connection cleanup

use chrono::{DateTime, Utc};

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
