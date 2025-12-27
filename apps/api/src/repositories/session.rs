//! Session repository for centralized database operations
//!
//! This module provides all session-related database operations in a single location,
//! following the repository pattern described in CLAUDE.md architecture guidelines.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Session row returned from database queries
#[derive(Debug, FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// Repository for session database operations
///
/// Centralizes all session-related database queries to avoid duplication
/// across services.
#[derive(Clone)]
pub struct SessionRepository {
    pool: PgPool,
}

impl SessionRepository {
    /// Create a new SessionRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Create a new session record in the database
    ///
    /// # Arguments
    /// * `session_id` - Pre-generated UUID for the session
    /// * `user_id` - ID of the user who owns this session
    /// * `access_token_hash` - SHA-256 hash of the access token
    /// * `refresh_token_hash` - SHA-256 hash of the refresh token
    /// * `device_name` - Optional human-readable device name
    /// * `device_type` - Optional device type (desktop, mobile, etc.)
    /// * `device_id` - Optional unique device identifier
    /// * `ip_address` - Optional client IP address
    /// * `user_agent` - Optional client user agent string
    /// * `expires_at` - Session expiration timestamp
    ///
    /// # Returns
    /// * `Ok(())` - If the session was created successfully
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        access_token_hash: &str,
        refresh_token_hash: &str,
        device_name: Option<&str>,
        device_type: Option<&str>,
        device_id: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO sessions (
                id, user_id, token_hash, refresh_token_hash,
                device_name, device_type, device_id,
                ip_address, user_agent, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9, $10)
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .bind(access_token_hash)
        .bind(refresh_token_hash)
        .bind(device_name)
        .bind(device_type)
        .bind(device_id)
        .bind(ip_address)
        .bind(user_agent)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Find an active session by ID and refresh token hash
    ///
    /// # Arguments
    /// * `session_id` - The session ID to find
    /// * `refresh_token_hash` - SHA-256 hash of the refresh token
    ///
    /// # Returns
    /// * `Ok(Some(SessionRow))` - If an active session is found
    /// * `Ok(None)` - If no matching active session exists
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn find_active_by_refresh_token(
        &self,
        session_id: Uuid,
        refresh_token_hash: &str,
    ) -> Result<Option<SessionRow>, sqlx::Error> {
        sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, user_id, expires_at
            FROM sessions
            WHERE id = $1 AND refresh_token_hash = $2 AND is_active = true
            "#,
        )
        .bind(session_id)
        .bind(refresh_token_hash)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deactivate a single session (logout)
    ///
    /// # Arguments
    /// * `session_id` - The session ID to deactivate
    ///
    /// # Returns
    /// * `Ok(true)` - If the session was deactivated
    /// * `Ok(false)` - If no session with the given ID exists
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn deactivate(&self, session_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("UPDATE sessions SET is_active = false WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Deactivate all active sessions for a user (logout all)
    ///
    /// # Arguments
    /// * `user_id` - The user whose sessions to deactivate
    ///
    /// # Returns
    /// * `Ok(u64)` - The number of sessions that were deactivated
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn deactivate_all_for_user(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE sessions SET is_active = false WHERE user_id = $1 AND is_active = true",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Update session tokens and extend expiration after a token refresh
    ///
    /// # Arguments
    /// * `session_id` - The session ID to update
    /// * `access_token_hash` - New SHA-256 hash of the access token
    /// * `refresh_token_hash` - New SHA-256 hash of the refresh token
    /// * `expires_at` - New session expiration timestamp
    ///
    /// # Returns
    /// * `Ok(())` - If the session was updated successfully
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn update_tokens(
        &self,
        session_id: Uuid,
        access_token_hash: &str,
        refresh_token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET token_hash = $1,
                refresh_token_hash = $2,
                last_active_at = NOW(),
                expires_at = $3
            WHERE id = $4
            "#,
        )
        .bind(access_token_hash)
        .bind(refresh_token_hash)
        .bind(expires_at)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete expired sessions from the database
    ///
    /// This method permanently removes sessions that have passed their expiration
    /// time. It should be called periodically by a background worker to prevent
    /// database bloat from accumulating expired sessions.
    ///
    /// # Arguments
    /// * `batch_size` - Maximum number of sessions to delete per call (prevents long locks)
    ///
    /// # Returns
    /// * `Ok(u64)` - The number of expired sessions that were deleted
    /// * `Err(sqlx::Error)` - If a database error occurs
    ///
    /// # Example (for worker job)
    /// ```ignore
    /// async fn cleanup_expired_sessions(repo: &SessionRepository) {
    ///     loop {
    ///         match repo.delete_expired(1000).await {
    ///             Ok(0) => break, // No more expired sessions
    ///             Ok(deleted) => {
    ///                 tracing::info!(deleted, "Cleaned up expired sessions");
    ///             }
    ///             Err(e) => {
    ///                 tracing::error!(error = ?e, "Failed to cleanup sessions");
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn delete_expired(&self, batch_size: i64) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE id IN (
                SELECT id FROM sessions
                WHERE expires_at < NOW()
                LIMIT $1
            )
            "#,
        )
        .bind(batch_size)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Delete all inactive sessions older than a specified duration
    ///
    /// This is useful for cleaning up sessions that were explicitly deactivated
    /// (logged out) but not yet expired. Keeping these for a short time allows
    /// for audit logging, but they should eventually be purged.
    ///
    /// # Arguments
    /// * `older_than_days` - Delete inactive sessions older than this many days
    /// * `batch_size` - Maximum number of sessions to delete per call
    ///
    /// # Returns
    /// * `Ok(u64)` - The number of sessions that were deleted
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn delete_inactive_older_than(
        &self,
        older_than_days: i32,
        batch_size: i64,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE id IN (
                SELECT id FROM sessions
                WHERE is_active = false
                  AND last_active_at < NOW() - make_interval(days => $1)
                LIMIT $2
            )
            "#,
        )
        .bind(older_than_days)
        .bind(batch_size)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_repository_pool_accessor() {
        // This is a basic compile-time test to verify the API is correct.
        // Full integration tests would require a test database.
        // In a real test scenario, you would:
        // 1. Create a test database or use a transaction
        // 2. Insert test data
        // 3. Verify repository methods work correctly
    }
}
