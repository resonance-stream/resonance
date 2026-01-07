//! System settings mutations for Resonance GraphQL API
//!
//! This module provides mutations for system configuration:
//! - createInitialAdmin: Create the first admin user during setup (no auth required if no users exist)
//! - completeSetup: Mark first-run setup as complete (admin-only)
//! - updateSystemSetting: Update service configuration (admin-only)
//! - testServiceConnection: Test connection to an external service (admin-only)
//! - addUserLibraryPath: Add a personal library path (authenticated)
//! - removeUserLibraryPath: Remove a personal library path (authenticated)
//! - setUserPrimaryLibrary: Set primary library path (authenticated)

use async_graphql::{Context, Object, Result, ID};
use sqlx::PgPool;
use std::time::Instant;

use crate::error::ApiError;
use crate::graphql::types::{
    AuthPayload, ConnectionTestResult, CreateAdminInput, ServiceType, SystemSettingInfo,
    UpdateSystemSettingInput, UserLibraryPath,
};
use crate::models::system_settings::{ServiceType as DbServiceType, SystemSettingInput};
use crate::models::user::{Claims, UserPreferences, UserRole};
use crate::repositories::{SystemSettingsRepository, UserRepository};
use crate::services::auth::is_valid_email;
use crate::services::{AuthService, EncryptionService, HealthService};

// =============================================================================
// Constants
// =============================================================================

/// Advisory lock ID for initial admin creation
/// This lock prevents TOCTOU race conditions when checking if users exist
/// and creating the first admin user. Reserved exclusively for this purpose.
const INITIAL_ADMIN_LOCK_ID: i64 = 1;

// =============================================================================
// Helper Functions
// =============================================================================

/// Check if the current user has admin role
fn require_admin(claims: &Claims) -> Result<()> {
    if claims.role != UserRole::Admin {
        return Err(async_graphql::Error::new("Admin access required"));
    }
    Ok(())
}

/// Check if the current user is authenticated
fn require_auth<'a>(ctx: &'a Context<'a>) -> Result<&'a Claims> {
    ctx.data_opt::<Claims>()
        .ok_or_else(|| async_graphql::Error::new("Authentication required"))
}

/// Sanitize auth errors to prevent information disclosure
fn sanitize_auth_error(error: &ApiError) -> async_graphql::Error {
    match error {
        ApiError::Unauthorized => async_graphql::Error::new("Invalid credentials"),
        ApiError::ValidationError(msg) => async_graphql::Error::new(msg.clone()),
        ApiError::Conflict { resource_type, .. } => {
            if resource_type.to_lowercase().contains("email")
                || resource_type.to_lowercase().contains("user")
            {
                async_graphql::Error::new("Email already registered")
            } else {
                async_graphql::Error::new("Resource conflict")
            }
        }
        _ => {
            tracing::error!(error = %error, "Internal error in system settings mutation");
            async_graphql::Error::new("An unexpected error occurred")
        }
    }
}

// =============================================================================
// System Settings Mutations
// =============================================================================

/// System settings mutations for setup wizard and admin configuration
#[derive(Default)]
pub struct SystemSettingsMutation;

#[Object]
impl SystemSettingsMutation {
    /// Create the initial admin user during first-run setup
    ///
    /// This mutation ONLY works if:
    /// 1. No users exist in the database yet
    ///
    /// After creation, the user is automatically logged in and receives auth tokens.
    ///
    /// # Arguments
    /// * `input` - Admin user details (username, email, password)
    ///
    /// # Returns
    /// Auth payload with the new admin user and tokens
    ///
    /// # Errors
    /// - Returns error if users already exist in the database
    /// - Returns error if email format is invalid
    /// - Returns error if password doesn't meet complexity requirements
    async fn create_initial_admin(
        &self,
        ctx: &Context<'_>,
        input: CreateAdminInput,
    ) -> Result<AuthPayload> {
        let pool = ctx.data::<PgPool>()?;
        let auth_service = ctx.data::<AuthService>()?;

        // Use a transaction with advisory lock to prevent TOCTOU race condition
        // Advisory lock 1 is reserved for initial admin creation
        // This ensures only one request can check and create at a time
        let mut tx = pool.begin().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to start transaction");
            async_graphql::Error::new("Failed to start transaction")
        })?;

        // Acquire advisory lock (released automatically when transaction ends)
        sqlx::query(&format!("SELECT pg_advisory_xact_lock({})", INITIAL_ADMIN_LOCK_ID))
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to acquire advisory lock");
                async_graphql::Error::new("Failed to acquire lock")
            })?;

        // Check if any users already exist (within the lock)
        let user_count: i64 = sqlx::query_scalar(r#"SELECT COUNT(*) FROM users"#)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to count users");
                async_graphql::Error::new("Failed to check existing users")
            })?;

        if user_count > 0 {
            // Roll back (releases lock) and return error
            tx.rollback().await.ok();
            return Err(async_graphql::Error::new(
                "Initial admin creation is only available when no users exist",
            ));
        }

        // Validate input
        if input.username.trim().is_empty() || input.username.len() > 100 {
            return Err(async_graphql::Error::new(
                "Username must be between 1 and 100 characters",
            ));
        }

        // Validate email format using shared validation from auth service
        let email = input.email.trim();
        if !is_valid_email(email) {
            return Err(async_graphql::Error::new("Invalid email format"));
        }

        // Use AuthService's internal validation and password hashing
        // First, hash the password using the auth service's method
        let password_hash = {
            // Create a dummy registration to leverage AuthService's password validation
            // We'll validate the password format first
            if input.password.len() < 8 {
                return Err(async_graphql::Error::new(
                    "Password must be at least 8 characters",
                ));
            }

            // Use argon2 for password hashing (same as AuthService)
            use argon2::{
                password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
                Argon2,
            };

            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();

            argon2
                .hash_password(input.password.as_bytes(), &salt)
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to hash password");
                    async_graphql::Error::new("Failed to process password")
                })?
                .to_string()
        };

        // Create the admin user within the transaction
        let preferences_json = serde_json::to_value(UserPreferences::default()).map_err(|e| {
            tracing::error!(error = %e, "Failed to serialize preferences");
            async_graphql::Error::new("Failed to create user preferences")
        })?;

        let user = sqlx::query_as::<_, crate::models::user::User>(
            r#"
            INSERT INTO users (email, password_hash, username, role, preferences)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(&password_hash)
        .bind(&input.username)
        .bind(UserRole::Admin)
        .bind(&preferences_json)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create admin user");
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.is_unique_violation() {
                    return async_graphql::Error::new("Email already registered");
                }
            }
            async_graphql::Error::new("Failed to create admin user")
        })?;

        // Commit the transaction (releases advisory lock)
        tx.commit().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to commit transaction");
            async_graphql::Error::new("Failed to complete admin creation")
        })?;

        tracing::info!(user_id = %user.id, email = %user.email, "Initial admin user created");

        // Create session and tokens for the new admin
        let (_, tokens) = auth_service
            .login(&input.email, &input.password, None, None, None)
            .await
            .map_err(|e| sanitize_auth_error(&e))?;

        Ok(AuthPayload::new(user, tokens))
    }

    /// Mark the first-run setup as complete
    ///
    /// This mutation is admin-only and sets `setup_status.completed = true`.
    ///
    /// # Returns
    /// `true` if setup was marked complete successfully
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    async fn complete_setup(&self, ctx: &Context<'_>) -> Result<bool> {
        let claims = require_auth(ctx)?;
        require_admin(claims)?;

        let pool = ctx.data::<PgPool>()?;
        let repo = SystemSettingsRepository::new(pool.clone());

        repo.mark_setup_complete(claims.sub).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to mark setup complete");
            async_graphql::Error::new("Failed to complete setup")
        })?;

        tracing::info!(user_id = %claims.sub, "Setup marked as complete");
        Ok(true)
    }

    /// Update a system setting for an external service
    ///
    /// Admin-only. Updates configuration for services like Ollama, Lidarr, Last.fm, etc.
    /// Secrets are encrypted before storage.
    ///
    /// # Arguments
    /// * `input` - The setting update details
    ///
    /// # Returns
    /// The updated system setting info
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    /// - Returns error if config JSON is invalid
    async fn update_system_setting(
        &self,
        ctx: &Context<'_>,
        input: UpdateSystemSettingInput,
    ) -> Result<SystemSettingInfo> {
        let claims = require_auth(ctx)?;
        require_admin(claims)?;

        let pool = ctx.data::<PgPool>()?;
        let encryption = ctx.data::<EncryptionService>()?;
        let repo = SystemSettingsRepository::new(pool.clone());

        // Get existing setting to merge with updates
        let db_service: DbServiceType = input.service.into();
        let existing = repo.get_by_service(db_service).await.map_err(|e| {
            tracing::error!(error = %e, service = ?input.service, "Failed to get existing setting");
            async_graphql::Error::new("Failed to retrieve existing setting")
        })?;

        // Parse config JSON if provided
        let config: serde_json::Value = if let Some(config_str) = &input.config {
            serde_json::from_str(config_str).map_err(|e| {
                tracing::warn!(error = %e, "Invalid config JSON");
                async_graphql::Error::new(format!("Invalid config JSON: {}", e))
            })?
        } else {
            existing
                .as_ref()
                .map(|e| e.config.clone())
                .unwrap_or_else(|| serde_json::json!({}))
        };

        // Encrypt secret if provided
        let encrypted_secrets: Option<Vec<u8>> = if let Some(secret) = &input.secret {
            Some(encryption.encrypt(secret).map_err(|e| {
                tracing::error!(error = %e, "Failed to encrypt secret");
                async_graphql::Error::new("Failed to encrypt secret")
            })?)
        } else {
            existing.as_ref().and_then(|e| e.encrypted_secrets.clone())
        };

        // Determine enabled status
        let enabled = input
            .enabled
            .unwrap_or_else(|| existing.as_ref().map(|e| e.enabled).unwrap_or(true));

        // Create input for upsert
        let setting_input = SystemSettingInput {
            service: db_service,
            enabled,
            config,
            encrypted_secrets,
        };

        // Upsert the setting
        let updated = repo.upsert(&setting_input, claims.sub).await.map_err(|e| {
            tracing::error!(error = %e, service = ?input.service, "Failed to update setting");
            async_graphql::Error::new("Failed to update system setting")
        })?;

        tracing::info!(
            user_id = %claims.sub,
            service = ?input.service,
            "System setting updated"
        );

        Ok(updated.into())
    }

    /// Test connection to an external service
    ///
    /// Admin-only. Tests connectivity to the specified service using its current configuration.
    ///
    /// # Arguments
    /// * `service` - The service type to test
    ///
    /// # Returns
    /// Connection test result with success status, response time, version, and any error
    ///
    /// # Errors
    /// - Returns error if not authenticated as admin
    async fn test_service_connection(
        &self,
        ctx: &Context<'_>,
        service: ServiceType,
    ) -> Result<ConnectionTestResult> {
        let claims = require_auth(ctx)?;
        require_admin(claims)?;

        let pool = ctx.data::<PgPool>()?;
        let encryption = ctx.data::<EncryptionService>()?;
        let health_service = HealthService::new();
        let repo = SystemSettingsRepository::new(pool.clone());

        // Get service configuration from database
        let db_service: DbServiceType = service.into();
        let setting = repo.get_by_service(db_service).await.map_err(|e| {
            tracing::error!(error = %e, service = ?service, "Failed to get service config");
            async_graphql::Error::new("Failed to retrieve service configuration")
        })?;

        // Track if we have a database setting to update
        let has_db_setting = setting.is_some();

        let start = Instant::now();

        // Test connection based on service type
        let result = match service {
            ServiceType::Ollama => {
                let url = setting
                    .as_ref()
                    .and_then(|s| s.config.get("url").and_then(|v| v.as_str()))
                    .unwrap_or("http://ollama:11434");

                let model = setting
                    .as_ref()
                    .and_then(|s| s.config.get("model").and_then(|v| v.as_str()))
                    .unwrap_or("mistral");

                let health = health_service.check_ollama(url, model).await;
                convert_health_to_result(health, start.elapsed())
            }
            ServiceType::Lidarr => {
                let lidarr_setting = setting
                    .as_ref()
                    .ok_or_else(|| async_graphql::Error::new("Lidarr is not configured"))?;

                let url = lidarr_setting
                    .config
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| async_graphql::Error::new("Lidarr URL not configured"))?;

                let api_key = if let Some(encrypted) = &lidarr_setting.encrypted_secrets {
                    encryption.decrypt(encrypted).map_err(|e| {
                        tracing::error!(error = %e, "Failed to decrypt Lidarr API key");
                        async_graphql::Error::new("Failed to decrypt API key")
                    })?
                } else {
                    return Ok(ConnectionTestResult {
                        success: false,
                        response_time_ms: Some(start.elapsed().as_millis() as i64),
                        version: None,
                        error: Some("Lidarr API key not configured".to_string()),
                    });
                };

                // Test Lidarr connection using their system/status endpoint
                test_lidarr_connection(url, &api_key).await
            }
            ServiceType::Lastfm => {
                let lastfm_setting = setting
                    .as_ref()
                    .ok_or_else(|| async_graphql::Error::new("Last.fm is not configured"))?;

                let api_key = if let Some(encrypted) = &lastfm_setting.encrypted_secrets {
                    encryption.decrypt(encrypted).map_err(|e| {
                        tracing::error!(error = %e, "Failed to decrypt Last.fm API key");
                        async_graphql::Error::new("Failed to decrypt API key")
                    })?
                } else {
                    return Ok(ConnectionTestResult {
                        success: false,
                        response_time_ms: Some(start.elapsed().as_millis() as i64),
                        version: None,
                        error: Some("Last.fm API key not configured".to_string()),
                    });
                };

                test_lastfm_connection(&api_key).await
            }
            ServiceType::Meilisearch => {
                let env_url = std::env::var("MEILISEARCH_URL").ok();
                let url = setting
                    .as_ref()
                    .and_then(|s| {
                        s.config
                            .get("url")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    })
                    .or(env_url)
                    .unwrap_or_else(|| "http://meilisearch:7700".to_string());

                let api_key = setting
                    .as_ref()
                    .and_then(|s| {
                        s.encrypted_secrets
                            .as_ref()
                            .and_then(|encrypted| encryption.decrypt(encrypted).ok())
                    })
                    .or_else(|| std::env::var("MEILISEARCH_KEY").ok())
                    .unwrap_or_default();

                let health = health_service.check_meilisearch(&url, &api_key).await;
                convert_health_to_result(health, start.elapsed())
            }
            ServiceType::MusicLibrary => {
                let env_path = std::env::var("MUSIC_LIBRARY_PATH").ok();
                let path = setting
                    .as_ref()
                    .and_then(|s| {
                        s.config
                            .get("path")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    })
                    .or(env_path)
                    .unwrap_or_else(|| "/music".to_string());

                test_music_library_path(&path).await
            }
        };

        // Update health status in database if we have a setting
        if has_db_setting {
            let _ = repo
                .update_health(db_service, result.success, result.error.clone())
                .await;
            tracing::debug!(
                service = ?service,
                success = result.success,
                "Updated service health status"
            );
        }

        Ok(result)
    }

    // =========================================================================
    // User Library Path Mutations
    // =========================================================================

    /// Add a library path for the current user
    ///
    /// Users can add their own music library paths. If this is the user's first path,
    /// it will automatically be set as primary.
    ///
    /// # Arguments
    /// * `path` - The file system path to the music library
    /// * `label` - Optional user-friendly label
    ///
    /// # Returns
    /// The newly created library path
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if path already exists for user
    async fn add_user_library_path(
        &self,
        ctx: &Context<'_>,
        path: String,
        label: Option<String>,
    ) -> Result<UserLibraryPath> {
        let claims = require_auth(ctx)?;

        if path.trim().is_empty() {
            return Err(async_graphql::Error::new("Path cannot be empty"));
        }

        let pool = ctx.data::<PgPool>()?;
        let repo = SystemSettingsRepository::new(pool.clone());

        let library_path = repo
            .add_user_library_path(claims.sub, &path, label.as_deref())
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %claims.sub, "Failed to add library path");
                if let sqlx::Error::Database(ref db_err) = e {
                    if db_err.is_unique_violation() {
                        return async_graphql::Error::new("This path is already configured");
                    }
                }
                async_graphql::Error::new("Failed to add library path")
            })?;

        tracing::info!(
            user_id = %claims.sub,
            path_id = %library_path.id,
            path = %path,
            "User library path added"
        );

        Ok(library_path.into())
    }

    /// Remove a library path for the current user
    ///
    /// Users can only remove their own library paths.
    ///
    /// # Arguments
    /// * `id` - The ID of the library path to remove
    ///
    /// # Returns
    /// `true` if the path was removed, `false` if it didn't exist
    ///
    /// # Errors
    /// - Returns error if not authenticated
    async fn remove_user_library_path(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let claims = require_auth(ctx)?;

        let path_id = UserLibraryPath::parse_id(&id)?;

        let pool = ctx.data::<PgPool>()?;
        let repo = SystemSettingsRepository::new(pool.clone());

        let removed = repo
            .remove_user_library_path(claims.sub, path_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %claims.sub, path_id = %path_id, "Failed to remove library path");
                async_graphql::Error::new("Failed to remove library path")
            })?;

        if removed {
            tracing::info!(
                user_id = %claims.sub,
                path_id = %path_id,
                "User library path removed"
            );
        }

        Ok(removed)
    }

    /// Set a library path as the user's primary path
    ///
    /// Users can only modify their own library paths.
    ///
    /// # Arguments
    /// * `id` - The ID of the library path to make primary
    ///
    /// # Returns
    /// The updated library path
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if path doesn't exist or doesn't belong to user
    async fn set_user_primary_library(&self, ctx: &Context<'_>, id: ID) -> Result<UserLibraryPath> {
        let claims = require_auth(ctx)?;

        let path_id = UserLibraryPath::parse_id(&id)?;

        let pool = ctx.data::<PgPool>()?;
        let repo = SystemSettingsRepository::new(pool.clone());

        let library_path = repo
            .set_primary_library_path(claims.sub, path_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %claims.sub, path_id = %path_id, "Failed to set primary library path");
                if matches!(e, sqlx::Error::RowNotFound) {
                    return async_graphql::Error::new("Library path not found");
                }
                async_graphql::Error::new("Failed to set primary library path")
            })?;

        tracing::info!(
            user_id = %claims.sub,
            path_id = %path_id,
            "User primary library path updated"
        );

        Ok(library_path.into())
    }
}

// =============================================================================
// Helper Functions for Connection Testing
// =============================================================================

/// Convert HealthService result to ConnectionTestResult
fn convert_health_to_result(
    health: crate::services::health::ServiceHealth,
    elapsed: std::time::Duration,
) -> ConnectionTestResult {
    use crate::services::health::ServiceStatus;

    let version = health
        .details
        .as_ref()
        .and_then(|d| d.get("version").and_then(|v| v.as_str()))
        .map(String::from);

    ConnectionTestResult {
        success: health.status == ServiceStatus::Healthy,
        response_time_ms: Some(elapsed.as_millis() as i64),
        version,
        error: health.error,
    }
}

/// Test Lidarr connection
async fn test_lidarr_connection(url: &str, api_key: &str) -> ConnectionTestResult {
    let start = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let status_url = format!("{}/api/v1/system/status", url.trim_end_matches('/'));

    match client
        .get(&status_url)
        .header("X-Api-Key", api_key)
        .send()
        .await
    {
        Ok(response) => {
            let elapsed = start.elapsed();
            if response.status().is_success() {
                let version = response
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|v| v.get("version").and_then(|v| v.as_str()).map(String::from));

                ConnectionTestResult {
                    success: true,
                    response_time_ms: Some(elapsed.as_millis() as i64),
                    version,
                    error: None,
                }
            } else {
                ConnectionTestResult {
                    success: false,
                    response_time_ms: Some(elapsed.as_millis() as i64),
                    version: None,
                    error: Some(format!("HTTP {}", response.status())),
                }
            }
        }
        Err(e) => ConnectionTestResult {
            success: false,
            response_time_ms: None,
            version: None,
            error: Some(format!("Connection failed: {}", e)),
        },
    }
}

/// Test Last.fm connection
async fn test_lastfm_connection(api_key: &str) -> ConnectionTestResult {
    let start = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    // Test with a simple API call (get artist info for a well-known artist)
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=artist.getinfo&artist=Radiohead&api_key={}&format=json",
        api_key
    );

    match client.get(&url).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            if response.status().is_success() {
                let body = response.json::<serde_json::Value>().await.ok();

                // Check if we got an error response from Last.fm
                if let Some(ref json) = body {
                    if json.get("error").is_some() {
                        let message = json
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Invalid API key");
                        return ConnectionTestResult {
                            success: false,
                            response_time_ms: Some(elapsed.as_millis() as i64),
                            version: None,
                            error: Some(message.to_string()),
                        };
                    }
                }

                ConnectionTestResult {
                    success: true,
                    response_time_ms: Some(elapsed.as_millis() as i64),
                    version: Some("2.0".to_string()), // Last.fm API version
                    error: None,
                }
            } else {
                ConnectionTestResult {
                    success: false,
                    response_time_ms: Some(elapsed.as_millis() as i64),
                    version: None,
                    error: Some(format!("HTTP {}", response.status())),
                }
            }
        }
        Err(e) => ConnectionTestResult {
            success: false,
            response_time_ms: None,
            version: None,
            error: Some(format!("Connection failed: {}", e)),
        },
    }
}

/// Test music library path accessibility
async fn test_music_library_path(path: &str) -> ConnectionTestResult {
    use tokio::fs;

    let start = Instant::now();
    let path = std::path::Path::new(path);

    // Use tokio::fs::metadata for async file system checks
    let metadata = match fs::metadata(path).await {
        Ok(m) => m,
        Err(e) => {
            let error_msg = if e.kind() == std::io::ErrorKind::NotFound {
                "Path does not exist".to_string()
            } else {
                format!("Cannot access path: {}", e)
            };
            return ConnectionTestResult {
                success: false,
                response_time_ms: Some(start.elapsed().as_millis() as i64),
                version: None,
                error: Some(error_msg),
            };
        }
    };

    if !metadata.is_dir() {
        return ConnectionTestResult {
            success: false,
            response_time_ms: Some(start.elapsed().as_millis() as i64),
            version: None,
            error: Some("Path is not a directory".to_string()),
        };
    }

    // Check if we can read the directory using async read_dir
    match fs::read_dir(path).await {
        Ok(mut entries) => {
            // Count entries asynchronously
            let mut count = 0;
            while let Ok(Some(_)) = entries.next_entry().await {
                count += 1;
            }
            ConnectionTestResult {
                success: true,
                response_time_ms: Some(start.elapsed().as_millis() as i64),
                version: None,
                error: if count == 0 {
                    Some("Directory is empty".to_string())
                } else {
                    None
                },
            }
        }
        Err(e) => ConnectionTestResult {
            success: false,
            response_time_ms: Some(start.elapsed().as_millis() as i64),
            version: None,
            error: Some(format!("Cannot read directory: {}", e)),
        },
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

    #[test]
    fn test_music_library_path_validation() {
        // Test with a non-existent path
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(test_music_library_path("/nonexistent/path/12345"));
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    // Note: Email validation tests are in services/auth.rs where is_valid_email is defined.
    // The create_initial_admin mutation uses that shared function, ensuring consistent behavior.
    // See test_is_valid_email in auth.rs for comprehensive email validation test coverage.
}
