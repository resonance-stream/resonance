//! Playback synchronization logic
//!
//! This module handles routing and processing of sync messages
//! between devices, including playback state, seek, queue, and
//! device transfer operations.

use once_cell::sync::Lazy;
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

use super::connection::ConnectionManager;
use super::messages::{
    ClientMessage, DevicePresence, ErrorPayload, PlaybackState, QueueState, ServerMessage,
    SyncEvent, SyncedSettings,
};
use super::pubsub::SyncPubSub;
use crate::models::queue::SetQueue;
use crate::repositories::QueueRepository;

/// Global semaphore to limit concurrent queue persistence tasks.
/// This prevents database connection pool exhaustion under burst load.
static PERSIST_SEMAPHORE: Lazy<Arc<Semaphore>> = Lazy::new(|| Arc::new(Semaphore::new(50)));

/// Per-user persistence tracking to prevent duplicate concurrent writes.
/// Tracks user IDs that have a persistence task currently running.
static USER_PERSIST_LOCKS: Lazy<Arc<Mutex<HashSet<Uuid>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

/// Handles synchronization messages for a single device connection
pub struct SyncHandler {
    user_id: Uuid,
    device_id: String,
    connection_manager: ConnectionManager,
    pubsub: SyncPubSub,
    pool: Option<PgPool>,
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
            pool: None,
        }
    }

    /// Create a new sync handler with database persistence
    pub fn with_pool(
        user_id: Uuid,
        device_id: String,
        connection_manager: ConnectionManager,
        pubsub: SyncPubSub,
        pool: PgPool,
    ) -> Self {
        Self {
            user_id,
            device_id,
            connection_manager,
            pubsub,
            pool: Some(pool),
        }
    }

    /// Handle an incoming client message
    ///
    /// Note: ChatSend messages are handled separately by the ChatHandler
    /// and should be intercepted before reaching this method.
    pub async fn handle_message(&self, message: ClientMessage) -> Result<(), SyncError> {
        // Update last activity timestamp for this device
        // If the device is not found, the connection is stale - reject the message
        if !self
            .connection_manager
            .touch_device(self.user_id, &self.device_id)
        {
            return Err(SyncError::Internal(
                "device not found for connection".to_string(),
            ));
        }

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
            // ChatSend is handled by ChatHandler, not SyncHandler
            // If it reaches here, something is misconfigured
            ClientMessage::ChatSend(_) => {
                tracing::warn!(
                    user_id = %self.user_id,
                    device_id = %self.device_id,
                    "ChatSend message reached SyncHandler - should be handled by ChatHandler"
                );
                Ok(())
            }
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

        // Persist queue to database (fire-and-forget to avoid blocking WebSocket)
        // Uses semaphore to limit concurrent tasks and per-user lock to prevent duplicate writes
        if let Some(pool) = &self.pool {
            let user_id = self.user_id;

            // Check if a persistence is already in progress for this user
            let should_persist = {
                let mut locks = USER_PERSIST_LOCKS.lock().await;
                if locks.contains(&user_id) {
                    // Already persisting for this user, skip this update
                    // (the most recent queue state will be picked up by the next persist)
                    tracing::debug!(user_id = %user_id, "Skipping queue persist - already in progress");
                    false
                } else {
                    locks.insert(user_id);
                    true
                }
            };

            if should_persist {
                let pool = pool.clone();
                let queue_clone = queue.clone();
                let semaphore = PERSIST_SEMAPHORE.clone();

                tokio::spawn(async move {
                    // Acquire semaphore permit (limits concurrent DB operations)
                    let _permit = match semaphore.acquire().await {
                        Ok(permit) => permit,
                        Err(_) => {
                            tracing::warn!(user_id = %user_id, "Semaphore closed, skipping persist");
                            // Release user lock
                            let mut locks = USER_PERSIST_LOCKS.lock().await;
                            locks.remove(&user_id);
                            return;
                        }
                    };

                    // Perform the persistence
                    if let Err(e) = persist_queue_to_db(pool, user_id, &queue_clone).await {
                        tracing::warn!(
                            user_id = %user_id,
                            error = %e,
                            "Failed to persist queue to database"
                        );
                    }

                    // Release user lock
                    let mut locks = USER_PERSIST_LOCKS.lock().await;
                    locks.remove(&user_id);
                });
            }
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
            new_device_id: Some(target_device_id.clone()),
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
    ///
    /// The `was_active` parameter indicates whether this device was the active device
    /// at the time of disconnection. This must be determined BEFORE the connection
    /// is removed from the connection manager to avoid race conditions.
    pub async fn handle_device_disconnected(&self, was_active: bool) {
        let event = SyncEvent::DeviceDisconnected {
            device_id: self.device_id.clone(),
        };
        self.pubsub.publish(self.user_id, event).await;

        // If this was the active device, clear active status and notify others
        if was_active {
            self.connection_manager.clear_active_device(self.user_id);

            // Broadcast that there's no longer an active device
            let change_event = SyncEvent::ActiveDeviceChanged {
                previous_device_id: Some(self.device_id.clone()),
                new_device_id: None,
            };
            self.pubsub.publish(self.user_id, change_event).await;
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

/// Persist queue state to database
///
/// Converts the WebSocket QueueState (with full track metadata) to a
/// SetQueue (track IDs only) and persists it via QueueRepository.
/// This runs in a spawned task to avoid blocking WebSocket handling.
///
/// # Errors
/// Returns an error if:
/// - Any track ID fails to parse as a valid UUID (atomic failure)
/// - The current_index exceeds i32::MAX
/// - Database operations fail
async fn persist_queue_to_db(
    pool: PgPool,
    user_id: Uuid,
    queue: &QueueState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Convert WebSocket QueueState to repository SetQueue
    // Use collect with Result to fail atomically if ANY UUID is invalid
    let track_ids: Vec<Uuid> = queue
        .tracks
        .iter()
        .enumerate()
        .map(|(idx, t)| {
            Uuid::parse_str(&t.id)
                .map_err(|e| format!("Invalid UUID at position {}: '{}' - {}", idx, t.id, e))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Bounds check for current_index (usize to i32 conversion)
    let current_index: i32 = queue.current_index.try_into().map_err(|_| {
        format!(
            "current_index {} exceeds maximum i32 value",
            queue.current_index
        )
    })?;

    let set_queue = SetQueue::new(track_ids, current_index);

    // Persist to database
    let repo = QueueRepository::new(pool);
    repo.set_queue(user_id, &set_queue).await?;

    tracing::debug!(
        user_id = %user_id,
        track_count = queue.tracks.len(),
        current_index = queue.current_index,
        "Queue persisted to database"
    );

    Ok(())
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

/// Unit tests for SyncHandler methods
#[cfg(test)]
mod handler_tests {
    use super::*;
    use crate::websocket::connection::DeviceInfo;
    use crate::websocket::messages::{QueueTrack, RepeatMode};
    use tokio::sync::mpsc;

    /// Helper to create a test setup with connection manager, pubsub, and handler
    struct TestSetup {
        user_id: Uuid,
        device_id: String,
        connection_manager: ConnectionManager,
        pubsub: SyncPubSub,
        handler: SyncHandler,
        /// Receiver for messages sent to this device
        rx: mpsc::UnboundedReceiver<ServerMessage>,
    }

    impl TestSetup {
        fn new(device_id: &str) -> Self {
            let user_id = Uuid::new_v4();
            let device_id = device_id.to_string();
            let connection_manager = ConnectionManager::new();
            let pubsub = SyncPubSub::new_in_memory();

            // Create channel and register connection
            let (tx, rx) = mpsc::unbounded_channel();
            let device_info = DeviceInfo::new(
                device_id.clone(),
                Some("Test Device".to_string()),
                Some("desktop".to_string()),
            );
            connection_manager.add_connection(user_id, device_id.clone(), tx, device_info);

            let handler = SyncHandler::new(
                user_id,
                device_id.clone(),
                connection_manager.clone(),
                pubsub.clone(),
            );

            Self {
                user_id,
                device_id,
                connection_manager,
                pubsub,
                handler,
                rx,
            }
        }

        /// Add another device and return its receiver
        fn add_device(&self, device_id: &str) -> mpsc::UnboundedReceiver<ServerMessage> {
            let (tx, rx) = mpsc::unbounded_channel();
            let device_info = DeviceInfo::new(
                device_id.to_string(),
                Some("Other Device".to_string()),
                Some("mobile".to_string()),
            );
            self.connection_manager.add_connection(
                self.user_id,
                device_id.to_string(),
                tx,
                device_info,
            );
            rx
        }

        /// Make this device the active device
        fn make_active(&self) {
            self.connection_manager
                .set_active_device(self.user_id, &self.device_id);
        }

        /// Create a handler for a different device
        fn handler_for(&self, device_id: &str) -> SyncHandler {
            SyncHandler::new(
                self.user_id,
                device_id.to_string(),
                self.connection_manager.clone(),
                self.pubsub.clone(),
            )
        }
    }

    // =========================================================================
    // handle_playback_update tests
    // =========================================================================

    #[tokio::test]
    async fn test_playback_update_as_active_device() {
        let mut setup = TestSetup::new("device-1");
        setup.make_active();

        // Subscribe to pubsub to verify broadcast
        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        // Add another device to verify it receives the broadcast
        let _other_rx = setup.add_device("device-2");

        let state = PlaybackState {
            track_id: Some("track-123".to_string()),
            is_playing: true,
            position_ms: 30000,
            timestamp: chrono::Utc::now().timestamp_millis(),
            volume: 0.8,
            is_muted: false,
            shuffle: false,
            repeat: RepeatMode::Off,
        };

        let result = setup.handler.handle_playback_update(state.clone()).await;
        assert!(result.is_ok());

        // Verify playback state was stored
        let stored_state = setup.connection_manager.get_playback_state(setup.user_id);
        assert!(stored_state.is_some());
        let stored = stored_state.unwrap();
        assert_eq!(stored.track_id, Some("track-123".to_string()));
        assert!(stored.is_playing);

        // Verify event was published to pubsub
        let event = pubsub_rx.try_recv();
        assert!(event.is_ok());
        if let Ok(SyncEvent::PlaybackUpdate {
            device_id,
            state: _,
        }) = event
        {
            assert_eq!(device_id, "device-1");
        } else {
            panic!("Expected PlaybackUpdate event");
        }

        // Verify the sending device does NOT receive an error
        assert!(setup.rx.try_recv().is_err()); // No error message
    }

    #[tokio::test]
    async fn test_playback_update_as_non_active_device() {
        let mut setup = TestSetup::new("device-1");

        // Add device-2 and make it active
        let _other_rx = setup.add_device("device-2");
        setup
            .connection_manager
            .set_active_device(setup.user_id, "device-2");

        // Subscribe to verify NO broadcast happens
        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        let state = PlaybackState::default();

        let result = setup.handler.handle_playback_update(state).await;
        assert!(result.is_ok());

        // Verify error was sent to the non-active device
        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        if let Ok(ServerMessage::Error(error)) = msg {
            assert_eq!(error.code, "NOT_ACTIVE_DEVICE");
        } else {
            panic!("Expected Error message, got {:?}", msg);
        }

        // Verify NO event was published to pubsub
        // Give a tiny bit of time for any async operations
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        assert!(pubsub_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_playback_update_no_active_device() {
        let mut setup = TestSetup::new("device-1");

        // No device is active - playback update should be rejected
        let state = PlaybackState::default();

        let result = setup.handler.handle_playback_update(state).await;
        assert!(result.is_ok());

        // Verify error was sent (device is not active)
        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        if let Ok(ServerMessage::Error(error)) = msg {
            assert_eq!(error.code, "NOT_ACTIVE_DEVICE");
        } else {
            panic!("Expected Error message");
        }
    }

    // =========================================================================
    // handle_seek tests
    // =========================================================================

    #[tokio::test]
    async fn test_seek_as_active_device_broadcasts() {
        let setup = TestSetup::new("device-1");
        setup.make_active();

        // Subscribe to pubsub to verify broadcast
        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        let position_ms = 45000u64;
        let result = setup.handler.handle_seek(position_ms).await;
        assert!(result.is_ok());

        // Verify seek event was published
        let event = pubsub_rx.try_recv();
        assert!(event.is_ok());
        match event.unwrap() {
            SyncEvent::SeekUpdate {
                device_id,
                position_ms: pos,
                timestamp,
            } => {
                assert_eq!(device_id, "device-1");
                assert_eq!(pos, 45000);
                // Timestamp should be close to now
                let now = chrono::Utc::now().timestamp_millis();
                assert!((now - timestamp).abs() < 1000); // Within 1 second
            }
            _ => panic!("Expected SeekUpdate event"),
        }
    }

    #[tokio::test]
    async fn test_seek_as_non_active_device_rejected() {
        let mut setup = TestSetup::new("device-1");

        // No active device, so seek should be rejected
        let result = setup.handler.handle_seek(30000).await;
        assert!(result.is_ok());

        // Verify error was sent
        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        if let Ok(ServerMessage::Error(error)) = msg {
            assert_eq!(error.code, "NOT_ACTIVE_DEVICE");
        } else {
            panic!("Expected Error message");
        }
    }

    #[tokio::test]
    async fn test_seek_timestamp_is_current() {
        let setup = TestSetup::new("device-1");
        setup.make_active();

        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        let before = chrono::Utc::now().timestamp_millis();
        let _ = setup.handler.handle_seek(0).await;
        let after = chrono::Utc::now().timestamp_millis();

        if let Ok(SyncEvent::SeekUpdate { timestamp, .. }) = pubsub_rx.try_recv() {
            assert!(timestamp >= before);
            assert!(timestamp <= after);
        } else {
            panic!("Expected SeekUpdate event");
        }
    }

    // =========================================================================
    // handle_transfer_request tests
    // =========================================================================

    #[tokio::test]
    async fn test_transfer_request_from_active_device() {
        let setup = TestSetup::new("device-1");
        setup.make_active();
        let _device_2_rx = setup.add_device("device-2");

        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        // Transfer from active device-1 to device-2
        let result = setup
            .handler
            .handle_transfer_request("device-2".to_string())
            .await;
        assert!(result.is_ok());

        // Verify device-2 is now active
        assert_eq!(
            setup.connection_manager.get_active_device(setup.user_id),
            Some("device-2".to_string())
        );

        // Verify events were published (TransferRequest, ActiveDeviceChanged, TransferAccept)
        let mut events = Vec::new();
        while let Ok(event) = pubsub_rx.try_recv() {
            events.push(event);
        }

        assert!(events.len() >= 3);

        // Check for TransferRequest
        assert!(events.iter().any(|e| matches!(
            e,
            SyncEvent::TransferRequest {
                from_device_id,
                to_device_id,
            } if from_device_id == "device-1" && to_device_id == "device-2"
        )));

        // Check for ActiveDeviceChanged
        assert!(events.iter().any(|e| matches!(
            e,
            SyncEvent::ActiveDeviceChanged {
                previous_device_id: Some(prev),
                new_device_id: Some(new),
            } if prev == "device-1" && new == "device-2"
        )));

        // Check for TransferAccept
        assert!(events.iter().any(|e| matches!(
            e,
            SyncEvent::TransferAccept {
                to_device_id,
                ..
            } if to_device_id == "device-2"
        )));
    }

    #[tokio::test]
    async fn test_transfer_request_from_non_active_device_rejected() {
        let mut setup = TestSetup::new("device-1");

        // Add device-2 and make it active
        let _device_2_rx = setup.add_device("device-2");
        setup
            .connection_manager
            .set_active_device(setup.user_id, "device-2");

        // device-1 (non-active) tries to transfer - should be rejected
        let result = setup
            .handler
            .handle_transfer_request("device-1".to_string())
            .await;
        assert!(result.is_ok());

        // Verify error was sent
        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        if let Ok(ServerMessage::Error(error)) = msg {
            assert_eq!(error.code, "NOT_AUTHORIZED");
        } else {
            panic!("Expected Error message");
        }

        // Verify device-2 is still active
        assert_eq!(
            setup.connection_manager.get_active_device(setup.user_id),
            Some("device-2".to_string())
        );
    }

    #[tokio::test]
    async fn test_transfer_first_device_claim_when_no_active() {
        let setup = TestSetup::new("device-1");
        let _device_2_rx = setup.add_device("device-2");

        // No active device exists - device-1 can claim device-2 as active
        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        let result = setup
            .handler
            .handle_transfer_request("device-2".to_string())
            .await;
        assert!(result.is_ok());

        // Verify device-2 is now active
        assert_eq!(
            setup.connection_manager.get_active_device(setup.user_id),
            Some("device-2".to_string())
        );

        // Verify ActiveDeviceChanged event with previous_device_id = None
        let mut found_active_changed = false;
        while let Ok(event) = pubsub_rx.try_recv() {
            if let SyncEvent::ActiveDeviceChanged {
                previous_device_id,
                new_device_id,
            } = event
            {
                assert!(previous_device_id.is_none());
                assert_eq!(new_device_id, Some("device-2".to_string()));
                found_active_changed = true;
            }
        }
        assert!(found_active_changed);
    }

    #[tokio::test]
    async fn test_transfer_to_nonexistent_device_rejected() {
        let mut setup = TestSetup::new("device-1");
        setup.make_active();

        // Try to transfer to a device that doesn't exist
        let result = setup
            .handler
            .handle_transfer_request("nonexistent".to_string())
            .await;
        assert!(result.is_ok());

        // Verify error was sent
        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        if let Ok(ServerMessage::Error(error)) = msg {
            assert_eq!(error.code, "DEVICE_NOT_FOUND");
            assert!(error.message.contains("nonexistent"));
        } else {
            panic!("Expected Error message");
        }

        // Verify device-1 is still active
        assert_eq!(
            setup.connection_manager.get_active_device(setup.user_id),
            Some("device-1".to_string())
        );
    }

    #[tokio::test]
    async fn test_transfer_to_self_when_active_rejected() {
        let mut setup = TestSetup::new("device-1");
        setup.make_active();

        // Try to transfer to self while already active
        let result = setup
            .handler
            .handle_transfer_request("device-1".to_string())
            .await;
        assert!(result.is_ok());

        // Verify error was sent
        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        if let Ok(ServerMessage::Error(error)) = msg {
            assert_eq!(error.code, "INVALID_TARGET");
            assert!(error.message.contains("Already"));
        } else {
            panic!("Expected Error message");
        }
    }

    // =========================================================================
    // handle_queue_update tests
    // =========================================================================

    #[tokio::test]
    async fn test_queue_update_broadcasts_as_active_device() {
        let setup = TestSetup::new("device-1");
        setup.make_active();

        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        let queue = QueueState {
            tracks: vec![QueueTrack {
                id: Uuid::new_v4().to_string(),
                title: "Test Track".to_string(),
                artist: "Test Artist".to_string(),
                album_id: None,
                album_title: "Test Album".to_string(),
                duration_ms: 180000,
                cover_url: None,
            }],
            current_index: 0,
        };

        let result = setup.handler.handle_queue_update(queue.clone()).await;
        assert!(result.is_ok());

        // Verify QueueUpdate event was published
        let event = pubsub_rx.try_recv();
        assert!(event.is_ok());
        match event.unwrap() {
            SyncEvent::QueueUpdate { device_id, state } => {
                assert_eq!(device_id, "device-1");
                assert_eq!(state.tracks.len(), 1);
                assert_eq!(state.current_index, 0);
            }
            _ => panic!("Expected QueueUpdate event"),
        }
    }

    #[tokio::test]
    async fn test_queue_update_rejected_as_non_active_device() {
        let mut setup = TestSetup::new("device-1");

        // No active device
        let queue = QueueState::default();

        let result = setup.handler.handle_queue_update(queue).await;
        assert!(result.is_ok());

        // Verify error was sent
        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        if let Ok(ServerMessage::Error(error)) = msg {
            assert_eq!(error.code, "NOT_ACTIVE_DEVICE");
        } else {
            panic!("Expected Error message");
        }
    }

    #[tokio::test]
    async fn test_queue_update_with_multiple_tracks() {
        let setup = TestSetup::new("device-1");
        setup.make_active();

        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        let queue = QueueState {
            tracks: vec![
                QueueTrack {
                    id: Uuid::new_v4().to_string(),
                    title: "Track 1".to_string(),
                    artist: "Artist 1".to_string(),
                    album_id: None,
                    album_title: "Album 1".to_string(),
                    duration_ms: 180000,
                    cover_url: None,
                },
                QueueTrack {
                    id: Uuid::new_v4().to_string(),
                    title: "Track 2".to_string(),
                    artist: "Artist 2".to_string(),
                    album_id: None,
                    album_title: "Album 2".to_string(),
                    duration_ms: 200000,
                    cover_url: None,
                },
                QueueTrack {
                    id: Uuid::new_v4().to_string(),
                    title: "Track 3".to_string(),
                    artist: "Artist 3".to_string(),
                    album_id: None,
                    album_title: "Album 3".to_string(),
                    duration_ms: 220000,
                    cover_url: None,
                },
            ],
            current_index: 1, // Currently on track 2
        };

        let result = setup.handler.handle_queue_update(queue).await;
        assert!(result.is_ok());

        // Verify queue state in broadcast
        if let Ok(SyncEvent::QueueUpdate { state, .. }) = pubsub_rx.try_recv() {
            assert_eq!(state.tracks.len(), 3);
            assert_eq!(state.current_index, 1);
            assert_eq!(state.tracks[1].title, "Track 2");
        } else {
            panic!("Expected QueueUpdate event");
        }
    }

    // =========================================================================
    // handle_heartbeat tests
    // =========================================================================

    #[tokio::test]
    async fn test_heartbeat_returns_pong() {
        let mut setup = TestSetup::new("device-1");

        let before = chrono::Utc::now().timestamp_millis();
        let result = setup.handler.handle_heartbeat().await;
        let after = chrono::Utc::now().timestamp_millis();

        assert!(result.is_ok());

        // Verify Pong message was sent
        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        match msg.unwrap() {
            ServerMessage::Pong { server_time } => {
                // Server time should be between before and after
                assert!(server_time >= before);
                assert!(server_time <= after);
            }
            other => panic!("Expected Pong message, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_heartbeat_works_without_active_device() {
        let mut setup = TestSetup::new("device-1");

        // No active device - heartbeat should still work
        let result = setup.handler.handle_heartbeat().await;
        assert!(result.is_ok());

        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        assert!(matches!(msg.unwrap(), ServerMessage::Pong { .. }));
    }

    #[tokio::test]
    async fn test_heartbeat_server_time_is_current() {
        let mut setup = TestSetup::new("device-1");

        let now_before = chrono::Utc::now().timestamp_millis();
        let _ = setup.handler.handle_heartbeat().await;
        let now_after = chrono::Utc::now().timestamp_millis();

        if let Ok(ServerMessage::Pong { server_time }) = setup.rx.try_recv() {
            // The server_time should be within the window
            assert!(
                server_time >= now_before && server_time <= now_after,
                "server_time {} should be between {} and {}",
                server_time,
                now_before,
                now_after
            );
        } else {
            panic!("Expected Pong message");
        }
    }

    // =========================================================================
    // handle_device_list_request tests
    // =========================================================================

    #[tokio::test]
    async fn test_device_list_request_returns_connected_devices() {
        let mut setup = TestSetup::new("device-1");
        setup.make_active();
        let _device_2_rx = setup.add_device("device-2");

        let result = setup.handler.handle_device_list_request().await;
        assert!(result.is_ok());

        let msg = setup.rx.try_recv();
        assert!(msg.is_ok());
        match msg.unwrap() {
            ServerMessage::DeviceList(devices) => {
                assert_eq!(devices.len(), 2);
                let device_ids: Vec<&str> = devices.iter().map(|d| d.device_id.as_str()).collect();
                assert!(device_ids.contains(&"device-1"));
                assert!(device_ids.contains(&"device-2"));

                // Verify active device flag is set correctly
                let active_device = devices.iter().find(|d| d.device_id == "device-1").unwrap();
                assert!(active_device.is_active);

                let other_device = devices.iter().find(|d| d.device_id == "device-2").unwrap();
                assert!(!other_device.is_active);
            }
            other => panic!("Expected DeviceList message, got {:?}", other),
        }
    }

    // =========================================================================
    // handle_settings_update tests
    // =========================================================================

    #[tokio::test]
    async fn test_settings_update_broadcasts() {
        let setup = TestSetup::new("device-1");

        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        let settings = SyncedSettings {
            crossfade_enabled: Some(true),
            crossfade_duration: Some(5.0),
            gapless_enabled: Some(true),
            normalize_volume: Some(false),
        };

        let result = setup.handler.handle_settings_update(settings.clone()).await;
        assert!(result.is_ok());

        // Verify settings event was published
        let event = pubsub_rx.try_recv();
        assert!(event.is_ok());
        match event.unwrap() {
            SyncEvent::SettingsUpdate {
                device_id,
                settings,
            } => {
                assert_eq!(device_id, "device-1");
                assert_eq!(settings.crossfade_enabled, Some(true));
                assert_eq!(settings.crossfade_duration, Some(5.0));
            }
            _ => panic!("Expected SettingsUpdate event"),
        }
    }

    // =========================================================================
    // handle_message tests (integration of all handlers)
    // =========================================================================

    #[tokio::test]
    async fn test_handle_message_routes_correctly() {
        let mut setup = TestSetup::new("device-1");
        setup.make_active();

        // Test Heartbeat routing
        let result = setup.handler.handle_message(ClientMessage::Heartbeat).await;
        assert!(result.is_ok());
        assert!(matches!(
            setup.rx.try_recv().unwrap(),
            ServerMessage::Pong { .. }
        ));

        // Test RequestDeviceList routing
        let result = setup
            .handler
            .handle_message(ClientMessage::RequestDeviceList)
            .await;
        assert!(result.is_ok());
        assert!(matches!(
            setup.rx.try_recv().unwrap(),
            ServerMessage::DeviceList(_)
        ));
    }

    #[tokio::test]
    async fn test_handle_message_rejects_stale_connection() {
        let setup = TestSetup::new("device-1");

        // Remove the connection to simulate stale state
        setup
            .connection_manager
            .remove_connection(setup.user_id, "device-1");

        // Message should be rejected because device is not found
        let result = setup.handler.handle_message(ClientMessage::Heartbeat).await;
        assert!(result.is_err());
        match result {
            Err(SyncError::Internal(msg)) => {
                assert!(msg.contains("device not found"));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    // =========================================================================
    // handle_device_connected / handle_device_disconnected tests
    // =========================================================================

    #[tokio::test]
    async fn test_device_connected_broadcasts() {
        let setup = TestSetup::new("device-1");

        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        let device_info = DeviceInfo::new(
            "device-1".to_string(),
            Some("My Device".to_string()),
            Some("desktop".to_string()),
        );

        setup.handler.handle_device_connected(device_info).await;

        // Verify DeviceConnected event was published
        let event = pubsub_rx.try_recv();
        assert!(event.is_ok());
        match event.unwrap() {
            SyncEvent::DeviceConnected { presence } => {
                assert_eq!(presence.device_id, "device-1");
                assert_eq!(presence.device_name, "My Device");
            }
            _ => panic!("Expected DeviceConnected event"),
        }
    }

    #[tokio::test]
    async fn test_device_disconnected_broadcasts() {
        let setup = TestSetup::new("device-1");

        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        // Disconnect without being active
        setup.handler.handle_device_disconnected(false).await;

        // Verify DeviceDisconnected event was published
        let event = pubsub_rx.try_recv();
        assert!(event.is_ok());
        match event.unwrap() {
            SyncEvent::DeviceDisconnected { device_id } => {
                assert_eq!(device_id, "device-1");
            }
            _ => panic!("Expected DeviceDisconnected event"),
        }

        // Verify NO ActiveDeviceChanged event (not active)
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        assert!(pubsub_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_active_device_disconnected_clears_and_broadcasts() {
        let setup = TestSetup::new("device-1");
        setup.make_active();

        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        // Disconnect as active device
        setup.handler.handle_device_disconnected(true).await;

        // Collect all events
        let mut events = Vec::new();
        while let Ok(event) = pubsub_rx.try_recv() {
            events.push(event);
        }

        // Should have DeviceDisconnected and ActiveDeviceChanged events
        assert!(events.iter().any(
            |e| matches!(e, SyncEvent::DeviceDisconnected { device_id } if device_id == "device-1")
        ));

        assert!(events.iter().any(|e| matches!(
            e,
            SyncEvent::ActiveDeviceChanged {
                previous_device_id: Some(prev),
                new_device_id: None,
            } if prev == "device-1"
        )));

        // Verify active device was cleared
        assert!(setup
            .connection_manager
            .get_active_device(setup.user_id)
            .is_none());
    }

    // =========================================================================
    // Edge case tests
    // =========================================================================

    #[tokio::test]
    async fn test_multiple_handlers_same_user() {
        let setup = TestSetup::new("device-1");
        let _device_2_rx = setup.add_device("device-2");

        let handler_1 = setup.handler_for("device-1");
        let handler_2 = setup.handler_for("device-2");

        // Make device-1 active
        setup
            .connection_manager
            .set_active_device(setup.user_id, "device-1");

        let mut pubsub_rx = setup.pubsub.subscribe(setup.user_id).await;

        // Handler 1 (active) sends playback update
        let state = PlaybackState::default();
        let result = handler_1.handle_playback_update(state).await;
        assert!(result.is_ok());

        // Handler 2 (not active) tries to send - should be rejected
        let state2 = PlaybackState::default();
        let result = handler_2.handle_playback_update(state2).await;
        assert!(result.is_ok()); // Returns Ok but sends error

        // Only one PlaybackUpdate should be in pubsub (from handler_1)
        let event_count = std::iter::from_fn(|| pubsub_rx.try_recv().ok())
            .filter(|e| matches!(e, SyncEvent::PlaybackUpdate { .. }))
            .count();
        assert_eq!(event_count, 1);
    }
}
