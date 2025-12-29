//! Playback synchronization logic
//!
//! This module handles routing and processing of sync messages
//! between devices, including playback state, seek, queue, and
//! device transfer operations.

use uuid::Uuid;

use super::connection::ConnectionManager;
use super::messages::{
    ClientMessage, DevicePresence, ErrorPayload, PlaybackState, QueueState, ServerMessage,
    SyncEvent, SyncedSettings,
};
use super::pubsub::SyncPubSub;

/// Handles synchronization messages for a single device connection
pub struct SyncHandler {
    user_id: Uuid,
    device_id: String,
    connection_manager: ConnectionManager,
    pubsub: SyncPubSub,
}

impl SyncHandler {
    /// Create a new sync handler for a device
    pub fn new(
        user_id: Uuid,
        device_id: String,
        connection_manager: ConnectionManager,
        pubsub: SyncPubSub,
    ) -> Self {
        Self {
            user_id,
            device_id,
            connection_manager,
            pubsub,
        }
    }

    /// Handle an incoming client message
    pub async fn handle_message(&self, message: ClientMessage) -> Result<(), SyncError> {
        match message {
            ClientMessage::PlaybackStateUpdate(state) => self.handle_playback_update(state).await,
            ClientMessage::Seek { position_ms } => self.handle_seek(position_ms).await,
            ClientMessage::QueueUpdate(queue) => self.handle_queue_update(queue).await,
            ClientMessage::TransferPlayback { target_device_id } => {
                self.handle_transfer_request(target_device_id).await
            }
            ClientMessage::RequestDeviceList => self.handle_device_list_request().await,
            ClientMessage::Heartbeat => self.handle_heartbeat().await,
            ClientMessage::SettingsUpdate(settings) => self.handle_settings_update(settings).await,
        }
    }

    /// Handle playback state update from active device
    async fn handle_playback_update(&self, state: PlaybackState) -> Result<(), SyncError> {
        // Check if this device is the active device
        if !self.is_active_device() {
            // Non-active device sent playback update - reject
            self.send_error(ErrorPayload::not_active_device());
            return Ok(());
        }

        // Update stored playback state
        self.connection_manager
            .set_playback_state(self.user_id, state.clone());

        // Broadcast to other devices via pub/sub
        let event = SyncEvent::PlaybackUpdate {
            device_id: self.device_id.clone(),
            state,
        };
        self.pubsub.publish(self.user_id, event).await;

        Ok(())
    }

    /// Handle seek from active device
    async fn handle_seek(&self, position_ms: u64) -> Result<(), SyncError> {
        // Check if this device is the active device
        if !self.is_active_device() {
            self.send_error(ErrorPayload::not_active_device());
            return Ok(());
        }

        let timestamp = chrono::Utc::now().timestamp_millis();

        // Broadcast seek to other devices
        let event = SyncEvent::SeekUpdate {
            device_id: self.device_id.clone(),
            position_ms,
            timestamp,
        };
        self.pubsub.publish(self.user_id, event).await;

        Ok(())
    }

    /// Handle queue update from active device
    async fn handle_queue_update(&self, queue: QueueState) -> Result<(), SyncError> {
        // Check if this device is the active device
        if !self.is_active_device() {
            self.send_error(ErrorPayload::not_active_device());
            return Ok(());
        }

        // Broadcast queue to other devices
        let event = SyncEvent::QueueUpdate {
            device_id: self.device_id.clone(),
            state: queue,
        };
        self.pubsub.publish(self.user_id, event).await;

        Ok(())
    }

    /// Handle playback transfer request
    ///
    /// # Authorization Model
    ///
    /// This follows the Spotify Connect explicit transfer model:
    /// - When an active device exists, only that device can transfer control
    /// - When no active device exists (e.g., fresh session), any device can claim control
    /// - This prevents "playback hijacking" where a non-controlling device takes over
    ///
    /// The flow is:
    /// 1. Device A becomes active (first to play or via transfer)
    /// 2. Device B wants control -> must request transfer from Device A
    /// 3. Device A (or server on its behalf) approves and sets Device B as active
    async fn handle_transfer_request(&self, target_device_id: String) -> Result<(), SyncError> {
        // Authorization: Only the active device (or any device when no active device exists)
        // can initiate a transfer. This prevents non-controlling devices from hijacking playback.
        let current_active = self.connection_manager.get_active_device(self.user_id);
        if current_active.is_some() && !self.is_active_device() {
            self.send_error(ErrorPayload::new(
                "NOT_AUTHORIZED",
                "Only the active device can transfer playback",
            ));
            return Ok(());
        }

        // Prevent transferring to self when already active (no-op)
        // But allow "claiming" control when no active device exists
        if self.device_id == target_device_id && current_active.is_some() {
            self.send_error(ErrorPayload::new(
                "INVALID_TARGET",
                "Already the active device",
            ));
            return Ok(());
        }

        // Verify target device exists
        if !self
            .connection_manager
            .device_exists(self.user_id, &target_device_id)
        {
            self.send_error(ErrorPayload::device_not_found(&target_device_id));
            return Ok(());
        }

        // Get current active device for the transfer event
        let previous_device_id = current_active;

        // Set new active device
        self.connection_manager
            .set_active_device(self.user_id, &target_device_id);

        // Broadcast transfer request to target device
        let request_event = SyncEvent::TransferRequest {
            from_device_id: self.device_id.clone(),
            to_device_id: target_device_id.clone(),
        };
        self.pubsub.publish(self.user_id, request_event).await;

        // Broadcast active device change
        let change_event = SyncEvent::ActiveDeviceChanged {
            previous_device_id,
            new_device_id: target_device_id.clone(),
        };
        self.pubsub.publish(self.user_id, change_event).await;

        // Send transfer accepted confirmation
        let accept_event = SyncEvent::TransferAccept {
            from_device_id: self.device_id.clone(),
            to_device_id: target_device_id,
        };
        self.pubsub.publish(self.user_id, accept_event).await;

        Ok(())
    }

    /// Handle device list request
    async fn handle_device_list_request(&self) -> Result<(), SyncError> {
        let devices = self.connection_manager.get_device_list(self.user_id);
        let msg = ServerMessage::DeviceList(devices);
        self.send_to_self(msg);
        Ok(())
    }

    /// Handle heartbeat
    async fn handle_heartbeat(&self) -> Result<(), SyncError> {
        let server_time = chrono::Utc::now().timestamp_millis();
        let msg = ServerMessage::Pong { server_time };
        self.send_to_self(msg);
        Ok(())
    }

    /// Handle settings update
    async fn handle_settings_update(&self, settings: SyncedSettings) -> Result<(), SyncError> {
        // Broadcast settings to other devices
        let event = SyncEvent::SettingsUpdate {
            device_id: self.device_id.clone(),
            settings,
        };
        self.pubsub.publish(self.user_id, event).await;
        Ok(())
    }

    /// Handle a device connection event
    pub async fn handle_device_connected(&self, device_info: super::connection::DeviceInfo) {
        let presence = DevicePresence {
            device_id: device_info.device_id.clone(),
            device_name: device_info.device_name,
            device_type: device_info.device_type,
            is_active: self.connection_manager.get_active_device(self.user_id)
                == Some(device_info.device_id.clone()),
            current_track: None,
            volume: 1.0,
            last_seen: chrono::Utc::now().timestamp_millis(),
        };

        let event = SyncEvent::DeviceConnected { presence };
        self.pubsub.publish(self.user_id, event).await;
    }

    /// Handle a device disconnection event
    pub async fn handle_device_disconnected(&self) {
        let event = SyncEvent::DeviceDisconnected {
            device_id: self.device_id.clone(),
        };
        self.pubsub.publish(self.user_id, event).await;

        // If this was the active device, clear active status
        if self.is_active_device() {
            self.connection_manager.clear_active_device(self.user_id);
        }
    }

    /// Check if this device is the active device
    fn is_active_device(&self) -> bool {
        self.connection_manager.get_active_device(self.user_id) == Some(self.device_id.clone())
    }

    /// Send a message to this device
    fn send_to_self(&self, msg: ServerMessage) {
        if let Err(e) = self
            .connection_manager
            .send_to_device(self.user_id, &self.device_id, msg)
        {
            tracing::debug!(
                user_id = %self.user_id,
                device_id = %self.device_id,
                error = %e,
                "Failed to send message to self"
            );
        }
    }

    /// Send an error to this device
    fn send_error(&self, error: ErrorPayload) {
        self.send_to_self(ServerMessage::Error(error));
    }

    /// Send a message to a specific device
    pub fn send_to_device(&self, device_id: &str, msg: ServerMessage) {
        if let Err(e) = self
            .connection_manager
            .send_to_device(self.user_id, device_id, msg)
        {
            tracing::debug!(
                user_id = %self.user_id,
                device_id = %device_id,
                error = %e,
                "Failed to send message to device"
            );
        }
    }
}

/// Convert a SyncEvent to a ServerMessage for a specific device
///
/// Returns None if the message should not be sent to this device
/// (e.g., the device is the source of the event)
pub fn sync_event_to_server_message(
    event: &SyncEvent,
    receiving_device_id: &str,
) -> Option<ServerMessage> {
    match event {
        SyncEvent::PlaybackUpdate { device_id, state } => {
            // Don't send back to the device that sent the update
            if device_id == receiving_device_id {
                return None;
            }
            Some(ServerMessage::PlaybackSync(state.clone()))
        }
        SyncEvent::SeekUpdate {
            device_id,
            position_ms,
            timestamp,
        } => {
            if device_id == receiving_device_id {
                return None;
            }
            Some(ServerMessage::SeekSync {
                position_ms: *position_ms,
                timestamp: *timestamp,
            })
        }
        SyncEvent::QueueUpdate { device_id, state } => {
            if device_id == receiving_device_id {
                return None;
            }
            Some(ServerMessage::QueueSync(state.clone()))
        }
        SyncEvent::DeviceConnected { presence } => {
            // Don't notify the device about its own connection
            if presence.device_id == receiving_device_id {
                return None;
            }
            Some(ServerMessage::DeviceConnected(presence.clone()))
        }
        SyncEvent::DeviceDisconnected { device_id } => {
            // Don't notify the disconnecting device
            if device_id == receiving_device_id {
                return None;
            }
            Some(ServerMessage::DeviceDisconnected {
                device_id: device_id.clone(),
            })
        }
        SyncEvent::ActiveDeviceChanged {
            previous_device_id,
            new_device_id,
        } => {
            // Notify all devices about the active device change
            Some(ServerMessage::ActiveDeviceChanged {
                previous_device_id: previous_device_id.clone(),
                new_device_id: new_device_id.clone(),
            })
        }
        SyncEvent::TransferRequest {
            from_device_id,
            to_device_id,
        } => {
            // Only send to the target device
            if receiving_device_id == to_device_id {
                Some(ServerMessage::TransferRequested {
                    from_device_id: from_device_id.clone(),
                })
            } else {
                None
            }
        }
        SyncEvent::TransferAccept { to_device_id, .. } => {
            // Send transfer accepted to all devices
            Some(ServerMessage::TransferAccepted {
                to_device_id: to_device_id.clone(),
            })
        }
        SyncEvent::SettingsUpdate {
            device_id,
            settings,
        } => {
            if device_id == receiving_device_id {
                return None;
            }
            Some(ServerMessage::SettingsSync(settings.clone()))
        }
    }
}

/// Errors that can occur during sync operations
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("not authorized")]
    NotAuthorized,

    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_event_to_server_message_filters_source() {
        let event = SyncEvent::PlaybackUpdate {
            device_id: "device-1".to_string(),
            state: PlaybackState::default(),
        };

        // Should not send back to source device
        let result = sync_event_to_server_message(&event, "device-1");
        assert!(result.is_none());

        // Should send to other devices
        let result = sync_event_to_server_message(&event, "device-2");
        assert!(result.is_some());
        assert!(matches!(result, Some(ServerMessage::PlaybackSync(_))));
    }

    #[test]
    fn test_sync_event_transfer_request_routing() {
        let event = SyncEvent::TransferRequest {
            from_device_id: "device-1".to_string(),
            to_device_id: "device-2".to_string(),
        };

        // Should only send to target device
        let result = sync_event_to_server_message(&event, "device-2");
        assert!(result.is_some());
        assert!(matches!(
            result,
            Some(ServerMessage::TransferRequested { .. })
        ));

        // Should not send to source device
        let result = sync_event_to_server_message(&event, "device-1");
        assert!(result.is_none());

        // Should not send to other devices
        let result = sync_event_to_server_message(&event, "device-3");
        assert!(result.is_none());
    }

    #[test]
    fn test_sync_event_seek_update() {
        let event = SyncEvent::SeekUpdate {
            device_id: "device-1".to_string(),
            position_ms: 45000,
            timestamp: 1234567890,
        };

        let result = sync_event_to_server_message(&event, "device-2");
        assert!(matches!(
            result,
            Some(ServerMessage::SeekSync {
                position_ms: 45000,
                timestamp: 1234567890
            })
        ));
    }
}
