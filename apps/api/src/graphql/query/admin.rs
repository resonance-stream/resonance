//! Admin queries for Resonance GraphQL API
//!
//! This module provides admin-only queries for:
//! - System statistics (user count, track count, etc.)
//! - User listing with pagination and search
//! - User detail with session information
//!
//! All queries require admin role authentication.

use async_graphql::{Context, Object, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::graphql::types::{
    AdminSession, AdminUserDetail, AdminUserList, AdminUserListItem, SystemStats,
};
use crate::models::user::{Claims, UserRole};
use crate::repositories::AdminRepository;

/// Admin-only queries
#[derive(Default)]
pub struct AdminQuery;

/// Check if the current user has admin role
fn require_admin(claims: &Claims) -> Result<()> {
    if claims.role != UserRole::Admin {
        return Err(async_graphql::Error::new("Admin access required"));
    }
    Ok(())
}

#[Object]
impl AdminQuery {
    /// Get system-wide statistics for the admin dashboard
    ///
    /// Returns counts of users, tracks, albums, artists, and active sessions,
    /// plus total library duration and file size.
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    async fn admin_system_stats(&self, ctx: &Context<'_>) -> Result<SystemStats> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        let pool = ctx.data::<PgPool>()?;
        let repo = AdminRepository::new(pool.clone());

        let stats = repo.get_system_stats().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get system stats");
            async_graphql::Error::new("Failed to retrieve system statistics")
        })?;

        Ok(stats.into())
    }

    /// List all users with pagination and optional search
    ///
    /// # Arguments
    /// * `limit` - Maximum number of users to return (default: 20, max: 100)
    /// * `offset` - Number of users to skip (default: 0)
    /// * `search` - Optional search query for email or display_name
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    async fn admin_users(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i32,
        #[graphql(default = 0)] offset: i32,
        search: Option<String>,
    ) -> Result<AdminUserList> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        // Clamp limit to reasonable bounds
        let limit = limit.clamp(1, 100) as i64;
        let offset = offset.max(0) as i64;

        let pool = ctx.data::<PgPool>()?;
        let repo = AdminRepository::new(pool.clone());

        let search_ref = search.as_deref();

        let (users, total_count) = tokio::try_join!(
            repo.list_users(limit, offset, search_ref),
            repo.count_users(search_ref),
        )
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list users");
            async_graphql::Error::new("Failed to retrieve users")
        })?;

        let users: Vec<AdminUserListItem> = users.into_iter().map(|u| u.into()).collect();
        let has_next_page = offset + (users.len() as i64) < total_count;

        Ok(AdminUserList {
            users,
            total_count,
            has_next_page,
        })
    }

    /// Get detailed user information including sessions
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user to retrieve
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    /// - Returns error if user not found
    async fn admin_user(&self, ctx: &Context<'_>, user_id: Uuid) -> Result<AdminUserDetail> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        let pool = ctx.data::<PgPool>()?;
        let repo = AdminRepository::new(pool.clone());

        // Get user by ID (efficient single query)
        let user_row = repo
            .find_by_id(user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %user_id, "Failed to get user");
                async_graphql::Error::new("Failed to retrieve user")
            })?
            .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        let sessions = repo.get_user_sessions(user_id).await.map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "Failed to get user sessions");
            async_graphql::Error::new("Failed to retrieve user sessions")
        })?;

        Ok(AdminUserDetail {
            user: user_row.into(),
            sessions: sessions.into_iter().map(|s| s.into()).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_admin() {
        use chrono::Utc;

        let admin_claims = Claims {
            sub: Uuid::new_v4(),
            email: "admin@example.com".to_string(),
            role: UserRole::Admin,
            sid: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
            exp: Utc::now().timestamp() + 3600,
            iss: "resonance".to_string(),
            aud: "resonance".to_string(),
        };
        assert!(require_admin(&admin_claims).is_ok());

        let user_claims = Claims {
            sub: Uuid::new_v4(),
            email: "user@example.com".to_string(),
            role: UserRole::User,
            sid: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
            exp: Utc::now().timestamp() + 3600,
            iss: "resonance".to_string(),
            aud: "resonance".to_string(),
        };
        assert!(require_admin(&user_claims).is_err());
    }
}
