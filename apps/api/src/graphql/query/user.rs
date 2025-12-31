//! User queries for Resonance GraphQL API
//!
//! This module provides queries for user data:
//! - me: Get the currently authenticated user

use async_graphql::{Context, Object, Result};
use sqlx::PgPool;

use crate::graphql::types::User;
use crate::models::user::Claims;

/// User-related queries
#[derive(Default)]
pub struct UserQuery;

#[Object]
impl UserQuery {
    /// Get the currently authenticated user
    ///
    /// Returns the full user profile for the authenticated user.
    /// Requires a valid access token.
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if user no longer exists
    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        // Get the current claims from context (set by auth middleware)
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("authentication required"))?;

        let pool = ctx.data::<PgPool>()?;

        // Fetch the full user from database
        let user: crate::models::user::User = sqlx::query_as(
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
                updated_at,
                password_updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(claims.sub)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            // Log the full error server-side but return a generic message
            tracing::error!(error = %e, user_id = %claims.sub, "Failed to fetch user");
            async_graphql::Error::new("An unexpected error occurred")
        })?
        .ok_or_else(|| async_graphql::Error::new("user not found"))?;

        // Update last seen timestamp asynchronously (fire and forget)
        let pool_clone = pool.clone();
        let user_id = user.id;
        tokio::spawn(async move {
            let _ = sqlx::query("UPDATE users SET last_seen_at = NOW() WHERE id = $1")
                .bind(user_id)
                .execute(&pool_clone)
                .await;
        });

        Ok(User::from(user))
    }
}
