//! System settings queries for Resonance GraphQL API
//!
//! This module provides queries for:
//! - Setup status (unauthenticated - for setup wizard)
//! - System settings listing (admin-only)
//! - Individual system setting lookup (admin-only)
//!
//! Note: The `setupStatus` query is intentionally unauthenticated to allow
//! the setup wizard to check if initial configuration is needed.

use async_graphql::{Context, Object, Result};
use sqlx::PgPool;

use crate::graphql::types::{ServiceType, SetupStatus, SystemSettingInfo};
use crate::models::system_settings::ServiceType as DbServiceType;
use crate::models::user::{Claims, UserRole};
use crate::repositories::SystemSettingsRepository;

/// System settings queries
#[derive(Default)]
pub struct SystemSettingsQuery;

/// Check if the current user has admin role
fn require_admin(claims: &Claims) -> Result<()> {
    if claims.role != UserRole::Admin {
        return Err(async_graphql::Error::new("Admin access required"));
    }
    Ok(())
}

#[Object]
impl SystemSettingsQuery {
    /// Get the current setup status
    ///
    /// This query is intentionally UNAUTHENTICATED to allow the setup wizard
    /// to determine if initial configuration is needed before any users exist.
    ///
    /// Returns:
    /// - `is_complete`: Whether the first-run setup wizard has been completed
    /// - `has_admin`: Whether at least one admin user exists
    /// - `configured_services`: List of services that have been configured
    async fn setup_status(&self, ctx: &Context<'_>) -> Result<SetupStatus> {
        let pool = ctx.data::<PgPool>()?;
        let repo = SystemSettingsRepository::new(pool.clone());

        // Check if setup is complete
        let is_complete = repo.is_setup_complete().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to check setup status");
            async_graphql::Error::new("Failed to check setup status")
        })?;

        // Check if any admin users exist
        let has_admin: Option<bool> =
            sqlx::query_scalar(r#"SELECT EXISTS(SELECT 1 FROM users WHERE role = 'admin')"#)
                .fetch_one(pool)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to check for admin users");
                    async_graphql::Error::new("Failed to check for admin users")
                })?;
        let has_admin = has_admin.unwrap_or(false);

        // Get all configured (enabled) services
        let settings = repo.get_all().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get system settings");
            async_graphql::Error::new("Failed to get system settings")
        })?;

        let configured_services: Vec<ServiceType> = settings
            .into_iter()
            .filter(|s| s.enabled)
            .map(|s| s.service.into())
            .collect();

        Ok(SetupStatus {
            is_complete,
            has_admin,
            configured_services,
        })
    }

    /// Get all system settings
    ///
    /// Returns all configured external service settings. Only accessible by admin users.
    /// Secrets are never exposed - only `has_secret` indicates if secrets are configured.
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    async fn system_settings(&self, ctx: &Context<'_>) -> Result<Vec<SystemSettingInfo>> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        let pool = ctx.data::<PgPool>()?;
        let repo = SystemSettingsRepository::new(pool.clone());

        let settings = repo.get_all().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get system settings");
            async_graphql::Error::new("Failed to retrieve system settings")
        })?;

        Ok(settings.into_iter().map(|s| s.into()).collect())
    }

    /// Get a single system setting by service type
    ///
    /// Returns the configuration for a specific service. Only accessible by admin users.
    /// Secrets are never exposed - only `has_secret` indicates if secrets are configured.
    ///
    /// # Arguments
    /// * `service` - The service type to look up
    ///
    /// # Returns
    /// * The setting info if found, or null if not configured
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    async fn system_setting(
        &self,
        ctx: &Context<'_>,
        service: ServiceType,
    ) -> Result<Option<SystemSettingInfo>> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        let pool = ctx.data::<PgPool>()?;
        let repo = SystemSettingsRepository::new(pool.clone());

        let db_service: DbServiceType = service.into();
        let setting = repo.get_by_service(db_service).await.map_err(|e| {
            tracing::error!(error = %e, service = ?service, "Failed to get system setting");
            async_graphql::Error::new("Failed to retrieve system setting")
        })?;

        Ok(setting.map(|s| s.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_require_admin_passes_for_admin() {
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
    }

    #[test]
    fn test_require_admin_fails_for_user() {
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
