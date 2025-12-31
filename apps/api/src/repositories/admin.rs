//! Admin repository for system statistics and user management
//!
//! This module provides admin-only database operations including:
//! - System statistics aggregation
//! - User listing with pagination and search
//! - Role management
//! - Session invalidation

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::models::user::UserRole;

/// System-wide statistics for the admin dashboard
#[derive(Debug, FromRow)]
pub struct SystemStats {
    pub user_count: i64,
    pub track_count: i64,
    pub album_count: i64,
    pub artist_count: i64,
    pub total_duration_ms: i64,
    pub total_file_size_bytes: i64,
    pub active_session_count: i64,
}

/// User summary for admin listing
#[derive(Debug, FromRow)]
pub struct AdminUserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub email_verified: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub session_count: i64,
}

/// Session info for admin user detail view
#[derive(Debug, FromRow)]
pub struct AdminSessionRow {
    pub id: Uuid,
    pub device_type: Option<String>,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub is_active: bool,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Repository for admin database operations
#[derive(Clone)]
pub struct AdminRepository {
    pool: PgPool,
}

impl AdminRepository {
    /// Create a new AdminRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get system-wide statistics
    pub async fn get_system_stats(&self) -> Result<SystemStats, sqlx::Error> {
        sqlx::query_as::<_, SystemStats>(
            r#"
            SELECT
                (SELECT COUNT(*) FROM users) as user_count,
                (SELECT COUNT(*) FROM tracks) as track_count,
                (SELECT COUNT(*) FROM albums) as album_count,
                (SELECT COUNT(*) FROM artists) as artist_count,
                (SELECT COALESCE(SUM(duration_ms), 0) FROM tracks) as total_duration_ms,
                (SELECT COALESCE(SUM(file_size), 0) FROM tracks) as total_file_size_bytes,
                (SELECT COUNT(*) FROM sessions WHERE is_active = true AND expires_at > NOW()) as active_session_count
            "#,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Find a user by ID with session count
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user to find
    ///
    /// # Returns
    /// * `Ok(Some(AdminUserRow))` - If the user exists
    /// * `Ok(None)` - If no user with the given ID exists
    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<AdminUserRow>, sqlx::Error> {
        sqlx::query_as::<_, AdminUserRow>(
            r#"
            SELECT
                u.id,
                u.email,
                u.display_name,
                u.avatar_url,
                u.role,
                u.email_verified,
                u.last_seen_at,
                u.created_at,
                COALESCE(s.session_count, 0) as session_count
            FROM users u
            LEFT JOIN (
                SELECT user_id, COUNT(*) as session_count
                FROM sessions
                WHERE is_active = true AND expires_at > NOW()
                GROUP BY user_id
            ) s ON s.user_id = u.id
            WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List all users with pagination and optional search
    ///
    /// Uses a LEFT JOIN for session counts to avoid N+1 query pattern.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of users to return
    /// * `offset` - Number of users to skip
    /// * `search` - Optional search query for email or display_name
    pub async fn list_users(
        &self,
        limit: i64,
        offset: i64,
        search: Option<&str>,
    ) -> Result<Vec<AdminUserRow>, sqlx::Error> {
        match search {
            Some(query) if !query.trim().is_empty() => {
                let search_pattern = format!("%{}%", query.trim().to_lowercase());
                sqlx::query_as::<_, AdminUserRow>(
                    r#"
                    SELECT
                        u.id,
                        u.email,
                        u.display_name,
                        u.avatar_url,
                        u.role,
                        u.email_verified,
                        u.last_seen_at,
                        u.created_at,
                        COALESCE(s.session_count, 0) as session_count
                    FROM users u
                    LEFT JOIN (
                        SELECT user_id, COUNT(*) as session_count
                        FROM sessions
                        WHERE is_active = true AND expires_at > NOW()
                        GROUP BY user_id
                    ) s ON s.user_id = u.id
                    WHERE LOWER(u.email) LIKE $1 OR LOWER(u.display_name) LIKE $1
                    ORDER BY u.created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(&search_pattern)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            _ => {
                sqlx::query_as::<_, AdminUserRow>(
                    r#"
                    SELECT
                        u.id,
                        u.email,
                        u.display_name,
                        u.avatar_url,
                        u.role,
                        u.email_verified,
                        u.last_seen_at,
                        u.created_at,
                        COALESCE(s.session_count, 0) as session_count
                    FROM users u
                    LEFT JOIN (
                        SELECT user_id, COUNT(*) as session_count
                        FROM sessions
                        WHERE is_active = true AND expires_at > NOW()
                        GROUP BY user_id
                    ) s ON s.user_id = u.id
                    ORDER BY u.created_at DESC
                    LIMIT $1 OFFSET $2
                    "#,
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }
    }

    /// Get total count of users (for pagination)
    pub async fn count_users(&self, search: Option<&str>) -> Result<i64, sqlx::Error> {
        match search {
            Some(query) if !query.trim().is_empty() => {
                let search_pattern = format!("%{}%", query.trim().to_lowercase());
                sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*) FROM users
                    WHERE LOWER(email) LIKE $1 OR LOWER(display_name) LIKE $1
                    "#,
                )
                .bind(&search_pattern)
                .fetch_one(&self.pool)
                .await
            }
            _ => {
                sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(&self.pool)
                    .await
            }
        }
    }

    /// Get user sessions for admin detail view
    pub async fn get_user_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<AdminSessionRow>, sqlx::Error> {
        sqlx::query_as::<_, AdminSessionRow>(
            r#"
            SELECT
                id,
                device_type,
                device_name,
                ip_address::text as ip_address,
                user_agent,
                is_active,
                last_active_at,
                created_at
            FROM sessions
            WHERE user_id = $1
            ORDER BY last_active_at DESC
            LIMIT 50
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a user's role
    ///
    /// # Arguments
    /// * `user_id` - The user to update
    /// * `new_role` - The new role to assign
    ///
    /// # Returns
    /// * `Ok(true)` - If the role was updated
    /// * `Ok(false)` - If no user was found
    pub async fn update_user_role(
        &self,
        user_id: Uuid,
        new_role: UserRole,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET role = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(new_role)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Count the number of admin users
    ///
    /// Used to prevent deleting or demoting the last admin.
    pub async fn count_admins(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
            .fetch_one(&self.pool)
            .await
    }

    /// Delete a user account
    ///
    /// This invalidates all sessions and removes the user from the database.
    /// The operation is wrapped in a transaction for consistency.
    ///
    /// # Arguments
    /// * `user_id` - The user to delete
    ///
    /// # Returns
    /// * `Ok(true)` - If the user was deleted
    /// * `Ok(false)` - If no user was found
    pub async fn delete_user(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // First invalidate all sessions
        sqlx::query("UPDATE sessions SET is_active = false WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Delete the user
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(result.rows_affected() > 0)
    }

    /// Invalidate all sessions for a user
    ///
    /// # Arguments
    /// * `user_id` - The user whose sessions to invalidate
    ///
    /// # Returns
    /// Number of sessions invalidated
    pub async fn invalidate_user_sessions(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE sessions SET is_active = false WHERE user_id = $1 AND is_active = true",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_stats_fields() {
        // Compile-time test to ensure struct fields match query
        let _stats = SystemStats {
            user_count: 0,
            track_count: 0,
            album_count: 0,
            artist_count: 0,
            total_duration_ms: 0,
            total_file_size_bytes: 0,
            active_session_count: 0,
        };
    }
}
