//! Queue models for persistent play queue storage
//!
//! This module provides database models for the user's play queue,
//! enabling cross-session persistence and queue-based prefetch
//! by the background worker.
//!
//! The hot path for real-time sync uses Redis pub/sub via WebSocket,
//! while these models represent the durable persistence layer.
//!
//! Note: `QueuePlaybackState` is the database model for playback position.
//! The WebSocket layer has its own `QueueState` (in websocket/messages.rs)
//! which includes full track metadata for client display.

// Allow unused code - these models are prepared for QueueRepository integration
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use thiserror::Error;
use uuid::Uuid;

/// Maximum queue size to prevent abuse
pub const MAX_QUEUE_SIZE: usize = 10_000;

/// Context type enum matching PostgreSQL context_type
///
/// Represents where a track was discovered/played from.
/// Used for analytics and prefetch weighting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "context_type", rename_all = "lowercase")]
pub enum ContextType {
    Album,
    Artist,
    Playlist,
    Search,
    Recommendation,
    Radio,
    #[default]
    Queue,
}

/// A single item in the user's play queue
///
/// Queue items are ordered by `position` (0-indexed).
/// The same track can appear multiple times at different positions.
#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct QueueItem {
    /// Unique queue item identifier
    pub id: Uuid,

    /// User who owns this queue
    pub user_id: Uuid,

    /// Track in the queue
    pub track_id: Uuid,

    /// Position in queue (0-indexed, lower = earlier)
    pub position: i32,

    /// Context where this track was added from
    pub source_type: Option<ContextType>,

    /// ID of the source (e.g., playlist_id, album_id)
    pub source_id: Option<Uuid>,

    /// When this track was added to the queue
    pub added_at: DateTime<Utc>,

    /// Metadata for prefetch optimization
    /// e.g., { "prefetched": true, "prefetch_priority": 0.85 }
    #[sqlx(json)]
    pub metadata: serde_json::Value,
}

impl QueueItem {
    /// Check if this item has been prefetched
    pub fn is_prefetched(&self) -> bool {
        self.metadata
            .get("prefetched")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Get prefetch priority (0.0-1.0)
    pub fn prefetch_priority(&self) -> Option<f64> {
        self.metadata
            .get("prefetch_priority")
            .and_then(|v| v.as_f64())
    }
}

/// Current playback state within the queue (database model)
///
/// Stored separately from queue_items to avoid row contention
/// during frequent index updates (every track transition).
///
/// Note: This is the database model. The WebSocket layer has its own
/// `QueueState` type (in websocket/messages.rs) that includes full
/// track metadata for client display.
///
/// `current_index` may temporarily exceed the actual queue length
/// after track removals. The service layer must handle this gracefully
/// by clamping to valid bounds.
#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct QueuePlaybackState {
    /// User who owns this queue state
    pub user_id: Uuid,

    /// Current position in queue (index into queue_items by position)
    pub current_index: i32,

    /// Last time queue state was modified
    pub updated_at: DateTime<Utc>,
}

/// Errors that can occur during queue validation
#[derive(Debug, Error)]
pub enum QueueValidationError {
    #[error("queue exceeds maximum size of {MAX_QUEUE_SIZE} (got {0})")]
    TooManyTracks(usize),

    #[error("current_index cannot be negative (got {0})")]
    NegativeIndex(i32),

    #[error("current_index {index} is out of bounds for queue of length {len}")]
    IndexOutOfBounds { index: i32, len: usize },

    #[error("source_type and source_id must both be present or both be absent")]
    IncompleteSource,
}

/// Data for setting/replacing the entire queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetQueue {
    /// Track IDs in order
    pub track_ids: Vec<Uuid>,

    /// Current playback index
    pub current_index: i32,

    /// Optional source context for all tracks
    pub source_type: Option<ContextType>,

    /// Optional source ID for all tracks
    pub source_id: Option<Uuid>,
}

impl SetQueue {
    pub fn new(track_ids: Vec<Uuid>, current_index: i32) -> Self {
        Self {
            track_ids,
            current_index,
            source_type: None,
            source_id: None,
        }
    }

    pub fn with_source(mut self, source_type: ContextType, source_id: Uuid) -> Self {
        self.source_type = Some(source_type);
        self.source_id = Some(source_id);
        self
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.track_ids.is_empty()
    }

    /// Get the current track ID if index is valid
    pub fn current_track(&self) -> Option<Uuid> {
        self.track_ids.get(self.current_index as usize).copied()
    }

    /// Validate the queue data before database operations.
    ///
    /// This catches validation errors early (before hitting the database)
    /// and provides clear error messages for debugging.
    pub fn validate(&self) -> Result<(), QueueValidationError> {
        // Check queue size limit
        if self.track_ids.len() > MAX_QUEUE_SIZE {
            return Err(QueueValidationError::TooManyTracks(self.track_ids.len()));
        }

        // Check negative index
        if self.current_index < 0 {
            return Err(QueueValidationError::NegativeIndex(self.current_index));
        }

        // Check index bounds (only if queue is not empty)
        if !self.track_ids.is_empty() && self.current_index as usize >= self.track_ids.len() {
            return Err(QueueValidationError::IndexOutOfBounds {
                index: self.current_index,
                len: self.track_ids.len(),
            });
        }

        // Check source_type and source_id consistency
        if self.source_type.is_some() != self.source_id.is_some() {
            return Err(QueueValidationError::IncompleteSource);
        }

        Ok(())
    }
}

/// Simplified queue item for prefetch queries
#[derive(Debug, Clone, FromRow)]
pub struct QueueTrackId {
    pub track_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_type_values() {
        // Ensure all context types serialize correctly
        let contexts = [
            ContextType::Album,
            ContextType::Artist,
            ContextType::Playlist,
            ContextType::Search,
            ContextType::Recommendation,
            ContextType::Radio,
            ContextType::Queue,
        ];

        for ctx in contexts {
            let json = serde_json::to_string(&ctx).expect("should serialize");
            let deserialized: ContextType =
                serde_json::from_str(&json).expect("should deserialize");
            assert_eq!(ctx, deserialized);
        }
    }

    #[test]
    fn test_context_type_serde_lowercase() {
        // Verify serde produces correct values
        // Note: serde serializes enums as PascalCase by default, sqlx handles the DB mapping
        let album = serde_json::to_string(&ContextType::Album).unwrap();
        assert_eq!(album, "\"Album\"");

        let recommendation = serde_json::to_string(&ContextType::Recommendation).unwrap();
        assert_eq!(recommendation, "\"Recommendation\"");
    }

    #[test]
    fn test_context_type_default() {
        assert_eq!(ContextType::default(), ContextType::Queue);
    }

    #[test]
    fn test_set_queue_builder() {
        let track_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let playlist_id = Uuid::new_v4();

        let set_queue =
            SetQueue::new(track_ids.clone(), 0).with_source(ContextType::Playlist, playlist_id);

        assert_eq!(set_queue.track_ids, track_ids);
        assert_eq!(set_queue.current_index, 0);
        assert_eq!(set_queue.source_type, Some(ContextType::Playlist));
        assert_eq!(set_queue.source_id, Some(playlist_id));
    }

    #[test]
    fn test_set_queue_empty() {
        let set_queue = SetQueue::new(vec![], 0);
        assert!(set_queue.is_empty());
        assert!(set_queue.current_track().is_none());
        assert!(set_queue.validate().is_ok());
    }

    #[test]
    fn test_set_queue_current_track() {
        let track_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let set_queue = SetQueue::new(track_ids.clone(), 1);

        assert_eq!(set_queue.current_track(), Some(track_ids[1]));
    }

    #[test]
    fn test_set_queue_validation_valid() {
        let set_queue = SetQueue::new(vec![Uuid::new_v4(), Uuid::new_v4()], 0);
        assert!(set_queue.validate().is_ok());
    }

    #[test]
    fn test_set_queue_validation_negative_index() {
        let set_queue = SetQueue::new(vec![Uuid::new_v4()], -1);
        assert!(matches!(
            set_queue.validate(),
            Err(QueueValidationError::NegativeIndex(-1))
        ));
    }

    #[test]
    fn test_set_queue_validation_index_out_of_bounds() {
        let set_queue = SetQueue::new(vec![Uuid::new_v4()], 5);
        assert!(matches!(
            set_queue.validate(),
            Err(QueueValidationError::IndexOutOfBounds { index: 5, len: 1 })
        ));
    }

    #[test]
    fn test_set_queue_validation_incomplete_source() {
        let mut set_queue = SetQueue::new(vec![Uuid::new_v4()], 0);
        set_queue.source_type = Some(ContextType::Playlist);
        // source_id is None

        assert!(matches!(
            set_queue.validate(),
            Err(QueueValidationError::IncompleteSource)
        ));
    }

    #[test]
    fn test_set_queue_validation_too_many_tracks() {
        let track_ids: Vec<Uuid> = (0..MAX_QUEUE_SIZE + 1).map(|_| Uuid::new_v4()).collect();
        let set_queue = SetQueue::new(track_ids, 0);

        assert!(matches!(
            set_queue.validate(),
            Err(QueueValidationError::TooManyTracks(_))
        ));
    }

    #[test]
    fn test_queue_item_metadata_default() {
        // Verify default metadata is empty object
        let metadata: serde_json::Value = serde_json::json!({});
        assert!(metadata.is_object());
        assert!(metadata.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_queue_item_is_prefetched() {
        let item = create_test_queue_item(serde_json::json!({"prefetched": true}));
        assert!(item.is_prefetched());

        let item_not_prefetched = create_test_queue_item(serde_json::json!({}));
        assert!(!item_not_prefetched.is_prefetched());
    }

    #[test]
    fn test_queue_item_prefetch_priority() {
        let item = create_test_queue_item(serde_json::json!({"prefetch_priority": 0.85}));
        assert_eq!(item.prefetch_priority(), Some(0.85));

        let item_no_priority = create_test_queue_item(serde_json::json!({}));
        assert_eq!(item_no_priority.prefetch_priority(), None);
    }

    fn create_test_queue_item(metadata: serde_json::Value) -> QueueItem {
        QueueItem {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            track_id: Uuid::new_v4(),
            position: 0,
            source_type: None,
            source_id: None,
            added_at: Utc::now(),
            metadata,
        }
    }
}
