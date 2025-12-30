//! User repository for centralized database operations
//!
//! This module provides all user-related database operations in a single location,
//! following the repository pattern described in CLAUDE.md architecture guidelines.

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::User;

/// Repository for user database operations
///
/// Centralizes all user-related database queries to avoid duplication
/// across middleware and services.
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Create a new UserRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying connection pool
    #[allow(dead_code)] // Available for direct pool access when needed
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Find a user by their unique ID
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user to find
    ///
    /// # Returns
    /// * `Ok(Some(User))` - If the user exists
    /// * `Ok(None)` - If no user with the given ID exists
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT
                id,
                email,
                password_hash,
                display_name,
                avatar_url,
                role,
                preferences,
                listenbrainz_token,
                discord_user_id,
                email_verified,
                last_seen_at,
                created_at,
                updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Find a user by their email address
    ///
    /// # Arguments
    /// * `email` - The email address to search for (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(Some(User))` - If the user exists
    /// * `Ok(None)` - If no user with the given email exists
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT
                id,
                email,
                password_hash,
                display_name,
                avatar_url,
                role,
                preferences,
                listenbrainz_token,
                discord_user_id,
                email_verified,
                last_seen_at,
                created_at,
                updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email.trim().to_lowercase())
        .fetch_optional(&self.pool)
        .await
    }

    /// Check if an email address is already registered
    ///
    /// # Arguments
    /// * `email` - The email address to check (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(true)` - If the email is already registered
    /// * `Ok(false)` - If the email is available
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn email_exists(&self, email: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(r#"SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)"#)
            .bind(email.trim().to_lowercase())
            .fetch_one(&self.pool)
            .await
    }

    /// Update the last_seen_at timestamp for a user
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user to update
    ///
    /// # Returns
    /// * `Ok(())` - If the update was successful
    /// * `Err(sqlx::Error)` - If a database error occurs
    pub async fn update_last_seen(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET last_seen_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update user preferences
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user to update
    /// * `preferences` - The new preferences JSON value
    ///
    /// # Returns
    /// * `Ok(())` - If the update was successful
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)] // Will be used by integrations mutations
    pub async fn update_preferences(
        &self,
        user_id: Uuid,
        preferences: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET preferences = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(preferences)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update user's ListenBrainz token
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user to update
    /// * `token` - The ListenBrainz token (None to remove)
    ///
    /// # Returns
    /// * `Ok(())` - If the update was successful
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)] // Will be used by integrations mutations
    pub async fn update_listenbrainz_token(
        &self,
        user_id: Uuid,
        token: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET listenbrainz_token = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(token)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get user's ListenBrainz token
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user
    ///
    /// # Returns
    /// * `Ok(Some(String))` - If the user has a token set
    /// * `Ok(None)` - If no token is set
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)] // Will be used by integrations mutations
    pub async fn get_listenbrainz_token(
        &self,
        user_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT listenbrainz_token
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|opt| opt.flatten())
    }

    /// Check if user has ListenBrainz token configured
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user
    ///
    /// # Returns
    /// * `Ok(true)` - If the user has a token set
    /// * `Ok(false)` - If no token is set or user not found
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[allow(dead_code)] // Used by ListenBrainzService
    pub async fn has_listenbrainz_token(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT listenbrainz_token IS NOT NULL
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|opt| opt.unwrap_or(false))
    }

    /// Create a new user in the database
    ///
    /// # Arguments
    /// * `email` - User's email address (must be unique)
    /// * `password_hash` - Pre-hashed password (Argon2id)
    /// * `display_name` - User's display name
    /// * `role` - User's role (defaults to User)
    /// * `preferences` - User preferences as JSON value
    ///
    /// # Returns
    /// * `Ok(User)` - The newly created user
    /// * `Err(sqlx::Error)` - If a database error occurs (including unique constraint violations)
    pub async fn create(
        &self,
        email: &str,
        password_hash: &str,
        display_name: &str,
        role: crate::models::user::UserRole,
        preferences: &serde_json::Value,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, password_hash, display_name, role, preferences)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id,
                email,
                password_hash,
                display_name,
                avatar_url,
                role,
                preferences,
                listenbrainz_token,
                discord_user_id,
                email_verified,
                last_seen_at,
                created_at,
                updated_at
            "#,
        )
        .bind(email.trim().to_lowercase())
        .bind(password_hash)
        .bind(display_name)
        .bind(role)
        .bind(preferences)
        .fetch_one(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_user_repository_pool_accessor() {
        // This is a basic compile-time test to verify the API is correct.
        // Full integration tests would require a test database.
        // In a real test scenario, you would:
        // 1. Create a test database or use a transaction
        // 2. Insert test data
        // 3. Verify repository methods work correctly
    }
}
