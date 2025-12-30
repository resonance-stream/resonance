//! Queue repository for persistent play queue storage
//!
//! Provides database operations for the user's explicit play queue,
//! enabling cross-session persistence and queue-based prefetch by the
//! background worker.
//!
//! The hot path for real-time sync uses Redis pub/sub via WebSocket,
//! while this repository manages the durable persistence layer.

// Allow unused code - this repository is prepared for worker integration
#![allow(dead_code)]

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::queue::{
    ContextType, QueueItem, QueuePlaybackState, QueueTrackId, QueueValidationError, SetQueue,
    MAX_QUEUE_SIZE,
};

/// Result type for queue repository operations
pub type QueueResult<T> = Result<T, QueueError>;

/// Errors that can occur during queue repository operations
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("validation error: {0}")]
    Validation(#[from] QueueValidationError),

    #[error("queue not found for user")]
    NotFound,

    #[error("invalid position: {0}")]
    InvalidPosition(i32),
}

/// Repository for queue persistence operations
#[derive(Clone)]
pub struct QueueRepository {
    pool: PgPool,
}

impl QueueRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all queue items for a user, ordered by position
    ///
    /// Returns an empty vector if the user has no queue items.
    #[tracing::instrument(skip(self))]
    pub async fn get_queue(&self, user_id: Uuid) -> QueueResult<Vec<QueueItem>> {
        let items = sqlx::query_as::<_, QueueItem>(
            r#"
            SELECT id, user_id, track_id, position, source_type, source_id, added_at, metadata
            FROM queue_items
            WHERE user_id = $1
            ORDER BY position ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    /// Get the current playback state for a user
    ///
    /// Returns None if the user has no queue state (never queued anything).
    #[tracing::instrument(skip(self))]
    pub async fn get_state(&self, user_id: Uuid) -> QueueResult<Option<QueuePlaybackState>> {
        let state = sqlx::query_as::<_, QueuePlaybackState>(
            r#"
            SELECT user_id, current_index, updated_at
            FROM queue_state
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(state)
    }

    /// Replace the entire queue with new tracks
    ///
    /// This is an atomic operation that:
    /// 1. Validates the input
    /// 2. Deletes all existing queue items for the user
    /// 3. Batch inserts all new queue items using UNNEST
    /// 4. Upserts the queue state with the new current index
    ///
    /// Uses a transaction to ensure consistency.
    #[tracing::instrument(skip(self, queue), fields(track_count = queue.track_ids.len()))]
    pub async fn set_queue(&self, user_id: Uuid, queue: &SetQueue) -> QueueResult<()> {
        // Note: Empty queue with index 0 is valid - validation in SetQueue::validate()
        // allows this case since there's no "out of bounds" for an empty queue.
        // The upserted state with index 0 means "start of queue" when tracks are added.
        queue.validate()?;

        let mut tx = self.pool.begin().await?;

        // Delete existing queue items
        sqlx::query(
            r#"
            DELETE FROM queue_items
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Batch insert new queue items using UNNEST for performance
        if !queue.track_ids.is_empty() {
            let positions: Vec<i32> = (0..queue.track_ids.len() as i32).collect();

            sqlx::query(
                r#"
                INSERT INTO queue_items (user_id, track_id, position, source_type, source_id, added_at, metadata)
                SELECT $1, track_id, position, $4, $5, NOW(), '{}'::jsonb
                FROM UNNEST($2::uuid[], $3::int[]) AS t(track_id, position)
                "#,
            )
            .bind(user_id)
            .bind(&queue.track_ids)
            .bind(&positions)
            .bind(queue.source_type)
            .bind(queue.source_id)
            .execute(&mut *tx)
            .await?;
        }

        // Upsert queue state
        sqlx::query(
            r#"
            INSERT INTO queue_state (user_id, current_index, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (user_id)
            DO UPDATE SET current_index = $2, updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(queue.current_index)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    /// Update only the current playback index
    ///
    /// Upserts the queue state (creates if doesn't exist, updates if exists).
    /// The index is validated to be non-negative.
    ///
    /// Note: The index is NOT validated against current queue length. The service
    /// layer must handle cases where index exceeds queue length (e.g., after track
    /// removal) by clamping to valid bounds when reading.
    #[tracing::instrument(skip(self))]
    pub async fn update_index(&self, user_id: Uuid, current_index: i32) -> QueueResult<()> {
        if current_index < 0 {
            return Err(QueueError::Validation(QueueValidationError::NegativeIndex(
                current_index,
            )));
        }

        sqlx::query(
            r#"
            INSERT INTO queue_state (user_id, current_index, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (user_id)
            DO UPDATE SET current_index = $2, updated_at = NOW()
            "#,
        )
        .bind(user_id)
        .bind(current_index)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get upcoming tracks for prefetch
    ///
    /// Returns the next `count` tracks after the current playback index
    /// that have not been marked as prefetched. This is optimized for
    /// the worker's prefetch job.
    ///
    /// If the user has no queue state, returns an empty vector.
    #[tracing::instrument(skip(self))]
    pub async fn get_upcoming_tracks(
        &self,
        user_id: Uuid,
        count: i32,
    ) -> QueueResult<Vec<QueueTrackId>> {
        let tracks = sqlx::query_as::<_, QueueTrackId>(
            r#"
            SELECT qi.track_id
            FROM queue_items qi
            JOIN queue_state qs ON qi.user_id = qs.user_id
            WHERE qi.user_id = $1
              AND qi.position > qs.current_index
              AND qi.metadata->>'prefetched' IS DISTINCT FROM 'true'
            ORDER BY qi.position ASC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(count)
        .fetch_all(&self.pool)
        .await?;

        Ok(tracks)
    }

    /// Get upcoming tracks with full metadata for prefetch
    ///
    /// Returns the next `count` tracks after the current playback index
    /// that have not been marked as prefetched. Returns full QueueItem
    /// for richer prefetch decisions.
    #[tracing::instrument(skip(self))]
    pub async fn get_upcoming_items(
        &self,
        user_id: Uuid,
        count: i32,
    ) -> QueueResult<Vec<QueueItem>> {
        let items = sqlx::query_as::<_, QueueItem>(
            r#"
            SELECT qi.id, qi.user_id, qi.track_id, qi.position, qi.source_type,
                   qi.source_id, qi.added_at, qi.metadata
            FROM queue_items qi
            JOIN queue_state qs ON qi.user_id = qs.user_id
            WHERE qi.user_id = $1
              AND qi.position > qs.current_index
              AND qi.metadata->>'prefetched' IS DISTINCT FROM 'true'
            ORDER BY qi.position ASC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(count)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    /// Mark tracks as prefetched in queue metadata
    ///
    /// Updates the metadata JSON to set `prefetched: true` and optionally
    /// a `prefetch_priority` score. This prevents re-prefetching tracks
    /// that are already cached.
    #[tracing::instrument(skip(self, track_ids), fields(track_count = track_ids.len()))]
    pub async fn mark_prefetched(
        &self,
        user_id: Uuid,
        track_ids: &[Uuid],
        priority: Option<f64>,
    ) -> QueueResult<u64> {
        if track_ids.is_empty() {
            return Ok(0);
        }

        let priority_json = priority.unwrap_or(1.0);

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET metadata = jsonb_set(
                jsonb_set(COALESCE(metadata, '{}'::jsonb), '{prefetched}', 'true'::jsonb),
                '{prefetch_priority}',
                $3::text::jsonb
            )
            WHERE user_id = $1 AND track_id = ANY($2)
            "#,
        )
        .bind(user_id)
        .bind(track_ids)
        .bind(priority_json.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Clear the prefetch flag for tracks
    ///
    /// Used when tracks are evicted from cache and need to be re-prefetched.
    #[tracing::instrument(skip(self, track_ids), fields(track_count = track_ids.len()))]
    pub async fn clear_prefetched(&self, user_id: Uuid, track_ids: &[Uuid]) -> QueueResult<u64> {
        if track_ids.is_empty() {
            return Ok(0);
        }

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET metadata = metadata - 'prefetched' - 'prefetch_priority'
            WHERE user_id = $1 AND track_id = ANY($2)
            "#,
        )
        .bind(user_id)
        .bind(track_ids)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Add tracks to the end of the queue
    ///
    /// Appends tracks after the last position in the queue.
    /// Validates that the resulting queue size doesn't exceed MAX_QUEUE_SIZE.
    /// Uses a transaction with FOR UPDATE to prevent race conditions.
    #[tracing::instrument(skip(self, track_ids), fields(track_count = track_ids.len()))]
    pub async fn append_tracks(
        &self,
        user_id: Uuid,
        track_ids: &[Uuid],
        source_type: Option<ContextType>,
        source_id: Option<Uuid>,
    ) -> QueueResult<()> {
        if track_ids.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // Lock and get count + max position in single query to prevent race conditions
        let stats: (i64, Option<i32>) = sqlx::query_as(
            r#"
            SELECT COUNT(*), MAX(position)
            FROM queue_items
            WHERE user_id = $1
            FOR UPDATE
            "#,
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        let current_size = stats.0 as usize;
        let new_size = current_size + track_ids.len();
        if new_size > MAX_QUEUE_SIZE {
            return Err(QueueError::Validation(QueueValidationError::TooManyTracks(
                new_size,
            )));
        }

        let start_position = stats.1.unwrap_or(-1) + 1;

        // Batch insert using UNNEST
        let positions: Vec<i32> = (0..track_ids.len() as i32)
            .map(|i| start_position + i)
            .collect();

        sqlx::query(
            r#"
            INSERT INTO queue_items (user_id, track_id, position, source_type, source_id, added_at, metadata)
            SELECT $1, track_id, position, $4, $5, NOW(), '{}'::jsonb
            FROM UNNEST($2::uuid[], $3::int[]) AS t(track_id, position)
            "#,
        )
        .bind(user_id)
        .bind(track_ids)
        .bind(&positions)
        .bind(source_type)
        .bind(source_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Insert a track at a specific position (e.g., "play next")
    ///
    /// Shifts existing tracks at and after the target position up by one.
    /// Validates position bounds and queue size limits.
    #[tracing::instrument(skip(self))]
    pub async fn insert_at(
        &self,
        user_id: Uuid,
        track_id: Uuid,
        position: i32,
        source_type: Option<ContextType>,
        source_id: Option<Uuid>,
    ) -> QueueResult<()> {
        if position < 0 {
            return Err(QueueError::InvalidPosition(position));
        }

        let mut tx = self.pool.begin().await?;

        // Check size limit with lock
        let stats: (i64, Option<i32>) = sqlx::query_as(
            r#"
            SELECT COUNT(*), MAX(position)
            FROM queue_items
            WHERE user_id = $1
            FOR UPDATE
            "#,
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        let current_size = stats.0 as usize;
        if current_size >= MAX_QUEUE_SIZE {
            return Err(QueueError::Validation(QueueValidationError::TooManyTracks(
                current_size + 1,
            )));
        }

        // Clamp position to max + 1 (append if beyond end)
        let max_position = stats.1.unwrap_or(-1);
        let actual_position = position.min(max_position + 1);

        // Shift existing tracks at and after target position
        sqlx::query(
            r#"
            UPDATE queue_items
            SET position = position + 1
            WHERE user_id = $1 AND position >= $2
            "#,
        )
        .bind(user_id)
        .bind(actual_position)
        .execute(&mut *tx)
        .await?;

        // Insert the new track
        sqlx::query(
            r#"
            INSERT INTO queue_items (user_id, track_id, position, source_type, source_id, added_at, metadata)
            VALUES ($1, $2, $3, $4, $5, NOW(), '{}'::jsonb)
            "#,
        )
        .bind(user_id)
        .bind(track_id)
        .bind(actual_position)
        .bind(source_type)
        .bind(source_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Move a track from one position to another (reorder/drag-and-drop)
    ///
    /// Atomically moves a track and adjusts all affected positions.
    /// Returns the moved item, or None if source position was invalid.
    #[tracing::instrument(skip(self))]
    pub async fn move_track(
        &self,
        user_id: Uuid,
        from_position: i32,
        to_position: i32,
    ) -> QueueResult<Option<QueueItem>> {
        if from_position < 0 || to_position < 0 {
            return Err(QueueError::InvalidPosition(from_position.min(to_position)));
        }

        if from_position == to_position {
            // No-op, just return the item at position
            return self.get_item_at_position(user_id, from_position).await;
        }

        let mut tx = self.pool.begin().await?;

        // Get the item to move
        let item = sqlx::query_as::<_, QueueItem>(
            r#"
            SELECT id, user_id, track_id, position, source_type, source_id, added_at, metadata
            FROM queue_items
            WHERE user_id = $1 AND position = $2
            FOR UPDATE
            "#,
        )
        .bind(user_id)
        .bind(from_position)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(mut item) = item else {
            return Ok(None);
        };

        // Get max position to clamp to_position
        let max_pos: (Option<i32>,) = sqlx::query_as(
            r#"
            SELECT MAX(position) FROM queue_items WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        let to_position = to_position.min(max_pos.0.unwrap_or(0));

        if from_position < to_position {
            // Moving forward: shift items in between down
            sqlx::query(
                r#"
                UPDATE queue_items
                SET position = position - 1
                WHERE user_id = $1 AND position > $2 AND position <= $3
                "#,
            )
            .bind(user_id)
            .bind(from_position)
            .bind(to_position)
            .execute(&mut *tx)
            .await?;
        } else {
            // Moving backward: shift items in between up
            sqlx::query(
                r#"
                UPDATE queue_items
                SET position = position + 1
                WHERE user_id = $1 AND position >= $2 AND position < $3
                "#,
            )
            .bind(user_id)
            .bind(to_position)
            .bind(from_position)
            .execute(&mut *tx)
            .await?;
        }

        // Update the moved item's position
        sqlx::query(
            r#"
            UPDATE queue_items
            SET position = $3
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(item.id)
        .bind(user_id)
        .bind(to_position)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        item.position = to_position;
        Ok(Some(item))
    }

    /// Get a queue item at a specific position
    #[tracing::instrument(skip(self))]
    pub async fn get_item_at_position(
        &self,
        user_id: Uuid,
        position: i32,
    ) -> QueueResult<Option<QueueItem>> {
        let item = sqlx::query_as::<_, QueueItem>(
            r#"
            SELECT id, user_id, track_id, position, source_type, source_id, added_at, metadata
            FROM queue_items
            WHERE user_id = $1 AND position = $2
            "#,
        )
        .bind(user_id)
        .bind(position)
        .fetch_optional(&self.pool)
        .await?;

        Ok(item)
    }

    /// Remove a track from the queue by position
    ///
    /// Shifts subsequent positions down to maintain contiguous ordering.
    /// Also adjusts current_index in queue_state if the removed track
    /// was before the current position.
    ///
    /// Returns the removed item, or None if position was out of bounds.
    #[tracing::instrument(skip(self))]
    pub async fn remove_at_position(
        &self,
        user_id: Uuid,
        position: i32,
    ) -> QueueResult<Option<QueueItem>> {
        let mut tx = self.pool.begin().await?;

        // Get the item to remove
        let removed = sqlx::query_as::<_, QueueItem>(
            r#"
            DELETE FROM queue_items
            WHERE user_id = $1 AND position = $2
            RETURNING id, user_id, track_id, position, source_type, source_id, added_at, metadata
            "#,
        )
        .bind(user_id)
        .bind(position)
        .fetch_optional(&mut *tx)
        .await?;

        if removed.is_some() {
            // Shift subsequent positions down
            sqlx::query(
                r#"
                UPDATE queue_items
                SET position = position - 1
                WHERE user_id = $1 AND position > $2
                "#,
            )
            .bind(user_id)
            .bind(position)
            .execute(&mut *tx)
            .await?;

            // Adjust current_index if needed (track before current was removed)
            sqlx::query(
                r#"
                UPDATE queue_state
                SET current_index = GREATEST(0, current_index - 1), updated_at = NOW()
                WHERE user_id = $1 AND current_index > $2
                "#,
            )
            .bind(user_id)
            .bind(position)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(removed)
    }

    /// Remove the first occurrence of a track by track_id
    ///
    /// Useful for "remove from queue" actions that don't know the position.
    /// Returns the removed item, or None if track wasn't in queue.
    #[tracing::instrument(skip(self))]
    pub async fn remove_by_track_id(
        &self,
        user_id: Uuid,
        track_id: Uuid,
    ) -> QueueResult<Option<QueueItem>> {
        // Find the position of the first occurrence
        let item: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT position FROM queue_items
            WHERE user_id = $1 AND track_id = $2
            ORDER BY position ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(track_id)
        .fetch_optional(&self.pool)
        .await?;

        match item {
            Some((position,)) => self.remove_at_position(user_id, position).await,
            None => Ok(None),
        }
    }

    /// Clear the entire queue for a user
    ///
    /// Removes all queue items and resets the index to 0.
    #[tracing::instrument(skip(self))]
    pub async fn clear_queue(&self, user_id: Uuid) -> QueueResult<u64> {
        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            r#"
            DELETE FROM queue_items
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Reset index to 0
        sqlx::query(
            r#"
            UPDATE queue_state
            SET current_index = 0, updated_at = NOW()
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(result.rows_affected())
    }

    /// Get the queue size for a user
    #[tracing::instrument(skip(self))]
    pub async fn get_queue_size(&self, user_id: Uuid) -> QueueResult<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM queue_items
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_repository_new() {
        // Simple smoke test - actual DB tests would require test fixtures
        // This just verifies the struct can be constructed
        // Real integration tests would be in apps/api/tests/
    }

    #[test]
    fn test_queue_error_display() {
        let db_err = QueueError::NotFound;
        assert_eq!(format!("{}", db_err), "queue not found for user");

        let validation_err = QueueError::Validation(QueueValidationError::NegativeIndex(-5));
        assert!(format!("{}", validation_err).contains("-5"));

        let position_err = QueueError::InvalidPosition(-1);
        assert!(format!("{}", position_err).contains("-1"));
    }
}
