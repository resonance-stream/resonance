//! Admin queries for Resonance GraphQL API
//!
//! This module provides admin-only queries for:
//! - System statistics (user count, track count, etc.)
//! - User listing with pagination and search
//! - User detail with session information
//! - Runtime configuration overview
//!
//! All queries require admin role authentication.

use async_graphql::{Context, Object, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::graphql::types::{
    AdminSession, AdminUserDetail, AdminUserList, AdminUserListItem, ConfigSource,
    RuntimeConfigOverview, RuntimeConfigStatus, ServiceType, SystemStats,
};
use crate::models::user::{Claims, UserRole};
use crate::repositories::AdminRepository;
use crate::services::config::ConfigService;

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

    /// Get runtime configuration overview
    ///
    /// Returns the current configuration status for all services,
    /// showing whether each service is configured and the source of
    /// its configuration (Database, Environment, or Default).
    ///
    /// This is useful for admins to understand which services are
    /// available and how they are configured.
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    async fn admin_runtime_config(&self, ctx: &Context<'_>) -> Result<RuntimeConfigOverview> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        let config_service = ctx.data::<ConfigService>()?;

        // Check each service's configuration status
        use crate::models::system_settings::ServiceType as DbServiceType;

        // Ollama - always returns config (has defaults), check if from DB or env
        let _ollama_config = config_service.get_ollama_config().await;
        let ollama_is_db = config_service
            .is_service_configured(DbServiceType::Ollama)
            .await;
        let ollama_source = if ollama_is_db {
            ConfigSource::Database
        } else if std::env::var("OLLAMA_URL").is_ok() {
            ConfigSource::Environment
        } else {
            ConfigSource::Default
        };

        // Lidarr - returns Option, None means not configured
        let lidarr_config = config_service.get_lidarr_config().await;
        let lidarr_is_db = config_service
            .is_service_configured(DbServiceType::Lidarr)
            .await;
        let (lidarr_configured, lidarr_source) = match (lidarr_config.is_some(), lidarr_is_db) {
            (true, true) => (true, ConfigSource::Database),
            (true, false) => (true, ConfigSource::Environment),
            (false, _) => (false, ConfigSource::NotConfigured),
        };

        // Last.fm - returns Option, None means not configured
        let lastfm_config = config_service.get_lastfm_config().await;
        let lastfm_is_db = config_service
            .is_service_configured(DbServiceType::Lastfm)
            .await;
        let (lastfm_configured, lastfm_source) = match (lastfm_config.is_some(), lastfm_is_db) {
            (true, true) => (true, ConfigSource::Database),
            (true, false) => (true, ConfigSource::Environment),
            (false, _) => (false, ConfigSource::NotConfigured),
        };

        // Music library - always returns path (has default), check source
        let music_path = config_service.get_music_library_path().await;
        let music_is_db = config_service
            .is_service_configured(DbServiceType::MusicLibrary)
            .await;
        let music_source = if music_is_db {
            ConfigSource::Database
        } else if std::env::var("MUSIC_LIBRARY_PATH").is_ok() {
            ConfigSource::Environment
        } else {
            ConfigSource::Default
        };
        let music_configured = music_path.exists() || music_is_db;

        Ok(RuntimeConfigOverview {
            ollama: RuntimeConfigStatus {
                service: ServiceType::Ollama,
                is_configured: true, // Ollama always has defaults
                config_source: ollama_source,
            },
            lidarr: RuntimeConfigStatus {
                service: ServiceType::Lidarr,
                is_configured: lidarr_configured,
                config_source: lidarr_source,
            },
            lastfm: RuntimeConfigStatus {
                service: ServiceType::Lastfm,
                is_configured: lastfm_configured,
                config_source: lastfm_source,
            },
            music_library: RuntimeConfigStatus {
                service: ServiceType::MusicLibrary,
                is_configured: music_configured,
                config_source: music_source,
            },
        })
    }

    /// Invalidate the configuration cache for a specific service
    ///
    /// Use this after updating system settings to ensure the new
    /// configuration takes effect immediately.
    ///
    /// # Arguments
    /// * `service` - The service type to invalidate cache for
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    async fn admin_invalidate_config_cache(
        &self,
        ctx: &Context<'_>,
        service: ServiceType,
    ) -> Result<bool> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        require_admin(claims)?;

        let config_service = ctx.data::<ConfigService>()?;

        use crate::models::system_settings::ServiceType as DbServiceType;
        let db_service: DbServiceType = service.into();

        config_service.invalidate_cache(db_service).await;

        tracing::info!(
            admin_id = %claims.sub,
            service = ?service,
            "Admin invalidated config cache"
        );

        Ok(true)
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
