//! Redis pub/sub for cross-instance synchronization
//!
//! This module provides real-time event distribution using Redis pub/sub
//! for multi-instance deployments, with an in-memory fallback for single
//! instance mode when Redis is unavailable.

use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::messages::SyncEvent;

/// Channel capacity for broadcast channels
const BROADCAST_CAPACITY: usize = 256;

/// Sync pub/sub system with Redis + in-memory fallback
#[derive(Clone)]
pub struct SyncPubSub {
    inner: Arc<SyncPubSubInner>,
}

enum SyncPubSubInner {
    /// Redis-backed pub/sub for multi-instance deployments
    Redis(RedisPubSub),
    /// In-memory pub/sub for single-instance mode
    InMemory(InMemoryPubSub),
}

impl SyncPubSub {
    /// Create a new pub/sub system with Redis
    pub fn new_with_redis(client: redis::Client) -> Self {
        Self {
            inner: Arc::new(SyncPubSubInner::Redis(RedisPubSub::new(client))),
        }
    }

    /// Create a new in-memory pub/sub system (single instance mode)
    pub fn new_in_memory() -> Self {
        Self {
            inner: Arc::new(SyncPubSubInner::InMemory(InMemoryPubSub::new())),
        }
    }

    /// Try to create with Redis, fall back to in-memory
    pub async fn try_with_redis(redis_url: &str) -> Self {
        match redis::Client::open(redis_url) {
            Ok(client) => {
                // Test connection
                match client.get_multiplexed_async_connection().await {
                    Ok(mut conn) => {
                        let pong: Result<String, _> =
                            redis::cmd("PING").query_async(&mut conn).await;
                        if pong.is_ok() {
                            tracing::info!("Redis pub/sub connected for real-time sync");
                            return Self::new_with_redis(client);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Redis pub/sub connection failed");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Redis client creation failed for pub/sub");
            }
        }

        tracing::warn!("Using in-memory pub/sub (single instance mode only)");
        Self::new_in_memory()
    }

    /// Publish an event for a specific user
    pub async fn publish(&self, user_id: Uuid, event: SyncEvent) {
        match &*self.inner {
            SyncPubSubInner::Redis(redis) => redis.publish(user_id, event).await,
            SyncPubSubInner::InMemory(memory) => memory.publish(user_id, event),
        }
    }

    /// Subscribe to events for a specific user
    pub async fn subscribe(&self, user_id: Uuid) -> broadcast::Receiver<SyncEvent> {
        match &*self.inner {
            SyncPubSubInner::Redis(redis) => redis.subscribe(user_id).await,
            SyncPubSubInner::InMemory(memory) => memory.subscribe(user_id),
        }
    }

    /// Check if we're using Redis (multi-instance capable)
    pub fn is_redis_backed(&self) -> bool {
        matches!(&*self.inner, SyncPubSubInner::Redis(_))
    }
}

/// Redis-backed pub/sub implementation
struct RedisPubSub {
    client: redis::Client,
    /// Local broadcast for redistribution to local subscribers
    local_sender: broadcast::Sender<(Uuid, SyncEvent)>,
}

impl RedisPubSub {
    fn new(client: redis::Client) -> Self {
        let (local_sender, _) = broadcast::channel(BROADCAST_CAPACITY);

        let pubsub = Self {
            client,
            local_sender,
        };

        // Start background task to listen for Redis pub/sub messages
        pubsub.start_listener();

        pubsub
    }

    fn start_listener(&self) {
        let client = self.client.clone();
        let sender = self.local_sender.clone();

        tokio::spawn(async move {
            const MAX_RECONNECT_DELAY_SECS: u64 = 60;
            const MAX_RECONNECT_ATTEMPTS: u32 = 100;

            let mut attempts = 0u32;
            let mut delay_secs = 1u64;

            loop {
                match Self::run_listener(&client, &sender).await {
                    Ok(()) => {
                        // Normal disconnection - still try to reconnect
                        tracing::warn!("Redis pub/sub listener disconnected, reconnecting...");
                        // Reset backoff and attempt counter on clean disconnect
                        attempts = 0;
                        delay_secs = 1;
                    }
                    Err(e) => {
                        attempts += 1;
                        if attempts >= MAX_RECONNECT_ATTEMPTS {
                            tracing::error!(
                                "Redis pub/sub max reconnect attempts ({}) exceeded, giving up",
                                MAX_RECONNECT_ATTEMPTS
                            );
                            break;
                        }
                        tracing::error!(
                            error = %e,
                            attempt = attempts,
                            delay_secs = delay_secs,
                            "Redis pub/sub listener error, reconnecting..."
                        );
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
                delay_secs = (delay_secs * 2).min(MAX_RECONNECT_DELAY_SECS);
            }
        });
    }

    async fn run_listener(
        client: &redis::Client,
        sender: &broadcast::Sender<(Uuid, SyncEvent)>,
    ) -> Result<(), redis::RedisError> {
        use futures_util::StreamExt;

        let conn = client.get_async_connection().await?;
        let mut pubsub = conn.into_pubsub();

        // Subscribe to the sync channel pattern
        pubsub.psubscribe("sync:user:*").await?;

        let mut stream = pubsub.on_message();

        while let Some(msg) = stream.next().await {
            let channel: String = msg.get_channel_name().to_string();
            let payload: Vec<u8> = msg.get_payload_bytes().to_vec();

            // Parse channel name to extract user_id
            // Format: sync:user:{user_id}
            if let Some(user_id_str) = channel.strip_prefix("sync:user:") {
                if let Ok(user_id) = Uuid::parse_str(user_id_str) {
                    if let Ok(payload_str) = String::from_utf8(payload) {
                        if let Ok(event) = serde_json::from_str::<SyncEvent>(&payload_str) {
                            // Broadcast to local subscribers
                            let _ = sender.send((user_id, event));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn publish(&self, user_id: Uuid, event: SyncEvent) {
        let channel = format!("sync:user:{}", user_id);

        match serde_json::to_string(&event) {
            Ok(payload) => {
                match self.client.get_multiplexed_async_connection().await {
                    Ok(mut conn) => {
                        let result: Result<(), _> = redis::cmd("PUBLISH")
                            .arg(&channel)
                            .arg(&payload)
                            .query_async(&mut conn)
                            .await;

                        if let Err(e) = result {
                            tracing::error!(error = %e, "Failed to publish to Redis");
                            // Fall back to local broadcast
                            let _ = self.local_sender.send((user_id, event));
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to get Redis connection for publish");
                        // Fall back to local broadcast
                        let _ = self.local_sender.send((user_id, event));
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize sync event");
            }
        }
    }

    async fn subscribe(&self, user_id: Uuid) -> broadcast::Receiver<SyncEvent> {
        // Create a filtered receiver that only receives events for this user
        let (tx, rx) = broadcast::channel(BROADCAST_CAPACITY);
        let mut global_rx = self.local_sender.subscribe();

        tokio::spawn(async move {
            while let Ok((event_user_id, event)) = global_rx.recv().await {
                if event_user_id == user_id && tx.send(event).is_err() {
                    // No more receivers, stop filtering
                    break;
                }
            }
        });

        rx
    }
}

/// In-memory pub/sub implementation for single-instance mode
struct InMemoryPubSub {
    /// Per-user broadcast channels
    channels: dashmap::DashMap<Uuid, broadcast::Sender<SyncEvent>>,
}

impl InMemoryPubSub {
    fn new() -> Self {
        Self {
            channels: dashmap::DashMap::new(),
        }
    }

    fn publish(&self, user_id: Uuid, event: SyncEvent) {
        if let Some(sender) = self.channels.get(&user_id) {
            // Ignore send errors (no receivers)
            let _ = sender.send(event);
        }
    }

    fn subscribe(&self, user_id: Uuid) -> broadcast::Receiver<SyncEvent> {
        let sender = self
            .channels
            .entry(user_id)
            .or_insert_with(|| broadcast::channel(BROADCAST_CAPACITY).0);
        sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::messages::PlaybackState;

    #[tokio::test]
    async fn test_in_memory_pubsub() {
        let pubsub = SyncPubSub::new_in_memory();
        let user_id = Uuid::new_v4();

        // Subscribe first
        let mut rx = pubsub.subscribe(user_id).await;

        // Publish an event
        let event = SyncEvent::PlaybackUpdate {
            device_id: "device-1".to_string(),
            state: PlaybackState::default(),
        };
        pubsub.publish(user_id, event.clone()).await;

        // Should receive the event
        let received = rx.recv().await.unwrap();
        assert!(matches!(received, SyncEvent::PlaybackUpdate { .. }));
    }

    #[tokio::test]
    async fn test_in_memory_pubsub_user_isolation() {
        let pubsub = SyncPubSub::new_in_memory();
        let user_1 = Uuid::new_v4();
        let user_2 = Uuid::new_v4();

        // Subscribe to user_2's events
        let mut rx = pubsub.subscribe(user_2).await;

        // Publish to user_1 (different user)
        let event = SyncEvent::PlaybackUpdate {
            device_id: "device-1".to_string(),
            state: PlaybackState::default(),
        };
        pubsub.publish(user_1, event).await;

        // Give some time for message to propagate
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Should not receive any event (different user)
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_is_redis_backed() {
        let in_memory = SyncPubSub::new_in_memory();
        assert!(!in_memory.is_redis_backed());
    }
}
