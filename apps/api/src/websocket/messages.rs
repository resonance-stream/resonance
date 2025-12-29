//! WebSocket message types for real-time synchronization
//!
//! This module defines the message protocol for client-server communication
//! over WebSocket connections. Messages are serialized as JSON.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// Client -> Server Messages
// =============================================================================

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// Update playback state (from active device)
    PlaybackStateUpdate(PlaybackState),

    /// Seek to position (from active device)
    Seek { position_ms: u64 },

    /// Update queue state
    QueueUpdate(QueueState),

    /// Request to transfer playback to another device
    TransferPlayback { target_device_id: String },

    /// Request list of connected devices
    RequestDeviceList,

    /// Heartbeat to keep connection alive
    Heartbeat,

    /// Update synced settings
    SettingsUpdate(SyncedSettings),
}

// =============================================================================
// Server -> Client Messages
// =============================================================================

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    /// Connection established successfully
    Connected(ConnectedPayload),

    /// Error occurred
    Error(ErrorPayload),

    /// Playback state sync (from another device)
    PlaybackSync(PlaybackState),

    /// Seek sync (from another device)
    SeekSync { position_ms: u64, timestamp: i64 },

    /// Queue state sync
    QueueSync(QueueState),

    /// List of connected devices
    DeviceList(Vec<DevicePresence>),

    /// A device connected
    DeviceConnected(DevicePresence),

    /// A device disconnected
    DeviceDisconnected { device_id: String },

    /// Playback transfer requested
    TransferRequested { from_device_id: String },

    /// Playback transfer accepted
    TransferAccepted { to_device_id: String },

    /// Heartbeat response
    Pong { server_time: i64 },

    /// Settings sync
    SettingsSync(SyncedSettings),
}

// =============================================================================
// Payload Types
// =============================================================================

/// Payload for Connected message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedPayload {
    pub device_id: String,
    pub session_id: Uuid,
    /// Current active device (if any)
    pub active_device_id: Option<String>,
}

/// Payload for Error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

impl ErrorPayload {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn auth_failed(message: impl Into<String>) -> Self {
        Self::new("AUTH_FAILED", message)
    }

    pub fn invalid_message(message: impl Into<String>) -> Self {
        Self::new("INVALID_MESSAGE", message)
    }

    pub fn rate_limited(retry_after: u64) -> Self {
        Self::new(
            "RATE_LIMITED",
            format!("Retry after {} seconds", retry_after),
        )
    }

    pub fn device_not_found(device_id: &str) -> Self {
        Self::new(
            "DEVICE_NOT_FOUND",
            format!("Device {} not found", device_id),
        )
    }

    pub fn not_active_device() -> Self {
        Self::new(
            "NOT_ACTIVE_DEVICE",
            "Only the active device can control playback",
        )
    }
}

/// Playback state for synchronization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaybackState {
    /// Currently playing track ID (null if nothing playing)
    pub track_id: Option<String>,

    /// Whether playback is active
    pub is_playing: bool,

    /// Current position in milliseconds
    pub position_ms: u64,

    /// Unix timestamp (ms) when this state was captured
    /// Used for clock drift correction
    pub timestamp: i64,

    /// Volume level (0.0 - 1.0)
    pub volume: f32,

    /// Whether audio is muted
    pub is_muted: bool,

    /// Shuffle mode enabled
    pub shuffle: bool,

    /// Repeat mode
    pub repeat: RepeatMode,
}

impl PlaybackState {
    /// Create a new playback state with current timestamp
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now().timestamp_millis(),
            ..Default::default()
        }
    }

    /// Update timestamp to current time
    pub fn with_current_timestamp(mut self) -> Self {
        self.timestamp = chrono::Utc::now().timestamp_millis();
        self
    }
}

/// Repeat mode options
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    #[default]
    Off,
    Track,
    Queue,
}

/// Queue state for synchronization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueState {
    /// Tracks in queue
    pub tracks: Vec<QueueTrack>,

    /// Current position in queue (index)
    pub current_index: usize,
}

/// Minimal track info for queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album_title: String,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
}

/// Device presence information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePresence {
    /// Unique device identifier
    pub device_id: String,

    /// Human-readable device name
    pub device_name: String,

    /// Type of device
    pub device_type: DeviceType,

    /// Whether this device is currently controlling playback
    pub is_active: bool,

    /// Current track (if playing)
    pub current_track: Option<TrackSummary>,

    /// Volume level
    pub volume: f32,

    /// Last activity timestamp (Unix ms)
    pub last_seen: i64,
}

impl DevicePresence {
    pub fn new(device_id: String, device_name: String, device_type: DeviceType) -> Self {
        Self {
            device_id,
            device_name,
            device_type,
            is_active: false,
            current_track: None,
            volume: 1.0,
            last_seen: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Device type categories
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    #[default]
    Web,
    Desktop,
    Mobile,
    Tablet,
    Speaker,
    Unknown,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Web => write!(f, "web"),
            DeviceType::Desktop => write!(f, "desktop"),
            DeviceType::Mobile => write!(f, "mobile"),
            DeviceType::Tablet => write!(f, "tablet"),
            DeviceType::Speaker => write!(f, "speaker"),
            DeviceType::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for DeviceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web" => Ok(DeviceType::Web),
            "desktop" => Ok(DeviceType::Desktop),
            "mobile" => Ok(DeviceType::Mobile),
            "tablet" => Ok(DeviceType::Tablet),
            "speaker" => Ok(DeviceType::Speaker),
            _ => Ok(DeviceType::Unknown),
        }
    }
}

/// Minimal track info for presence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackSummary {
    pub id: String,
    pub title: String,
    pub artist: String,
}

/// Settings that are synced across devices
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncedSettings {
    /// Crossfade enabled
    pub crossfade_enabled: Option<bool>,

    /// Crossfade duration in seconds
    pub crossfade_duration: Option<f32>,

    /// Gapless playback enabled
    pub gapless_enabled: Option<bool>,

    /// Volume normalization enabled
    pub normalize_volume: Option<bool>,
}

// =============================================================================
// Internal Sync Events (for Redis pub/sub)
// =============================================================================

/// Events published through Redis pub/sub
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum SyncEvent {
    /// Playback state changed
    PlaybackUpdate {
        device_id: String,
        state: PlaybackState,
    },

    /// Seek position changed
    SeekUpdate {
        device_id: String,
        position_ms: u64,
        timestamp: i64,
    },

    /// Queue state changed
    QueueUpdate {
        device_id: String,
        state: QueueState,
    },

    /// Device connected
    DeviceConnected { presence: DevicePresence },

    /// Device disconnected
    DeviceDisconnected { device_id: String },

    /// Active device changed
    ActiveDeviceChanged {
        previous_device_id: Option<String>,
        new_device_id: String,
    },

    /// Playback transfer requested
    TransferRequest {
        from_device_id: String,
        to_device_id: String,
    },

    /// Playback transfer accepted
    TransferAccept {
        from_device_id: String,
        to_device_id: String,
    },

    /// Settings updated
    SettingsUpdate {
        device_id: String,
        settings: SyncedSettings,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::PlaybackStateUpdate(PlaybackState {
            track_id: Some("track-123".into()),
            is_playing: true,
            position_ms: 45000,
            timestamp: 1234567890,
            volume: 0.75,
            is_muted: false,
            shuffle: false,
            repeat: RepeatMode::Off,
        });

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("PlaybackStateUpdate"));
        assert!(json.contains("track-123"));

        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ClientMessage::PlaybackStateUpdate(_)));
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Connected(ConnectedPayload {
            device_id: "device-1".into(),
            session_id: Uuid::nil(),
            active_device_id: None,
        });

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Connected"));
        assert!(json.contains("device-1"));

        let parsed: ServerMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ServerMessage::Connected(_)));
    }

    #[test]
    fn test_repeat_mode_serialization() {
        assert_eq!(serde_json::to_string(&RepeatMode::Off).unwrap(), "\"off\"");
        assert_eq!(
            serde_json::to_string(&RepeatMode::Track).unwrap(),
            "\"track\""
        );
        assert_eq!(
            serde_json::to_string(&RepeatMode::Queue).unwrap(),
            "\"queue\""
        );
    }

    #[test]
    fn test_device_type_display() {
        assert_eq!(DeviceType::Web.to_string(), "web");
        assert_eq!(DeviceType::Desktop.to_string(), "desktop");
        assert_eq!(DeviceType::Mobile.to_string(), "mobile");
    }

    #[test]
    fn test_device_type_from_str() {
        assert_eq!("web".parse::<DeviceType>().unwrap(), DeviceType::Web);
        assert_eq!(
            "DESKTOP".parse::<DeviceType>().unwrap(),
            DeviceType::Desktop
        );
        assert_eq!(
            "invalid".parse::<DeviceType>().unwrap(),
            DeviceType::Unknown
        );
    }

    #[test]
    fn test_error_payload_constructors() {
        let auth = ErrorPayload::auth_failed("Token expired");
        assert_eq!(auth.code, "AUTH_FAILED");

        let rate = ErrorPayload::rate_limited(60);
        assert_eq!(rate.code, "RATE_LIMITED");
        assert!(rate.message.contains("60"));
    }

    #[test]
    fn test_sync_event_serialization() {
        let event = SyncEvent::PlaybackUpdate {
            device_id: "device-1".into(),
            state: PlaybackState::default(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("PlaybackUpdate"));

        let parsed: SyncEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, SyncEvent::PlaybackUpdate { .. }));
    }
}
