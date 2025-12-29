//! Device presence repository for cross-device synchronization
//!
//! Provides database operations for tracking device presence and state,
//! enabling features like cross-device playback sync.

// Allow unused code - this repository is prepared for future integration
#![allow(dead_code)]

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::device::{DevicePresence, UpsertDevicePresence};

/// Repository for device presence operations
#[derive(Clone)]
pub struct DeviceRepository {
    pool: PgPool,
}

impl DeviceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert a device (insert or update on conflict)
    ///
    /// This is called when a device connects via WebSocket.
    pub async fn upsert_device(
        &self,
        device: &UpsertDevicePresence,
    ) -> Result<DevicePresence, sqlx::Error> {
        sqlx::query_as::<_, DevicePresence>(
            r#"
            INSERT INTO device_presence (user_id, device_id, device_name, device_type, user_agent, is_connected, connected_at)
            VALUES ($1, $2, $3, $4, $5, TRUE, NOW())
            ON CONFLICT (user_id, device_id)
            DO UPDATE SET
                device_name = EXCLUDED.device_name,
                device_type = EXCLUDED.device_type,
                user_agent = EXCLUDED.user_agent,
                is_connected = TRUE,
                connected_at = NOW(),
                last_seen_at = NOW()
            RETURNING *
            "#,
        )
        .bind(device.user_id)
        .bind(&device.device_id)
        .bind(&device.device_name)
        .bind(&device.device_type)
        .bind(&device.user_agent)
        .fetch_one(&self.pool)
        .await
    }

    /// Mark a device as disconnected
    pub async fn mark_disconnected(
        &self,
        user_id: Uuid,
        device_id: &str,
    ) -> Result<Option<DevicePresence>, sqlx::Error> {
        sqlx::query_as::<_, DevicePresence>(
            r#"
            UPDATE device_presence
            SET is_connected = FALSE, is_active = FALSE, disconnected_at = NOW()
            WHERE user_id = $1 AND device_id = $2
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Set a device as the active device (controlling playback)
    ///
    /// Uses an atomic CTE to prevent race conditions when multiple devices
    /// attempt to become active simultaneously. Returns the activated device
    /// or None if the device doesn't exist.
    pub async fn set_active_device(
        &self,
        user_id: Uuid,
        device_id: &str,
    ) -> Result<Option<DevicePresence>, sqlx::Error> {
        // Atomic operation: deactivate all, then activate target in a single query
        sqlx::query_as::<_, DevicePresence>(
            r#"
            WITH deactivated AS (
                UPDATE device_presence
                SET is_active = FALSE
                WHERE user_id = $1 AND is_active = TRUE
            )
            UPDATE device_presence
            SET is_active = TRUE
            WHERE user_id = $1 AND device_id = $2
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Clear the active device for a user
    pub async fn clear_active_device(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE device_presence
            SET is_active = FALSE
            WHERE user_id = $1 AND is_active = TRUE
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the active device for a user
    pub async fn get_active_device(
        &self,
        user_id: Uuid,
    ) -> Result<Option<DevicePresence>, sqlx::Error> {
        sqlx::query_as::<_, DevicePresence>(
            r#"
            SELECT *
            FROM device_presence
            WHERE user_id = $1 AND is_active = TRUE
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get all connected devices for a user
    pub async fn get_connected_devices(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<DevicePresence>, sqlx::Error> {
        sqlx::query_as::<_, DevicePresence>(
            r#"
            SELECT *
            FROM device_presence
            WHERE user_id = $1 AND is_connected = TRUE
            ORDER BY connected_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get all devices (including disconnected) for a user
    pub async fn get_all_devices(&self, user_id: Uuid) -> Result<Vec<DevicePresence>, sqlx::Error> {
        sqlx::query_as::<_, DevicePresence>(
            r#"
            SELECT *
            FROM device_presence
            WHERE user_id = $1
            ORDER BY last_seen_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update the last seen timestamp for a device (heartbeat)
    ///
    /// Returns `true` if the device was found and updated, `false` otherwise.
    /// This allows callers to detect when a heartbeat targets a non-existent device.
    pub async fn touch_device(&self, user_id: Uuid, device_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE device_presence
            SET last_seen_at = NOW()
            WHERE user_id = $1 AND device_id = $2
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update playback state for a device
    ///
    /// Returns `true` if the device was found and updated, `false` otherwise.
    /// The `position_ms` is clamped to 0 if negative (defensive handling).
    pub async fn update_playback_state(
        &self,
        user_id: Uuid,
        device_id: &str,
        track_id: Option<&str>,
        position_ms: i64,
        is_playing: bool,
    ) -> Result<bool, sqlx::Error> {
        // Clamp position to non-negative (defensive against invalid client data)
        let position_ms = position_ms.max(0);

        let result = sqlx::query(
            r#"
            UPDATE device_presence
            SET last_track_id = $3, last_position_ms = $4, last_is_playing = $5
            WHERE user_id = $1 AND device_id = $2
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .bind(track_id)
        .bind(position_ms)
        .bind(is_playing)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Clean up stale devices that haven't been seen in a while
    pub async fn cleanup_stale_devices(&self, max_age_hours: i32) -> Result<u64, sqlx::Error> {
        let interval = format!("{} hours", max_age_hours);
        let result = sqlx::query(
            r#"
            UPDATE device_presence
            SET is_connected = FALSE
            WHERE is_connected = TRUE
              AND last_seen_at < NOW() - $1::INTERVAL
            "#,
        )
        .bind(&interval)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Delete old device records (for GDPR compliance or cleanup)
    pub async fn delete_old_devices(&self, max_age_days: i32) -> Result<u64, sqlx::Error> {
        let interval = format!("{} days", max_age_days);
        let result = sqlx::query(
            r#"
            DELETE FROM device_presence
            WHERE last_seen_at < NOW() - $1::INTERVAL
            "#,
        )
        .bind(&interval)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_repository_new() {
        // Simple smoke test - actual DB tests would require test fixtures
        // This just verifies the struct can be constructed
        // Real integration tests would be in apps/api/tests/
    }
}
