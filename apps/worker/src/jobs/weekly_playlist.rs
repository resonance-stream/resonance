//! Weekly playlist generation job
//!
//! Generates personalized "Discover Weekly" style playlists for users
//! based on their listening history and AI recommendations.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::WorkerResult;
use crate::AppState;

/// Weekly playlist generation job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyPlaylistJob {
    /// User ID to generate playlist for
    /// If None, generates for all users
    pub user_id: Option<Uuid>,

    /// Number of tracks to include in the playlist
    pub track_count: Option<usize>,
}

impl Default for WeeklyPlaylistJob {
    fn default() -> Self {
        Self {
            user_id: None,
            track_count: Some(30),
        }
    }
}

/// User record
#[derive(Debug, sqlx::FromRow)]
struct UserRecord {
    id: Uuid,
}

/// Genre play count
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct GenrePlayCount {
    name: String,
    play_count: i64,
}

/// Execute the weekly playlist generation job
pub async fn execute(state: &AppState, job: &WeeklyPlaylistJob) -> WorkerResult<()> {
    let track_count = job.track_count.unwrap_or(30);

    match job.user_id {
        Some(user_id) => {
            tracing::info!("Generating weekly playlist for user: {}", user_id);
            generate_for_user(state, user_id, track_count).await?;
        }
        None => {
            tracing::info!("Generating weekly playlists for all users");

            // Get all active users
            let users: Vec<UserRecord> = sqlx::query_as(
                "SELECT id FROM users WHERE is_active = true"
            )
            .fetch_all(&state.db)
            .await?;

            for user in users {
                if let Err(e) = generate_for_user(state, user.id, track_count).await {
                    tracing::error!("Failed to generate playlist for user {}: {}", user.id, e);
                }
            }
        }
    }

    tracing::info!("Weekly playlist generation completed");

    Ok(())
}

/// Generate weekly playlist for a specific user
async fn generate_for_user(state: &AppState, user_id: Uuid, track_count: usize) -> WorkerResult<()> {
    tracing::debug!("Processing user: {}", user_id);

    // TODO: Implement playlist generation logic
    // 1. Analyze user's listening history (last 30 days)
    // 2. Find similar tracks based on:
    //    a. Genre preferences
    //    b. Audio feature similarity (BPM, energy, etc.)
    //    c. Vector embedding similarity (pgvector)
    // 3. Filter out recently played tracks
    // 4. Create or update "Discover Weekly" playlist

    // Placeholder: Get user's top genres from listening history
    let _top_genres: Vec<GenrePlayCount> = sqlx::query_as(
        r#"
        SELECT g.name, COUNT(*) as play_count
        FROM listening_history lh
        JOIN tracks t ON lh.track_id = t.id
        JOIN track_genres tg ON t.id = tg.track_id
        JOIN genres g ON tg.genre_id = g.id
        WHERE lh.user_id = $1
          AND lh.played_at > NOW() - INTERVAL '30 days'
        GROUP BY g.name
        ORDER BY play_count DESC
        LIMIT 5
        "#
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    // TODO: Use AI to find similar tracks
    // TODO: Create playlist with discovered tracks

    let _ = track_count; // Suppress unused warning for now

    Ok(())
}
