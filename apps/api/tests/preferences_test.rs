//! Integration tests for GraphQL preferences mutations
//!
//! Tests the complete preferences lifecycle:
//! - updatePreferences: Valid updates, partial updates, validation errors
//! - resetPreferences: Reset to default values
//! - getPreferences: Query preferences mutation
//!
//! # Requirements
//!
//! These tests require a PostgreSQL database to be running. Set the `DATABASE_URL`
//! environment variable or have a local database at `postgres://resonance:resonance@localhost:5432/resonance_test`.
//!
//! To run the tests:
//! ```bash
//! # Start the test database (from project root)
//! docker compose up -d postgres
//!
//! # Run the tests
//! DATABASE_URL="postgres://resonance:resonance@localhost:5432/resonance" cargo test --test preferences_test -p resonance-api
//! ```
//!
//! If the database is not available, tests will be skipped automatically.

mod common;

use async_graphql::{EmptySubscription, Schema};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use resonance_api::graphql::mutation::Mutation;
use resonance_api::graphql::query::Query;
use resonance_api::models::user::{Claims, UserRole};
use resonance_api::repositories::UserRepository;

// ========== Test Fixtures ==========

/// Create a test database pool connected to test database.
/// Returns None if the database is not available, allowing tests to be skipped.
async fn try_create_test_pool() -> Option<PgPool> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://resonance:resonance@localhost:5432/resonance_test".to_string()
    });

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .ok()
}

/// Macro to skip tests if the database is not available
macro_rules! require_db {
    ($pool_var:ident) => {
        let $pool_var = match try_create_test_pool().await {
            Some(p) => p,
            None => {
                eprintln!("Skipping test: database not available");
                return;
            }
        };
    };
}

/// Test context that manages test users and provides GraphQL schema execution
struct PreferencesTestContext {
    pool: PgPool,
    user_id: Uuid,
    email: String,
    schema: Schema<Query, Mutation, EmptySubscription>,
}

impl PreferencesTestContext {
    /// Create a new test context with a registered test user
    async fn new(pool: PgPool) -> Self {
        let user_id = Uuid::new_v4();
        let email = format!("test_prefs_{}@example.com", Uuid::new_v4());
        let password_hash = "$argon2id$v=19$m=65536,t=3,p=4$test$hash"; // Dummy hash

        // Create a test user
        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, display_name, role, preferences)
            VALUES ($1, $2, $3, $4, 'user', $5)
            "#,
        )
        .bind(user_id)
        .bind(&email)
        .bind(password_hash)
        .bind("Test Preferences User")
        .bind(json!({
            "theme": "dark",
            "quality": "high",
            "crossfade_duration_ms": 0,
            "gapless_playback": true,
            "normalize_volume": false,
            "show_explicit": true,
            "private_session": false,
            "discord_rpc": true,
            "listenbrainz_scrobble": false
        }))
        .execute(&pool)
        .await
        .expect("Failed to create test user");

        let user_repo = UserRepository::new(pool.clone());

        // Build schema with test data
        let schema = Schema::build(Query::default(), Mutation::default(), EmptySubscription)
            .data(pool.clone())
            .data(user_repo)
            .finish();

        Self {
            pool,
            user_id,
            email,
            schema,
        }
    }

    /// Execute a GraphQL query with authentication
    async fn execute_authenticated(&self, query: &str) -> async_graphql::Response {
        let claims = Claims {
            sub: self.user_id,
            email: self.email.clone(),
            role: UserRole::User,
            sid: Uuid::new_v4(),
            iat: chrono::Utc::now().timestamp(),
            exp: chrono::Utc::now().timestamp() + 3600,
            iss: "resonance".to_string(),
            aud: "resonance".to_string(),
        };

        let request = async_graphql::Request::new(query).data(claims);
        self.schema.execute(request).await
    }

    /// Execute a GraphQL query with variables and authentication
    async fn execute_authenticated_with_variables(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> async_graphql::Response {
        let claims = Claims {
            sub: self.user_id,
            email: self.email.clone(),
            role: UserRole::User,
            sid: Uuid::new_v4(),
            iat: chrono::Utc::now().timestamp(),
            exp: chrono::Utc::now().timestamp() + 3600,
            iss: "resonance".to_string(),
            aud: "resonance".to_string(),
        };

        let request = async_graphql::Request::new(query)
            .data(claims)
            .variables(async_graphql::Variables::from_json(variables));
        self.schema.execute(request).await
    }

    /// Execute a GraphQL query without authentication
    async fn execute_unauthenticated(&self, query: &str) -> async_graphql::Response {
        self.schema.execute(query).await
    }

    /// Clean up test data
    async fn cleanup(&self) {
        // Delete sessions first (foreign key constraint)
        let _ = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(self.user_id)
            .execute(&self.pool)
            .await;

        // Delete user
        let _ = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(self.user_id)
            .execute(&self.pool)
            .await;
    }
}

// =============================================================================
// updatePreferences Mutation Tests
// =============================================================================

#[tokio::test]
async fn test_update_preferences_theme() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { theme: "light" }) {
                id
                preferences {
                    theme
                    quality
                    gaplessPlayback
                }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should update theme without errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let prefs = &data["updatePreferences"]["preferences"];

    assert_eq!(prefs["theme"].as_str().unwrap(), "light");
    // Other fields should remain unchanged
    assert_eq!(prefs["quality"].as_str().unwrap(), "high");
    assert!(prefs["gaplessPlayback"].as_bool().unwrap());

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_quality() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { quality: "lossless" }) {
                preferences {
                    quality
                }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should update quality without errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    assert_eq!(
        data["updatePreferences"]["preferences"]["quality"]
            .as_str()
            .unwrap(),
        "lossless"
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_crossfade() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { crossfadeDurationMs: 3000 }) {
                preferences {
                    crossfadeDurationMs
                }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should update crossfade without errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    assert_eq!(
        data["updatePreferences"]["preferences"]["crossfadeDurationMs"]
            .as_u64()
            .unwrap(),
        3000
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_boolean_fields() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: {
                gaplessPlayback: false,
                normalizeVolume: true,
                showExplicit: false,
                privateSession: true,
                discordRpc: false,
                listenbrainzScrobble: true
            }) {
                preferences {
                    gaplessPlayback
                    normalizeVolume
                    showExplicit
                    privateSession
                    discordRpc
                    listenbrainzScrobble
                }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should update boolean fields without errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let prefs = &data["updatePreferences"]["preferences"];

    assert!(!prefs["gaplessPlayback"].as_bool().unwrap());
    assert!(prefs["normalizeVolume"].as_bool().unwrap());
    assert!(!prefs["showExplicit"].as_bool().unwrap());
    assert!(prefs["privateSession"].as_bool().unwrap());
    assert!(!prefs["discordRpc"].as_bool().unwrap());
    assert!(prefs["listenbrainzScrobble"].as_bool().unwrap());

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_multiple_fields() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: {
                theme: "light",
                quality: "lossless",
                crossfadeDurationMs: 5000,
                gaplessPlayback: true,
                normalizeVolume: true
            }) {
                preferences {
                    theme
                    quality
                    crossfadeDurationMs
                    gaplessPlayback
                    normalizeVolume
                }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should update multiple fields without errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let prefs = &data["updatePreferences"]["preferences"];

    assert_eq!(prefs["theme"].as_str().unwrap(), "light");
    assert_eq!(prefs["quality"].as_str().unwrap(), "lossless");
    assert_eq!(prefs["crossfadeDurationMs"].as_u64().unwrap(), 5000);
    assert!(prefs["gaplessPlayback"].as_bool().unwrap());
    assert!(prefs["normalizeVolume"].as_bool().unwrap());

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_with_variables() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation UpdatePrefs($input: UpdatePreferencesInput!) {
            updatePreferences(input: $input) {
                preferences {
                    theme
                    quality
                }
            }
        }
    "#;

    let variables = json!({
        "input": {
            "theme": "light",
            "quality": "medium"
        }
    });

    let response = ctx
        .execute_authenticated_with_variables(mutation, variables)
        .await;

    assert!(
        response.errors.is_empty(),
        "Should update with variables without errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let prefs = &data["updatePreferences"]["preferences"];

    assert_eq!(prefs["theme"].as_str().unwrap(), "light");
    assert_eq!(prefs["quality"].as_str().unwrap(), "medium");

    ctx.cleanup().await;
}

// =============================================================================
// updatePreferences Validation Tests
// =============================================================================

#[tokio::test]
async fn test_update_preferences_invalid_theme() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { theme: "auto" }) {
                preferences { theme }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(!response.errors.is_empty(), "Should error on invalid theme");
    let error_msg = response.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("invalid") || error_msg.contains("theme"),
        "Error should mention invalid theme: {}",
        response.errors[0].message
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_invalid_quality() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { quality: "ultra" }) {
                preferences { quality }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        !response.errors.is_empty(),
        "Should error on invalid quality"
    );
    let error_msg = response.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("invalid") || error_msg.contains("quality"),
        "Error should mention invalid quality: {}",
        response.errors[0].message
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_crossfade_exceeds_max() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { crossfadeDurationMs: 15000 }) {
                preferences { crossfadeDurationMs }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        !response.errors.is_empty(),
        "Should error on crossfade exceeding max"
    );
    let error_msg = response.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("12000") || error_msg.contains("exceed"),
        "Error should mention max crossfade: {}",
        response.errors[0].message
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_empty_input() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: {}) {
                preferences { theme }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(!response.errors.is_empty(), "Should error on empty input");
    let error_msg = response.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("field") || error_msg.contains("provided"),
        "Error should mention no fields provided: {}",
        response.errors[0].message
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_unauthenticated() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { theme: "light" }) {
                preferences { theme }
            }
        }
    "#;

    let response = ctx.execute_unauthenticated(mutation).await;

    assert!(
        !response.errors.is_empty(),
        "Should error when unauthenticated"
    );
    let error_msg = response.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("auth"),
        "Error should mention authentication: {}",
        response.errors[0].message
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_theme_case_insensitive() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { theme: "DARK" }) {
                preferences { theme }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should accept uppercase theme: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    // Should be normalized to lowercase
    assert_eq!(
        data["updatePreferences"]["preferences"]["theme"]
            .as_str()
            .unwrap(),
        "dark"
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_quality_with_whitespace() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { quality: "  High  " }) {
                preferences { quality }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should accept quality with whitespace: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    // Should be trimmed and normalized
    assert_eq!(
        data["updatePreferences"]["preferences"]["quality"]
            .as_str()
            .unwrap(),
        "high"
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_crossfade_zero() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { crossfadeDurationMs: 0 }) {
                preferences { crossfadeDurationMs }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should accept zero crossfade (disabled): {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    assert_eq!(
        data["updatePreferences"]["preferences"]["crossfadeDurationMs"]
            .as_u64()
            .unwrap(),
        0
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_crossfade_max() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { crossfadeDurationMs: 12000 }) {
                preferences { crossfadeDurationMs }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should accept max crossfade (12000): {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    assert_eq!(
        data["updatePreferences"]["preferences"]["crossfadeDurationMs"]
            .as_u64()
            .unwrap(),
        12000
    );

    ctx.cleanup().await;
}

// =============================================================================
// resetPreferences Mutation Tests
// =============================================================================

#[tokio::test]
async fn test_reset_preferences() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    // First, update some preferences
    let update_mutation = r#"
        mutation {
            updatePreferences(input: {
                theme: "light",
                quality: "lossless",
                crossfadeDurationMs: 5000,
                normalizeVolume: true
            }) {
                preferences { theme }
            }
        }
    "#;

    let response = ctx.execute_authenticated(update_mutation).await;
    assert!(response.errors.is_empty(), "Update should succeed first");

    // Now reset preferences
    let reset_mutation = r#"
        mutation {
            resetPreferences {
                preferences {
                    theme
                    quality
                    crossfadeDurationMs
                    gaplessPlayback
                    normalizeVolume
                    showExplicit
                    privateSession
                    discordRpc
                    listenbrainzScrobble
                }
            }
        }
    "#;

    let response = ctx.execute_authenticated(reset_mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should reset preferences without errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let prefs = &data["resetPreferences"]["preferences"];

    // Verify all defaults are restored
    assert_eq!(prefs["theme"].as_str().unwrap(), "dark");
    assert_eq!(prefs["quality"].as_str().unwrap(), "high");
    assert_eq!(prefs["crossfadeDurationMs"].as_u64().unwrap(), 0);
    assert!(prefs["gaplessPlayback"].as_bool().unwrap());
    assert!(!prefs["normalizeVolume"].as_bool().unwrap());
    assert!(prefs["showExplicit"].as_bool().unwrap());
    assert!(!prefs["privateSession"].as_bool().unwrap());
    assert!(prefs["discordRpc"].as_bool().unwrap());
    assert!(!prefs["listenbrainzScrobble"].as_bool().unwrap());

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_reset_preferences_unauthenticated() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            resetPreferences {
                preferences { theme }
            }
        }
    "#;

    let response = ctx.execute_unauthenticated(mutation).await;

    assert!(
        !response.errors.is_empty(),
        "Should error when unauthenticated"
    );
    let error_msg = response.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("auth"),
        "Error should mention authentication: {}",
        response.errors[0].message
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_reset_preferences_returns_user_info() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            resetPreferences {
                id
                email
                displayName
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should return user info: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    assert!(data["resetPreferences"]["id"].is_string());
    assert!(data["resetPreferences"]["email"].is_string());
    assert!(data["resetPreferences"]["displayName"].is_string());

    ctx.cleanup().await;
}

// =============================================================================
// getPreferences Mutation Tests
// =============================================================================

#[tokio::test]
async fn test_get_preferences() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            getPreferences {
                theme
                quality
                crossfadeDurationMs
                gaplessPlayback
                normalizeVolume
                showExplicit
                privateSession
                discordRpc
                listenbrainzScrobble
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should get preferences without errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let prefs = &data["getPreferences"];

    // Verify default values are returned
    assert_eq!(prefs["theme"].as_str().unwrap(), "dark");
    assert_eq!(prefs["quality"].as_str().unwrap(), "high");
    assert_eq!(prefs["crossfadeDurationMs"].as_u64().unwrap(), 0);
    assert!(prefs["gaplessPlayback"].as_bool().unwrap());
    assert!(!prefs["normalizeVolume"].as_bool().unwrap());
    assert!(prefs["showExplicit"].as_bool().unwrap());
    assert!(!prefs["privateSession"].as_bool().unwrap());
    assert!(prefs["discordRpc"].as_bool().unwrap());
    assert!(!prefs["listenbrainzScrobble"].as_bool().unwrap());

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_get_preferences_after_update() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    // First update preferences
    let update_mutation = r#"
        mutation {
            updatePreferences(input: {
                theme: "light",
                quality: "lossless"
            }) {
                preferences { theme }
            }
        }
    "#;

    let response = ctx.execute_authenticated(update_mutation).await;
    assert!(response.errors.is_empty(), "Update should succeed");

    // Now get preferences
    let get_mutation = r#"
        mutation {
            getPreferences {
                theme
                quality
            }
        }
    "#;

    let response = ctx.execute_authenticated(get_mutation).await;

    assert!(
        response.errors.is_empty(),
        "Should get updated preferences: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let prefs = &data["getPreferences"];

    assert_eq!(prefs["theme"].as_str().unwrap(), "light");
    assert_eq!(prefs["quality"].as_str().unwrap(), "lossless");

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_get_preferences_unauthenticated() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            getPreferences {
                theme
            }
        }
    "#;

    let response = ctx.execute_unauthenticated(mutation).await;

    assert!(
        !response.errors.is_empty(),
        "Should error when unauthenticated"
    );
    let error_msg = response.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("auth"),
        "Error should mention authentication: {}",
        response.errors[0].message
    );

    ctx.cleanup().await;
}

// =============================================================================
// Preferences Persistence Tests
// =============================================================================

#[tokio::test]
async fn test_preferences_persist_across_queries() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    // Update preferences
    let update_mutation = r#"
        mutation {
            updatePreferences(input: {
                theme: "light",
                quality: "medium",
                crossfadeDurationMs: 2500
            }) {
                preferences { theme }
            }
        }
    "#;

    let response = ctx.execute_authenticated(update_mutation).await;
    assert!(response.errors.is_empty(), "Update should succeed");

    // Get preferences to verify persistence
    let get_mutation = r#"
        mutation {
            getPreferences {
                theme
                quality
                crossfadeDurationMs
            }
        }
    "#;

    let response = ctx.execute_authenticated(get_mutation).await;

    assert!(response.errors.is_empty(), "Get should succeed");

    let data = response.data.into_json().unwrap();
    let prefs = &data["getPreferences"];

    assert_eq!(prefs["theme"].as_str().unwrap(), "light");
    assert_eq!(prefs["quality"].as_str().unwrap(), "medium");
    assert_eq!(prefs["crossfadeDurationMs"].as_u64().unwrap(), 2500);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preserves_unset_fields() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    // First update: set theme
    let mutation1 = r#"
        mutation {
            updatePreferences(input: { theme: "light" }) {
                preferences { theme quality }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation1).await;
    assert!(response.errors.is_empty());

    let data = response.data.into_json().unwrap();
    assert_eq!(
        data["updatePreferences"]["preferences"]["theme"]
            .as_str()
            .unwrap(),
        "light"
    );
    assert_eq!(
        data["updatePreferences"]["preferences"]["quality"]
            .as_str()
            .unwrap(),
        "high"
    );

    // Second update: set quality (theme should remain "light")
    let mutation2 = r#"
        mutation {
            updatePreferences(input: { quality: "lossless" }) {
                preferences { theme quality }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation2).await;
    assert!(response.errors.is_empty());

    let data = response.data.into_json().unwrap();
    assert_eq!(
        data["updatePreferences"]["preferences"]["theme"]
            .as_str()
            .unwrap(),
        "light" // Should still be "light" from first update
    );
    assert_eq!(
        data["updatePreferences"]["preferences"]["quality"]
            .as_str()
            .unwrap(),
        "lossless"
    );

    ctx.cleanup().await;
}

// =============================================================================
// All Valid Quality Values Tests
// =============================================================================

#[tokio::test]
async fn test_update_preferences_all_valid_qualities() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let valid_qualities = ["low", "medium", "high", "lossless"];

    for quality in valid_qualities {
        let mutation = format!(
            r#"
            mutation {{
                updatePreferences(input: {{ quality: "{}" }}) {{
                    preferences {{ quality }}
                }}
            }}
            "#,
            quality
        );

        let response = ctx.execute_authenticated(&mutation).await;

        assert!(
            response.errors.is_empty(),
            "Quality '{}' should be valid: {:?}",
            quality,
            response.errors
        );

        let data = response.data.into_json().unwrap();
        assert_eq!(
            data["updatePreferences"]["preferences"]["quality"]
                .as_str()
                .unwrap(),
            quality
        );
    }

    ctx.cleanup().await;
}

// =============================================================================
// All Valid Theme Values Tests
// =============================================================================

#[tokio::test]
async fn test_update_preferences_all_valid_themes() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let valid_themes = ["dark", "light"];

    for theme in valid_themes {
        let mutation = format!(
            r#"
            mutation {{
                updatePreferences(input: {{ theme: "{}" }}) {{
                    preferences {{ theme }}
                }}
            }}
            "#,
            theme
        );

        let response = ctx.execute_authenticated(&mutation).await;

        assert!(
            response.errors.is_empty(),
            "Theme '{}' should be valid: {:?}",
            theme,
            response.errors
        );

        let data = response.data.into_json().unwrap();
        assert_eq!(
            data["updatePreferences"]["preferences"]["theme"]
                .as_str()
                .unwrap(),
            theme
        );
    }

    ctx.cleanup().await;
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[tokio::test]
async fn test_update_preferences_boundary_crossfade_values() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    // Test boundary values
    let boundary_values = [0_u64, 1, 1000, 6000, 11999, 12000];

    for value in boundary_values {
        let mutation = format!(
            r#"
            mutation {{
                updatePreferences(input: {{ crossfadeDurationMs: {} }}) {{
                    preferences {{ crossfadeDurationMs }}
                }}
            }}
            "#,
            value
        );

        let response = ctx.execute_authenticated(&mutation).await;

        assert!(
            response.errors.is_empty(),
            "Crossfade {} should be valid: {:?}",
            value,
            response.errors
        );

        let data = response.data.into_json().unwrap();
        assert_eq!(
            data["updatePreferences"]["preferences"]["crossfadeDurationMs"]
                .as_u64()
                .unwrap(),
            value
        );
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_update_preferences_crossfade_just_over_max() {
    require_db!(pool);
    let ctx = PreferencesTestContext::new(pool).await;

    let mutation = r#"
        mutation {
            updatePreferences(input: { crossfadeDurationMs: 12001 }) {
                preferences { crossfadeDurationMs }
            }
        }
    "#;

    let response = ctx.execute_authenticated(mutation).await;

    assert!(
        !response.errors.is_empty(),
        "Crossfade 12001 should be rejected"
    );

    ctx.cleanup().await;
}
