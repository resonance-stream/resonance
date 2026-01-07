//! System settings repository for centralized database operations
//!
//! This module provides all system settings, setup status, and user library path
//! database operations following the repository pattern.

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::system_settings::{
    ServiceType, SetupStatus, SystemSetting, SystemSettingInput, UserLibraryPath,
};

/// Repository for system settings database operations
///
/// Centralizes all system settings, setup status, and user library path
/// database queries to avoid duplication across services.
#[derive(Clone)]
#[allow(dead_code)] // Infrastructure for setup wizard and admin settings
pub struct SystemSettingsRepository {
    pool: PgPool,
}

#[allow(dead_code)] // Infrastructure for setup wizard and admin settings
impl SystemSettingsRepository {
    /// Create a new SystemSettingsRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying connection pool
    #[allow(dead_code)]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ========================================================================
    // System Settings Operations
    // ========================================================================

    /// Get system setting by service type
    ///
    /// # Arguments
    /// * `service` - The service type to look up
    ///
    /// # Returns
    /// * `Ok(Some(SystemSetting))` - If the setting exists
    /// * `Ok(None)` - If no setting for this service exists
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn get_by_service(
        &self,
        service: ServiceType,
    ) -> Result<Option<SystemSetting>, sqlx::Error> {
        sqlx::query_as::<_, SystemSetting>(
            r#"
            SELECT
                id,
                service,
                enabled,
                config,
                encrypted_secrets,
                last_connection_test,
                connection_healthy,
                connection_error,
                updated_by,
                created_at,
                updated_at
            FROM system_settings
            WHERE service = $1
            "#,
        )
        .bind(service)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get all system settings
    ///
    /// # Returns
    /// * `Ok(Vec<SystemSetting>)` - All configured system settings
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn get_all(&self) -> Result<Vec<SystemSetting>, sqlx::Error> {
        sqlx::query_as::<_, SystemSetting>(
            r#"
            SELECT
                id,
                service,
                enabled,
                config,
                encrypted_secrets,
                last_connection_test,
                connection_healthy,
                connection_error,
                updated_by,
                created_at,
                updated_at
            FROM system_settings
            ORDER BY service
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Get all enabled system settings
    ///
    /// # Returns
    /// * `Ok(Vec<SystemSetting>)` - All enabled system settings
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)]
    pub async fn get_enabled(&self) -> Result<Vec<SystemSetting>, sqlx::Error> {
        sqlx::query_as::<_, SystemSetting>(
            r#"
            SELECT
                id,
                service,
                enabled,
                config,
                encrypted_secrets,
                last_connection_test,
                connection_healthy,
                connection_error,
                updated_by,
                created_at,
                updated_at
            FROM system_settings
            WHERE enabled = true
            ORDER BY service
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Insert or update a system setting
    ///
    /// # Arguments
    /// * `input` - The setting input data
    /// * `updated_by` - The user ID performing the update
    ///
    /// # Returns
    /// * `Ok(SystemSetting)` - The created or updated setting
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn upsert(
        &self,
        input: &SystemSettingInput,
        updated_by: Uuid,
    ) -> Result<SystemSetting, sqlx::Error> {
        sqlx::query_as::<_, SystemSetting>(
            r#"
            INSERT INTO system_settings (service, enabled, config, encrypted_secrets, updated_by)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (service) DO UPDATE SET
                enabled = EXCLUDED.enabled,
                config = EXCLUDED.config,
                encrypted_secrets = EXCLUDED.encrypted_secrets,
                updated_by = EXCLUDED.updated_by,
                updated_at = NOW()
            RETURNING
                id,
                service,
                enabled,
                config,
                encrypted_secrets,
                last_connection_test,
                connection_healthy,
                connection_error,
                updated_by,
                created_at,
                updated_at
            "#,
        )
        .bind(input.service)
        .bind(input.enabled)
        .bind(&input.config)
        .bind(&input.encrypted_secrets)
        .bind(updated_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Update the health status of a service
    ///
    /// # Arguments
    /// * `service` - The service type to update
    /// * `healthy` - Whether the connection is healthy
    /// * `error` - Optional error message if unhealthy
    ///
    /// # Returns
    /// * `Ok(())` - If the update was successful
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn update_health(
        &self,
        service: ServiceType,
        healthy: bool,
        error: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE system_settings
            SET
                last_connection_test = NOW(),
                connection_healthy = $2,
                connection_error = $3
            WHERE service = $1
            "#,
        )
        .bind(service)
        .bind(healthy)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a system setting
    ///
    /// # Arguments
    /// * `service` - The service type to delete
    ///
    /// # Returns
    /// * `Ok(true)` - If the setting was deleted
    /// * `Ok(false)` - If no setting with that service existed
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)]
    pub async fn delete(&self, service: ServiceType) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM system_settings
            WHERE service = $1
            "#,
        )
        .bind(service)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Setup Status Operations
    // ========================================================================

    /// Check if first-run setup has been completed
    ///
    /// # Returns
    /// * `Ok(true)` - If setup is complete
    /// * `Ok(false)` - If setup is not complete or status doesn't exist
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn is_setup_complete(&self) -> Result<bool, sqlx::Error> {
        let result: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT completed FROM setup_status WHERE id = 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.unwrap_or(false))
    }

    /// Get full setup status
    ///
    /// # Returns
    /// * `Ok(Some(SetupStatus))` - The setup status record
    /// * `Ok(None)` - If no setup status exists
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)]
    pub async fn get_setup_status(&self) -> Result<Option<SetupStatus>, sqlx::Error> {
        sqlx::query_as::<_, SetupStatus>(
            r#"
            SELECT id, completed, completed_at, completed_by
            FROM setup_status
            WHERE id = 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Mark first-run setup as complete
    ///
    /// # Arguments
    /// * `user_id` - The user who completed setup
    ///
    /// # Returns
    /// * `Ok(())` - If the update was successful
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn mark_setup_complete(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE setup_status
            SET completed = true, completed_at = NOW(), completed_by = $1
            WHERE id = 1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Reset setup status (for testing or re-configuration)
    ///
    /// # Returns
    /// * `Ok(())` - If the reset was successful
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)]
    pub async fn reset_setup_status(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE setup_status
            SET completed = false, completed_at = NULL, completed_by = NULL
            WHERE id = 1
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ========================================================================
    // User Library Paths Operations
    // ========================================================================

    /// Get all library paths for a user
    ///
    /// # Arguments
    /// * `user_id` - The user whose paths to retrieve
    ///
    /// # Returns
    /// * `Ok(Vec<UserLibraryPath>)` - The user's library paths
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn get_user_library_paths(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserLibraryPath>, sqlx::Error> {
        sqlx::query_as::<_, UserLibraryPath>(
            r#"
            SELECT id, user_id, path, label, is_primary, created_at
            FROM user_library_paths
            WHERE user_id = $1
            ORDER BY is_primary DESC, created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get the primary library path for a user
    ///
    /// # Arguments
    /// * `user_id` - The user whose primary path to retrieve
    ///
    /// # Returns
    /// * `Ok(Some(UserLibraryPath))` - The user's primary library path
    /// * `Ok(None)` - If the user has no primary path
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)]
    pub async fn get_primary_library_path(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserLibraryPath>, sqlx::Error> {
        sqlx::query_as::<_, UserLibraryPath>(
            r#"
            SELECT id, user_id, path, label, is_primary, created_at
            FROM user_library_paths
            WHERE user_id = $1 AND is_primary = true
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Add a new library path for a user
    ///
    /// # Arguments
    /// * `user_id` - The user to add the path for
    /// * `path` - The file system path
    /// * `label` - Optional user-friendly label
    ///
    /// # Returns
    /// * `Ok(UserLibraryPath)` - The newly created path
    /// * `Err(sqlx::Error)` - If a database error occurs (including unique constraint violations)
    pub async fn add_user_library_path(
        &self,
        user_id: Uuid,
        path: &str,
        label: Option<&str>,
    ) -> Result<UserLibraryPath, sqlx::Error> {
        // Check if user has any paths - if not, make this one primary
        let existing_count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM user_library_paths WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let is_primary = existing_count == 0;

        sqlx::query_as::<_, UserLibraryPath>(
            r#"
            INSERT INTO user_library_paths (user_id, path, label, is_primary)
            VALUES ($1, $2, $3, $4)
            RETURNING id, user_id, path, label, is_primary, created_at
            "#,
        )
        .bind(user_id)
        .bind(path)
        .bind(label)
        .bind(is_primary)
        .fetch_one(&self.pool)
        .await
    }

    /// Remove a library path for a user
    ///
    /// # Arguments
    /// * `user_id` - The user whose path to remove
    /// * `path_id` - The ID of the path to remove
    ///
    /// # Returns
    /// * `Ok(true)` - If the path was removed
    /// * `Ok(false)` - If no matching path was found
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn remove_user_library_path(
        &self,
        user_id: Uuid,
        path_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM user_library_paths
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(path_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Set a library path as the primary path for a user
    /// This will unset any existing primary path first
    ///
    /// # Arguments
    /// * `user_id` - The user whose primary path to set
    /// * `path_id` - The ID of the path to make primary
    ///
    /// # Returns
    /// * `Ok(UserLibraryPath)` - The updated path
    /// * `Err(sqlx::Error)` - If a database error occurs or path not found
    pub async fn set_primary_library_path(
        &self,
        user_id: Uuid,
        path_id: Uuid,
    ) -> Result<UserLibraryPath, sqlx::Error> {
        // Use a transaction to ensure atomicity
        let mut tx = self.pool.begin().await?;

        // Unset any existing primary path
        sqlx::query(
            r#"
            UPDATE user_library_paths
            SET is_primary = false
            WHERE user_id = $1 AND is_primary = true
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Set the new primary path
        let path = sqlx::query_as::<_, UserLibraryPath>(
            r#"
            UPDATE user_library_paths
            SET is_primary = true
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, path, label, is_primary, created_at
            "#,
        )
        .bind(path_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(path)
    }

    /// Update the label of a library path
    ///
    /// # Arguments
    /// * `user_id` - The user whose path to update
    /// * `path_id` - The ID of the path to update
    /// * `label` - The new label (None to clear)
    ///
    /// # Returns
    /// * `Ok(true)` - If the path was updated
    /// * `Ok(false)` - If no matching path was found
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)]
    pub async fn update_library_path_label(
        &self,
        user_id: Uuid,
        path_id: Uuid,
        label: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE user_library_paths
            SET label = $3
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(path_id)
        .bind(user_id)
        .bind(label)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_settings_repository_creation() {
        // Compile-time test to verify the API is correct.
        // Full integration tests require a test database.
    }
}
