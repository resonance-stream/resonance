//! WebSocket connection management
//!
//! This module handles tracking and managing WebSocket connections
//! across all connected devices for each user.

use dashmap::DashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::messages::{DevicePresence, DeviceType, PlaybackState, ServerMessage};

/// Handle for sending messages to a specific WebSocket connection
#[derive(Debug)]
pub struct ConnectionHandle {
    /// Channel for sending messages to this connection
    pub sender: mpsc::UnboundedSender<ServerMessage>,

    /// Device information
    pub device_info: DeviceInfo,

    /// When this connection was established (Unix timestamp ms)
    pub connected_at: i64,

    /// Last activity timestamp (atomic for thread-safe updates)
    pub last_activity: Arc<AtomicI64>,
}

impl ConnectionHandle {
    pub fn new(sender: mpsc::UnboundedSender<ServerMessage>, device_info: DeviceInfo) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            sender,
            device_info,
            connected_at: now,
            last_activity: Arc::new(AtomicI64::new(now)),
        }
    }

    /// Update last activity timestamp
    pub fn touch(&self) {
        self.last_activity
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// Get last activity timestamp
    pub fn last_seen(&self) -> i64 {
        self.last_activity.load(Ordering::Relaxed)
    }

    /// Send a message to this connection
    #[allow(clippy::result_large_err)]
    pub fn send(&self, msg: ServerMessage) -> Result<(), mpsc::error::SendError<ServerMessage>> {
        self.touch();
        self.sender.send(msg)
    }

    /// Check if the connection is still alive
    pub fn is_alive(&self) -> bool {
        !self.sender.is_closed()
    }
}

/// Device information for a connection
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Unique device identifier (client-provided or generated)
    pub device_id: String,

    /// Human-readable device name
    pub device_name: String,

    /// Type of device
    pub device_type: DeviceType,

    /// User agent string (for debugging)
    pub user_agent: Option<String>,
}

impl DeviceInfo {
    pub fn new(
        device_id: String,
        device_name: Option<String>,
        device_type: Option<String>,
    ) -> Self {
        let device_type = device_type
            .and_then(|t| t.parse().ok())
            .unwrap_or(DeviceType::Unknown);

        let device_name = device_name.unwrap_or_else(|| {
            match device_type {
                DeviceType::Web => "Web Browser",
                DeviceType::Desktop => "Desktop App",
                DeviceType::Mobile => "Mobile Device",
                DeviceType::Tablet => "Tablet",
                DeviceType::Speaker => "Speaker",
                DeviceType::Unknown => "Unknown Device",
            }
            .to_string()
        });

        Self {
            device_id,
            device_name,
            device_type,
            user_agent: None,
        }
    }

    pub fn with_user_agent(mut self, user_agent: Option<String>) -> Self {
        self.user_agent = user_agent;
        self
    }
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self {
            device_id: Uuid::new_v4().to_string(),
            device_name: "Unknown Device".to_string(),
            device_type: DeviceType::Unknown,
            user_agent: None,
        }
    }
}

/// State for a single user's connections
#[derive(Debug, Default)]
pub struct UserConnectionState {
    /// Map of device_id -> ConnectionHandle
    pub connections: DashMap<String, ConnectionHandle>,

    /// Currently active device (controlling playback)
    pub active_device_id: Option<String>,

    /// Current playback state
    pub playback_state: Option<PlaybackState>,
}

impl UserConnectionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the number of connected devices
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Check if a device is connected
    pub fn is_connected(&self, device_id: &str) -> bool {
        self.connections.contains_key(device_id)
    }

    /// Get all device presences
    pub fn get_device_presences(&self) -> Vec<DevicePresence> {
        self.connections
            .iter()
            .map(|entry| {
                let handle = entry.value();
                DevicePresence {
                    device_id: handle.device_info.device_id.clone(),
                    device_name: handle.device_info.device_name.clone(),
                    device_type: handle.device_info.device_type,
                    is_active: self.active_device_id.as_ref()
                        == Some(&handle.device_info.device_id),
                    current_track: self.playback_state.as_ref().and_then(|ps| {
                        ps.track_id
                            .as_ref()
                            .map(|id| super::messages::TrackSummary {
                                id: id.clone(),
                                title: String::new(), // Would need to be filled from DB
                                artist: String::new(),
                            })
                    }),
                    volume: self
                        .playback_state
                        .as_ref()
                        .map(|ps| ps.volume)
                        .unwrap_or(1.0),
                    last_seen: handle.last_seen(),
                }
            })
            .collect()
    }
}

/// Manages WebSocket connections for all users
///
/// Thread-safe structure for tracking connections across the application.
/// Uses DashMap for concurrent access without explicit locking.
/// Wrapped in Arc for cheap cloning.
#[derive(Debug, Clone, Default)]
pub struct ConnectionManager {
    /// Map of user_id -> UserConnectionState
    users: Arc<DashMap<Uuid, UserConnectionState>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            users: Arc::new(DashMap::new()),
        }
    }

    /// Add a new connection for a user
    pub fn add_connection(
        &self,
        user_id: Uuid,
        device_id: String,
        sender: mpsc::UnboundedSender<ServerMessage>,
        device_info: DeviceInfo,
    ) {
        let handle = ConnectionHandle::new(sender, device_info);

        self.users
            .entry(user_id)
            .or_default()
            .connections
            .insert(device_id, handle);

        tracing::debug!(
            user_id = %user_id,
            device_count = self.users.get(&user_id).map(|s| s.connection_count()).unwrap_or(0),
            "Connection added"
        );
    }

    /// Remove a connection
    pub fn remove_connection(&self, user_id: Uuid, device_id: &str) -> bool {
        if let Some(mut user_state) = self.users.get_mut(&user_id) {
            let removed = user_state.connections.remove(device_id).is_some();

            // If this was the active device, clear it
            if user_state.active_device_id.as_deref() == Some(device_id) {
                user_state.active_device_id = None;
            }

            // Clean up empty user entries
            let is_empty = user_state.connections.is_empty();
            drop(user_state);

            if is_empty {
                self.users.remove(&user_id);
            }

            if removed {
                tracing::debug!(
                    user_id = %user_id,
                    device_id = %device_id,
                    "Connection removed"
                );
            }

            removed
        } else {
            false
        }
    }

    /// Check if a user has any connections
    pub fn has_connections(&self, user_id: Uuid) -> bool {
        self.users
            .get(&user_id)
            .map(|s| !s.connections.is_empty())
            .unwrap_or(false)
    }

    /// Get the number of connections for a user
    pub fn connection_count(&self, user_id: Uuid) -> usize {
        self.users
            .get(&user_id)
            .map(|s| s.connection_count())
            .unwrap_or(0)
    }

    /// Get all device presences for a user
    pub fn get_device_presences(&self, user_id: Uuid) -> Vec<DevicePresence> {
        self.users
            .get(&user_id)
            .map(|s| s.get_device_presences())
            .unwrap_or_default()
    }

    /// Get the active device ID for a user
    pub fn get_active_device(&self, user_id: Uuid) -> Option<String> {
        self.users
            .get(&user_id)
            .and_then(|s| s.active_device_id.clone())
    }

    /// Set the active device for a user
    pub fn set_active_device(&self, user_id: Uuid, device_id: &str) {
        if let Some(mut user_state) = self.users.get_mut(&user_id) {
            user_state.active_device_id = Some(device_id.to_string());
        }
    }

    /// Clear the active device for a user
    pub fn clear_active_device(&self, user_id: Uuid) {
        if let Some(mut user_state) = self.users.get_mut(&user_id) {
            user_state.active_device_id = None;
        }
    }

    /// Check if a device exists for a user
    pub fn device_exists(&self, user_id: Uuid, device_id: &str) -> bool {
        self.users
            .get(&user_id)
            .map(|s| s.connections.contains_key(device_id))
            .unwrap_or(false)
    }

    /// Get all device presences for a user (alias for get_device_presences)
    pub fn get_device_list(&self, user_id: Uuid) -> Vec<DevicePresence> {
        self.get_device_presences(user_id)
    }

    /// Set playback state for a user
    pub fn set_playback_state(&self, user_id: Uuid, state: PlaybackState) {
        if let Some(mut user_state) = self.users.get_mut(&user_id) {
            user_state.playback_state = Some(state);
        }
    }

    /// Update playback state for a user
    pub fn update_playback_state(&self, user_id: Uuid, state: PlaybackState) {
        if let Some(mut user_state) = self.users.get_mut(&user_id) {
            user_state.playback_state = Some(state);
        }
    }

    /// Get current playback state for a user
    pub fn get_playback_state(&self, user_id: Uuid) -> Option<PlaybackState> {
        self.users
            .get(&user_id)
            .and_then(|s| s.playback_state.clone())
    }

    /// Send a message to a specific device
    pub fn send_to_device(
        &self,
        user_id: Uuid,
        device_id: &str,
        msg: ServerMessage,
    ) -> Result<(), SendError> {
        let user_state = self.users.get(&user_id).ok_or(SendError::UserNotFound)?;

        let handle = user_state
            .connections
            .get(device_id)
            .ok_or(SendError::DeviceNotFound)?;

        handle.send(msg).map_err(|_| SendError::ConnectionClosed)?;

        Ok(())
    }

    /// Update last activity timestamp for a device (call when receiving messages)
    ///
    /// Returns true if the device was found and updated, false otherwise.
    pub fn touch_device(&self, user_id: Uuid, device_id: &str) -> bool {
        if let Some(user_state) = self.users.get(&user_id) {
            if let Some(handle) = user_state.connections.get(device_id) {
                handle.touch();
                return true;
            }
        }
        false
    }

    /// Send a message to all devices for a user
    pub fn broadcast_to_user(&self, user_id: Uuid, msg: ServerMessage) -> usize {
        let user_state = match self.users.get(&user_id) {
            Some(s) => s,
            None => return 0,
        };

        let mut sent = 0;
        for entry in user_state.connections.iter() {
            if entry.value().send(msg.clone()).is_ok() {
                sent += 1;
            }
        }

        sent
    }

    /// Send a message to all devices except the sender
    pub fn broadcast_to_others(
        &self,
        user_id: Uuid,
        sender_device_id: &str,
        msg: ServerMessage,
    ) -> usize {
        let user_state = match self.users.get(&user_id) {
            Some(s) => s,
            None => return 0,
        };

        let mut sent = 0;
        for entry in user_state.connections.iter() {
            if entry.key() != sender_device_id && entry.value().send(msg.clone()).is_ok() {
                sent += 1;
            }
        }

        sent
    }

    /// Get total number of connections across all users
    pub fn total_connections(&self) -> usize {
        self.users.iter().map(|e| e.connection_count()).sum()
    }

    /// Get number of connected users
    pub fn total_users(&self) -> usize {
        self.users.len()
    }

    /// Clean up stale connections (connections that haven't been active)
    pub fn cleanup_stale_connections(&self, max_idle_ms: i64) -> usize {
        let now = chrono::Utc::now().timestamp_millis();
        let mut removed = 0;

        for user_entry in self.users.iter_mut() {
            let user_id = *user_entry.key();
            let stale_devices: Vec<String> = user_entry
                .connections
                .iter()
                .filter(|e| {
                    let idle_time = now - e.value().last_seen();
                    idle_time > max_idle_ms || !e.value().is_alive()
                })
                .map(|e| e.key().clone())
                .collect();

            for device_id in stale_devices {
                if user_entry.connections.remove(&device_id).is_some() {
                    removed += 1;
                    tracing::debug!(
                        user_id = %user_id,
                        device_id = %device_id,
                        "Removed stale connection"
                    );
                }
            }
        }

        // Clean up empty user entries
        self.users.retain(|_, state| !state.connections.is_empty());

        removed
    }
}

/// Error type for send operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    UserNotFound,
    DeviceNotFound,
    ConnectionClosed,
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::UserNotFound => write!(f, "user not found"),
            SendError::DeviceNotFound => write!(f, "device not found"),
            SendError::ConnectionClosed => write!(f, "connection closed"),
        }
    }
}

impl std::error::Error for SendError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::messages::RepeatMode;

    #[test]
    fn test_connection_manager_add_remove() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();
        let device_id = "device-1".to_string();

        let (tx, _rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, device_id.clone(), tx, DeviceInfo::default());

        assert!(manager.has_connections(user_id));
        assert_eq!(manager.connection_count(user_id), 1);

        manager.remove_connection(user_id, &device_id);

        assert!(!manager.has_connections(user_id));
        assert_eq!(manager.connection_count(user_id), 0);
    }

    #[test]
    fn test_connection_manager_multiple_devices() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        manager.add_connection(user_id, "device-1".to_string(), tx1, DeviceInfo::default());
        manager.add_connection(user_id, "device-2".to_string(), tx2, DeviceInfo::default());

        assert_eq!(manager.connection_count(user_id), 2);

        manager.remove_connection(user_id, "device-1");
        assert_eq!(manager.connection_count(user_id), 1);
    }

    #[test]
    fn test_connection_manager_active_device() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx, _rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, "device-1".to_string(), tx, DeviceInfo::default());

        assert!(manager.get_active_device(user_id).is_none());

        manager.set_active_device(user_id, "device-1");
        assert_eq!(
            manager.get_active_device(user_id),
            Some("device-1".to_string())
        );
    }

    #[test]
    fn test_connection_manager_broadcast() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        manager.add_connection(user_id, "device-1".to_string(), tx1, DeviceInfo::default());
        manager.add_connection(user_id, "device-2".to_string(), tx2, DeviceInfo::default());

        let msg = ServerMessage::Pong { server_time: 123 };
        let sent = manager.broadcast_to_user(user_id, msg);

        assert_eq!(sent, 2);
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_connection_manager_broadcast_to_others() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        manager.add_connection(user_id, "device-1".to_string(), tx1, DeviceInfo::default());
        manager.add_connection(user_id, "device-2".to_string(), tx2, DeviceInfo::default());

        let msg = ServerMessage::Pong { server_time: 123 };
        let sent = manager.broadcast_to_others(user_id, "device-1", msg);

        assert_eq!(sent, 1);
        assert!(rx1.try_recv().is_err()); // device-1 should not receive
        assert!(rx2.try_recv().is_ok()); // device-2 should receive
    }

    #[test]
    fn test_device_info_construction() {
        let info = DeviceInfo::new(
            "test-id".to_string(),
            Some("My Device".to_string()),
            Some("desktop".to_string()),
        );

        assert_eq!(info.device_id, "test-id");
        assert_eq!(info.device_name, "My Device");
        assert_eq!(info.device_type, DeviceType::Desktop);
    }

    #[test]
    fn test_device_info_defaults() {
        let info = DeviceInfo::new("test-id".to_string(), None, None);

        assert_eq!(info.device_id, "test-id");
        assert_eq!(info.device_name, "Unknown Device");
        assert_eq!(info.device_type, DeviceType::Unknown);
    }

    #[test]
    fn test_connection_manager_remove_clears_active_device() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();
        let device_id = "active-device".to_string();

        let (tx, _rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, device_id.clone(), tx, DeviceInfo::default());

        // Set this device as active
        manager.set_active_device(user_id, &device_id);
        assert_eq!(
            manager.get_active_device(user_id),
            Some(device_id.clone())
        );

        // Remove the active device
        let removed = manager.remove_connection(user_id, &device_id);
        assert!(removed);

        // Active device should be cleared
        assert!(manager.get_active_device(user_id).is_none());

        // User entry should be cleaned up since no connections remain
        assert!(!manager.has_connections(user_id));
    }

    #[test]
    fn test_connection_manager_remove_preserves_active_device_for_other_devices() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        manager.add_connection(user_id, "device-1".to_string(), tx1, DeviceInfo::default());
        manager.add_connection(user_id, "device-2".to_string(), tx2, DeviceInfo::default());

        // Set device-1 as active
        manager.set_active_device(user_id, "device-1");

        // Remove device-2 (not the active device)
        manager.remove_connection(user_id, "device-2");

        // Active device should still be device-1
        assert_eq!(
            manager.get_active_device(user_id),
            Some("device-1".to_string())
        );
    }

    #[test]
    fn test_connection_manager_cleanup_stale_connections() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx1, rx1) = mpsc::unbounded_channel();
        let (tx2, rx2) = mpsc::unbounded_channel();

        manager.add_connection(user_id, "device-1".to_string(), tx1, DeviceInfo::default());
        manager.add_connection(user_id, "device-2".to_string(), tx2, DeviceInfo::default());

        assert_eq!(manager.connection_count(user_id), 2);

        // Drop receivers to close the connections - closed connections should be removed
        // regardless of idle time
        drop(rx1);
        drop(rx2);

        // Cleanup should remove closed connections (is_alive() returns false)
        let removed = manager.cleanup_stale_connections(1_000_000_000);
        assert_eq!(removed, 2);

        // User entry should be cleaned up
        assert!(!manager.has_connections(user_id));
        assert_eq!(manager.total_users(), 0);
    }

    #[test]
    fn test_connection_manager_cleanup_stale_preserves_active_connections() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        manager.add_connection(user_id, "device-1".to_string(), tx1, DeviceInfo::default());
        manager.add_connection(user_id, "device-2".to_string(), tx2, DeviceInfo::default());

        // Touch device-1 to update its timestamp
        manager.touch_device(user_id, "device-1");

        // Use a large idle time that won't expire fresh connections
        let removed = manager.cleanup_stale_connections(1_000_000_000); // ~11.5 days
        assert_eq!(removed, 0);

        assert_eq!(manager.connection_count(user_id), 2);
    }

    #[test]
    fn test_connection_manager_cleanup_removes_closed_connections() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx1, rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        manager.add_connection(user_id, "device-1".to_string(), tx1, DeviceInfo::default());
        manager.add_connection(user_id, "device-2".to_string(), tx2, DeviceInfo::default());

        // Drop the receiver to close device-1's connection
        drop(rx1);

        // Even with a large idle time, closed connections should be removed
        let removed = manager.cleanup_stale_connections(1_000_000_000);
        assert_eq!(removed, 1);

        assert_eq!(manager.connection_count(user_id), 1);
        assert!(manager.device_exists(user_id, "device-2"));
        assert!(!manager.device_exists(user_id, "device-1"));
    }

    #[test]
    fn test_connection_manager_send_to_device_user_not_found() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let result = manager.send_to_device(user_id, "device-1", ServerMessage::Pong { server_time: 123 });

        assert_eq!(result, Err(SendError::UserNotFound));
    }

    #[test]
    fn test_connection_manager_send_to_device_device_not_found() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx, _rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, "device-1".to_string(), tx, DeviceInfo::default());

        let result = manager.send_to_device(
            user_id,
            "nonexistent-device",
            ServerMessage::Pong { server_time: 123 },
        );

        assert_eq!(result, Err(SendError::DeviceNotFound));
    }

    #[test]
    fn test_connection_manager_send_to_device_connection_closed() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx, rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, "device-1".to_string(), tx, DeviceInfo::default());

        // Drop the receiver to close the connection
        drop(rx);

        let result = manager.send_to_device(user_id, "device-1", ServerMessage::Pong { server_time: 123 });

        assert_eq!(result, Err(SendError::ConnectionClosed));
    }

    #[test]
    fn test_connection_manager_send_to_device_success() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, "device-1".to_string(), tx, DeviceInfo::default());

        let result = manager.send_to_device(user_id, "device-1", ServerMessage::Pong { server_time: 456 });

        assert!(result.is_ok());

        // Verify message was received
        let received = rx.try_recv().unwrap();
        assert!(matches!(received, ServerMessage::Pong { server_time: 456 }));
    }

    #[test]
    fn test_connection_manager_playback_state_crud() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx, _rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, "device-1".to_string(), tx, DeviceInfo::default());

        // Initially no playback state
        assert!(manager.get_playback_state(user_id).is_none());

        // Create playback state
        let state = PlaybackState {
            track_id: Some("track-123".to_string()),
            is_playing: true,
            position_ms: 5000,
            timestamp: chrono::Utc::now().timestamp_millis(),
            volume: 0.8,
            is_muted: false,
            shuffle: true,
            repeat: RepeatMode::Queue,
        };

        // Set playback state
        manager.set_playback_state(user_id, state.clone());

        // Read playback state
        let retrieved = manager.get_playback_state(user_id).unwrap();
        assert_eq!(retrieved.track_id, Some("track-123".to_string()));
        assert!(retrieved.is_playing);
        assert_eq!(retrieved.position_ms, 5000);
        assert_eq!(retrieved.volume, 0.8);
        assert!(retrieved.shuffle);
        assert_eq!(retrieved.repeat, RepeatMode::Queue);

        // Update playback state
        let updated_state = PlaybackState {
            track_id: Some("track-456".to_string()),
            is_playing: false,
            position_ms: 10000,
            timestamp: chrono::Utc::now().timestamp_millis(),
            volume: 0.5,
            is_muted: true,
            shuffle: false,
            repeat: RepeatMode::Track,
        };

        manager.update_playback_state(user_id, updated_state);

        let retrieved = manager.get_playback_state(user_id).unwrap();
        assert_eq!(retrieved.track_id, Some("track-456".to_string()));
        assert!(!retrieved.is_playing);
        assert_eq!(retrieved.position_ms, 10000);
        assert_eq!(retrieved.volume, 0.5);
        assert!(retrieved.is_muted);
        assert!(!retrieved.shuffle);
        assert_eq!(retrieved.repeat, RepeatMode::Track);
    }

    #[test]
    fn test_connection_manager_playback_state_no_user() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        // Trying to get/set playback state for non-existent user should not panic
        assert!(manager.get_playback_state(user_id).is_none());

        // Setting playback state for non-existent user is a no-op (no panic)
        let state = PlaybackState::default();
        manager.set_playback_state(user_id, state.clone());

        // Still no state because user doesn't exist
        assert!(manager.get_playback_state(user_id).is_none());
    }

    #[test]
    fn test_connection_handle_touch_updates_timestamp() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let handle = ConnectionHandle::new(tx, DeviceInfo::default());

        let initial_timestamp = handle.last_seen();

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(5));

        handle.touch();

        let updated_timestamp = handle.last_seen();
        assert!(
            updated_timestamp > initial_timestamp,
            "Timestamp should increase after touch: {} vs {}",
            updated_timestamp,
            initial_timestamp
        );
    }

    #[test]
    fn test_connection_handle_send_updates_timestamp() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let handle = ConnectionHandle::new(tx, DeviceInfo::default());

        let initial_timestamp = handle.last_seen();

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(5));

        let result = handle.send(ServerMessage::Pong { server_time: 123 });
        assert!(result.is_ok());

        let updated_timestamp = handle.last_seen();
        assert!(
            updated_timestamp > initial_timestamp,
            "Timestamp should increase after send: {} vs {}",
            updated_timestamp,
            initial_timestamp
        );

        // Verify message was sent
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn test_connection_handle_is_alive_when_receiver_active() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let handle = ConnectionHandle::new(tx, DeviceInfo::default());

        // Connection should be alive when receiver is active
        assert!(handle.is_alive());
    }

    #[test]
    fn test_connection_handle_is_alive_when_receiver_dropped() {
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = ConnectionHandle::new(tx, DeviceInfo::default());

        assert!(handle.is_alive());

        // Drop the receiver
        drop(rx);

        // Connection should now be dead
        assert!(!handle.is_alive());
    }

    #[test]
    fn test_connection_handle_send_fails_when_closed() {
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = ConnectionHandle::new(tx, DeviceInfo::default());

        // Drop the receiver to close the channel
        drop(rx);

        let result = handle.send(ServerMessage::Pong { server_time: 123 });
        assert!(result.is_err());
    }

    #[test]
    fn test_send_error_display() {
        assert_eq!(SendError::UserNotFound.to_string(), "user not found");
        assert_eq!(SendError::DeviceNotFound.to_string(), "device not found");
        assert_eq!(SendError::ConnectionClosed.to_string(), "connection closed");
    }

    #[test]
    fn test_touch_device_returns_true_for_existing_device() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx, _rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, "device-1".to_string(), tx, DeviceInfo::default());

        assert!(manager.touch_device(user_id, "device-1"));
    }

    #[test]
    fn test_touch_device_returns_false_for_nonexistent_user() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        assert!(!manager.touch_device(user_id, "device-1"));
    }

    #[test]
    fn test_touch_device_returns_false_for_nonexistent_device() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx, _rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, "device-1".to_string(), tx, DeviceInfo::default());

        assert!(!manager.touch_device(user_id, "nonexistent-device"));
    }

    #[test]
    fn test_device_info_with_user_agent() {
        let info = DeviceInfo::new(
            "test-id".to_string(),
            Some("My Device".to_string()),
            Some("web".to_string()),
        )
        .with_user_agent(Some("Mozilla/5.0".to_string()));

        assert_eq!(info.user_agent, Some("Mozilla/5.0".to_string()));
    }

    #[test]
    fn test_user_connection_state_get_device_presences() {
        let user_state = UserConnectionState::new();

        let (tx, _rx) = mpsc::unbounded_channel();
        let device_info = DeviceInfo::new(
            "device-1".to_string(),
            Some("My Phone".to_string()),
            Some("mobile".to_string()),
        );
        let handle = ConnectionHandle::new(tx, device_info);
        user_state.connections.insert("device-1".to_string(), handle);

        let presences = user_state.get_device_presences();
        assert_eq!(presences.len(), 1);
        assert_eq!(presences[0].device_id, "device-1");
        assert_eq!(presences[0].device_name, "My Phone");
        assert_eq!(presences[0].device_type, DeviceType::Mobile);
        assert!(!presences[0].is_active);
    }

    #[test]
    fn test_user_connection_state_get_device_presences_with_active_device() {
        let mut user_state = UserConnectionState::new();
        user_state.active_device_id = Some("device-1".to_string());

        let (tx, _rx) = mpsc::unbounded_channel();
        let device_info = DeviceInfo::new(
            "device-1".to_string(),
            Some("My Phone".to_string()),
            Some("mobile".to_string()),
        );
        let handle = ConnectionHandle::new(tx, device_info);
        user_state.connections.insert("device-1".to_string(), handle);

        let presences = user_state.get_device_presences();
        assert_eq!(presences.len(), 1);
        assert!(presences[0].is_active);
    }

    #[test]
    fn test_total_connections_and_users() {
        let manager = ConnectionManager::new();

        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        assert_eq!(manager.total_connections(), 0);
        assert_eq!(manager.total_users(), 0);

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();

        manager.add_connection(user1, "device-1".to_string(), tx1, DeviceInfo::default());
        manager.add_connection(user1, "device-2".to_string(), tx2, DeviceInfo::default());
        manager.add_connection(user2, "device-3".to_string(), tx3, DeviceInfo::default());

        assert_eq!(manager.total_connections(), 3);
        assert_eq!(manager.total_users(), 2);
    }

    #[test]
    fn test_clear_active_device() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx, _rx) = mpsc::unbounded_channel();
        manager.add_connection(user_id, "device-1".to_string(), tx, DeviceInfo::default());

        manager.set_active_device(user_id, "device-1");
        assert!(manager.get_active_device(user_id).is_some());

        manager.clear_active_device(user_id);
        assert!(manager.get_active_device(user_id).is_none());
    }

    #[test]
    fn test_get_device_list_alias() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();

        let (tx, _rx) = mpsc::unbounded_channel();
        let device_info = DeviceInfo::new("device-1".to_string(), Some("My Device".to_string()), None);
        manager.add_connection(user_id, "device-1".to_string(), tx, device_info);

        // get_device_list is an alias for get_device_presences
        let presences = manager.get_device_list(user_id);
        assert_eq!(presences.len(), 1);
        assert_eq!(presences[0].device_id, "device-1");
    }
}
