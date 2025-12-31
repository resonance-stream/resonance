//! Admin mutations for Resonance GraphQL API
//!
//! This module provides admin-only mutations for:
//! - Updating user roles
//! - Deleting users
//! - Invalidating user sessions
//!
//! All mutations require admin role authentication.

use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use sqlx::PgPool;
use uuid::Uuid;

use crate::graphql::types::AdminUserListItem;
use crate::models::user::{Claims, UserRole as DbUserRole};
use crate::repositories::AdminRepository;

/// User role input for admin operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum UserRoleInput {
    /// Administrator with full access
    Admin,
    /// Regular user
    User,
    /// Guest with limited access
    Guest,
}

impl From<UserRoleInput> for DbUserRole {
    fn from(role: UserRoleInput) -> Self {
        match role {
            UserRoleInput::Admin => DbUserRole::Admin,
            UserRoleInput::User => DbUserRole::User,
            UserRoleInput::Guest => DbUserRole::Guest,
        }
    }
}

/// Input for updating a user's role
#[derive(Debug, InputObject)]
pub struct UpdateUserRoleInput {
    /// The ID of the user to update
    pub user_id: Uuid,
    /// The new role to assign
    pub role: UserRoleInput,
}

/// Result of an admin operation
#[derive(Debug, Clone, SimpleObject)]
pub struct AdminOperationResult {
    /// Whether the operation was successful
    pub success: bool,
    /// Optional message describing the result
    pub message: Option<String>,
}

/// Result of a session invalidation operation
#[derive(Debug, Clone, SimpleObject)]
pub struct InvalidateSessionsResult {
    /// Whether the operation was successful
    pub success: bool,
    /// Number of sessions that were invalidated
    pub sessions_invalidated: i64,
}

/// Admin-only mutations
#[derive(Default)]
pub struct AdminMutation;

/// Check if the current user has admin role
fn require_admin(claims: &Claims) -> Result<()> {
    if claims.role != DbUserRole::Admin {
        return Err(async_graphql::Error::new("Admin access required"));
    }
    Ok(())
}

#[Object]
impl AdminMutation {
    /// Update a user's role
    ///
    /// # Arguments
    /// * `input` - The user ID and new role
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    /// - Returns error if trying to change own role
    /// - Returns error if demoting the last admin
    /// - Returns error if user not found
    async fn admin_update_user_role(
        &self,
        ctx: &Context<'_>,
        input: UpdateUserRoleInput,
    ) -> Result<AdminUserListItem> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        // Prevent admin from changing their own role
        if input.user_id == claims.sub {
            return Err(async_graphql::Error::new("Cannot change your own role"));
        }

        let pool = ctx.data::<PgPool>()?;
        let repo = AdminRepository::new(pool.clone());

        // Check if demoting the last admin
        let db_role: DbUserRole = input.role.into();
        if db_role != DbUserRole::Admin {
            // Get current user to check if they're an admin
            let current_user = repo
                .find_by_id(input.user_id)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, user_id = %input.user_id, "Failed to get user");
                    async_graphql::Error::new("Failed to get user")
                })?
                .ok_or_else(|| async_graphql::Error::new("User not found"))?;

            if current_user.role == DbUserRole::Admin {
                let admin_count = repo.count_admins().await.map_err(|e| {
                    tracing::error!(error = %e, "Failed to count admins");
                    async_graphql::Error::new("Failed to verify admin count")
                })?;

                if admin_count <= 1 {
                    return Err(async_graphql::Error::new(
                        "Cannot demote the last administrator",
                    ));
                }
            }
        }

        let updated = repo
            .update_user_role(input.user_id, db_role)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %input.user_id, "Failed to update user role");
                async_graphql::Error::new("Failed to update user role")
            })?;

        if !updated {
            return Err(async_graphql::Error::new("User not found"));
        }

        tracing::info!(
            admin_id = %claims.sub,
            target_user_id = %input.user_id,
            new_role = ?input.role,
            "Admin updated user role"
        );

        // Fetch the updated user (efficient single query)
        let user = repo
            .find_by_id(input.user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch updated user");
                async_graphql::Error::new("Role updated but failed to fetch user details")
            })?
            .ok_or_else(|| async_graphql::Error::new("User not found after update"))?;

        Ok(user.into())
    }

    /// Delete a user account
    ///
    /// This will invalidate all sessions and remove the user from the database.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user to delete
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    /// - Returns error if trying to delete own account
    /// - Returns error if user not found
    async fn admin_delete_user(
        &self,
        ctx: &Context<'_>,
        user_id: Uuid,
    ) -> Result<AdminOperationResult> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        // Prevent admin from deleting themselves
        if user_id == claims.sub {
            return Err(async_graphql::Error::new("Cannot delete your own account"));
        }

        let pool = ctx.data::<PgPool>()?;
        let repo = AdminRepository::new(pool.clone());

        let deleted = repo.delete_user(user_id).await.map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "Failed to delete user");
            async_graphql::Error::new("Failed to delete user")
        })?;

        if !deleted {
            return Err(async_graphql::Error::new("User not found"));
        }

        tracing::info!(
            admin_id = %claims.sub,
            deleted_user_id = %user_id,
            "Admin deleted user"
        );

        Ok(AdminOperationResult {
            success: true,
            message: Some("User deleted successfully".to_string()),
        })
    }

    /// Invalidate all sessions for a user (force logout)
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user whose sessions to invalidate
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    async fn admin_invalidate_sessions(
        &self,
        ctx: &Context<'_>,
        user_id: Uuid,
    ) -> Result<InvalidateSessionsResult> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        let pool = ctx.data::<PgPool>()?;
        let repo = AdminRepository::new(pool.clone());

        let count = repo.invalidate_user_sessions(user_id).await.map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "Failed to invalidate sessions");
            async_graphql::Error::new("Failed to invalidate sessions")
        })?;

        tracing::info!(
            admin_id = %claims.sub,
            target_user_id = %user_id,
            sessions_invalidated = count,
            "Admin invalidated user sessions"
        );

        Ok(InvalidateSessionsResult {
            success: true,
            sessions_invalidated: count as i64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_input_conversion() {
        assert!(matches!(
            DbUserRole::from(UserRoleInput::Admin),
            DbUserRole::Admin
        ));
        assert!(matches!(
            DbUserRole::from(UserRoleInput::User),
            DbUserRole::User
        ));
        assert!(matches!(
            DbUserRole::from(UserRoleInput::Guest),
            DbUserRole::Guest
        ));
    }
}
