//! Weekly playlist generation job
//!
//! Generates personalized "Discover Weekly" style playlists for users
//! based on their listening history and AI recommendations using pgvector
//! embeddings for semantic similarity.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::WorkerResult;
use crate::AppState;

// =============================================================================
// Configuration Constants
// =============================================================================

/// Number of seed tracks to use for similarity search
const SEED_TRACK_COUNT: i64 = 10;

/// Number of days to look back for seed tracks
const SEED_HISTORY_DAYS: i32 = 30;

/// Number of days to filter out recently played tracks
const RECENTLY_PLAYED_DAYS: i32 = 7;

/// Minimum completed plays to count as a seed track
const MIN_COMPLETED_PLAYS: i32 = 1;

/// Maximum tracks to include in a playlist (prevents abuse)
const MAX_TRACK_COUNT: usize = 100;

/// Name for the auto-generated discover weekly playlist
const DISCOVER_WEEKLY_NAME: &str = "Discover Weekly";

/// Description for the auto-generated discover weekly playlist
const DISCOVER_WEEKLY_DESCRIPTION: &str =
    "Personalized tracks based on your listening history, updated weekly";

// =============================================================================
// Job Types
// =============================================================================

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

// =============================================================================
// Database Record Types
// =============================================================================

/// User record
#[derive(Debug, sqlx::FromRow)]
struct UserRecord {
    id: Uuid,
}

/// Track ID record for UUID-based queries
#[derive(Debug, sqlx::FromRow)]
struct TrackIdRecord {
    id: Uuid,
}

/// Playlist record
#[derive(Debug, sqlx::FromRow)]
struct PlaylistRecord {
    id: Uuid,
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
            let users: Vec<UserRecord> =
                sqlx::query_as("SELECT id FROM users WHERE is_active = true")
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
///
/// Algorithm:
/// 1. Get user's top 10 most-played tracks from last 30 days (seeds)
/// 2. Find tracks similar to seeds using pgvector embedding similarity
/// 3. Filter out recently played tracks (last 7 days) and seed tracks
/// 4. Find or create "Discover Weekly" playlist
/// 5. Replace playlist tracks with new discoveries
async fn generate_for_user(
    state: &AppState,
    user_id: Uuid,
    track_count: usize,
) -> WorkerResult<()> {
    // Clamp track count to prevent abuse
    let track_count = track_count.min(MAX_TRACK_COUNT);

    tracing::debug!(user_id = %user_id, track_count, "Processing user for weekly playlist");

    // Step 1: Get user's top seed tracks from listening history
    let seed_tracks = get_seed_tracks(state, user_id).await?;

    if seed_tracks.is_empty() {
        // No listening history is a valid state for new users - skip gracefully
        tracing::info!(
            user_id = %user_id,
            "No listening history found, skipping weekly playlist generation"
        );
        return Ok(());
    }

    tracing::debug!(
        user_id = %user_id,
        seed_count = seed_tracks.len(),
        "Found seed tracks for similarity search"
    );

    // Step 2: Find similar tracks based on embeddings
    let similar_tracks =
        find_similar_tracks_for_seeds(state, user_id, &seed_tracks, track_count).await?;

    if similar_tracks.is_empty() {
        tracing::warn!(
            user_id = %user_id,
            "No similar tracks found, skipping playlist update"
        );
        return Ok(());
    }

    tracing::debug!(
        user_id = %user_id,
        track_count = similar_tracks.len(),
        "Found similar tracks for weekly playlist"
    );

    // Step 3: Find or create the Discover Weekly playlist
    let playlist_id = find_or_create_discover_playlist(state, user_id).await?;

    // Step 4: Replace playlist tracks with new discoveries
    replace_playlist_tracks(state, user_id, playlist_id, &similar_tracks).await?;

    tracing::info!(
        user_id = %user_id,
        playlist_id = %playlist_id,
        track_count = similar_tracks.len(),
        "Weekly playlist updated successfully"
    );

    Ok(())
}

/// Get user's top tracks from listening history as seeds for similarity search
async fn get_seed_tracks(state: &AppState, user_id: Uuid) -> WorkerResult<Vec<Uuid>> {
    let seeds: Vec<TrackIdRecord> = sqlx::query_as(
        r#"
        SELECT lh.track_id as id
        FROM listening_history lh
        JOIN track_embeddings te ON te.track_id = lh.track_id
        WHERE lh.user_id = $1
          AND lh.played_at > NOW() - make_interval(days => $2)
          AND lh.completed = true
        GROUP BY lh.track_id
        HAVING COUNT(*) >= $3
        ORDER BY COUNT(*) DESC, MAX(lh.played_at) DESC
        LIMIT $4
        "#,
    )
    .bind(user_id)
    .bind(SEED_HISTORY_DAYS)
    .bind(MIN_COMPLETED_PLAYS)
    .bind(SEED_TRACK_COUNT)
    .fetch_all(&state.db)
    .await?;

    Ok(seeds.into_iter().map(|s| s.id).collect())
}

/// Find tracks similar to seed tracks using pgvector embeddings
///
/// Uses the average embedding of all seed tracks to find similar tracks,
/// filtering out recently played tracks and the seeds themselves.
///
/// NOTE: This uses only description_embedding for similarity. A future improvement
/// would be to adopt the combined similarity approach from prefetch.rs that weights:
/// - Semantic similarity (50%): description_embedding
/// - Acoustic similarity (30%): audio features
/// - Categorical similarity (20%): genres, moods, AI tags
///
/// The average embedding approach may produce suboptimal results for users with
/// eclectic taste, as the centroid may fall between distinct preference clusters.
/// Consider per-seed retrieval with merge for better diversity.
async fn find_similar_tracks_for_seeds(
    state: &AppState,
    user_id: Uuid,
    seed_ids: &[Uuid],
    limit: usize,
) -> WorkerResult<Vec<Uuid>> {
    if seed_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Find tracks similar to the average embedding of all seeds
    // This approach finds tracks that are "generally similar" to user's taste
    let tracks: Vec<TrackIdRecord> = sqlx::query_as(
        r#"
        WITH seed_embeddings AS (
            SELECT AVG(description_embedding) as avg_embedding
            FROM track_embeddings
            WHERE track_id = ANY($1)
              AND description_embedding IS NOT NULL
        ),
        recently_played AS (
            SELECT DISTINCT track_id
            FROM listening_history
            WHERE user_id = $2
              AND played_at > NOW() - make_interval(days => $3)
        )
        SELECT te.track_id as id
        FROM track_embeddings te
        CROSS JOIN seed_embeddings se
        WHERE te.track_id != ALL($1)
          AND te.track_id NOT IN (SELECT track_id FROM recently_played)
          AND te.description_embedding IS NOT NULL
        ORDER BY te.description_embedding <=> se.avg_embedding
        LIMIT $4
        "#,
    )
    .bind(seed_ids)
    .bind(user_id)
    .bind(RECENTLY_PLAYED_DAYS)
    .bind(limit as i64)
    .fetch_all(&state.db)
    .await?;

    Ok(tracks.into_iter().map(|t| t.id).collect())
}

/// Find or create the Discover Weekly playlist for a user
///
/// NOTE: This uses a SELECT-then-INSERT pattern which has a theoretical TOCTOU race
/// condition if two concurrent requests process the same user. In practice, this job
/// runs on a schedule (weekly) and processes users sequentially, so this is not a
/// concern. For true concurrent safety, add a partial unique index:
/// `CREATE UNIQUE INDEX idx_playlists_user_discover ON playlists(user_id, name) WHERE playlist_type = 'discover'`
/// and use INSERT ... ON CONFLICT.
async fn find_or_create_discover_playlist(state: &AppState, user_id: Uuid) -> WorkerResult<Uuid> {
    // Try to find existing discover playlist
    let existing: Option<PlaylistRecord> = sqlx::query_as(
        r#"
        SELECT id
        FROM playlists
        WHERE user_id = $1
          AND playlist_type = 'discover'
          AND name = $2
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(DISCOVER_WEEKLY_NAME)
    .fetch_optional(&state.db)
    .await?;

    if let Some(playlist) = existing {
        tracing::debug!(
            user_id = %user_id,
            playlist_id = %playlist.id,
            "Found existing Discover Weekly playlist"
        );
        return Ok(playlist.id);
    }

    // Create new discover playlist
    let new_playlist: PlaylistRecord = sqlx::query_as(
        r#"
        INSERT INTO playlists (user_id, name, description, playlist_type, is_public)
        VALUES ($1, $2, $3, 'discover', false)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(DISCOVER_WEEKLY_NAME)
    .bind(DISCOVER_WEEKLY_DESCRIPTION)
    .fetch_one(&state.db)
    .await?;

    tracing::info!(
        user_id = %user_id,
        playlist_id = %new_playlist.id,
        "Created new Discover Weekly playlist"
    );

    Ok(new_playlist.id)
}

/// Replace all tracks in a playlist with new tracks
///
/// Uses a transaction to ensure atomicity of the replacement.
/// Batch inserts tracks using UNNEST for efficiency (single round-trip).
async fn replace_playlist_tracks(
    state: &AppState,
    user_id: Uuid,
    playlist_id: Uuid,
    track_ids: &[Uuid],
) -> WorkerResult<()> {
    let mut tx = state.db.begin().await?;

    // Delete existing tracks
    sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = $1")
        .bind(playlist_id)
        .execute(&mut *tx)
        .await?;

    // Batch insert new tracks with positions using UNNEST
    // This is more efficient than individual INSERTs (single round-trip vs N)
    if !track_ids.is_empty() {
        sqlx::query(
            r#"
            INSERT INTO playlist_tracks (playlist_id, track_id, added_by, position)
            SELECT $1, track_id, $2, position::int - 1
            FROM UNNEST($3::uuid[]) WITH ORDINALITY AS t(track_id, position)
            "#,
        )
        .bind(playlist_id)
        .bind(user_id)
        .bind(track_ids)
        .execute(&mut *tx)
        .await?;
    }

    // Note: track_count and total_duration_ms are updated by database trigger

    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_weekly_playlist_job() {
        let job = WeeklyPlaylistJob::default();
        assert!(job.user_id.is_none());
        assert_eq!(job.track_count, Some(30));
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_configuration_constants() {
        // These assertions validate that constants are within expected ranges.
        // They serve as documentation and catch accidental changes to constants.

        // Seed track count should be reasonable (5-20)
        assert!(SEED_TRACK_COUNT >= 5 && SEED_TRACK_COUNT <= 20);

        // History days should cover at least 2 weeks
        assert!(SEED_HISTORY_DAYS >= 14);

        // Recently played filter should be less than seed history
        assert!(RECENTLY_PLAYED_DAYS < SEED_HISTORY_DAYS);

        // Minimum plays should be at least 1
        assert!(MIN_COMPLETED_PLAYS >= 1);

        // Max track count should be reasonable
        assert!(MAX_TRACK_COUNT >= 50 && MAX_TRACK_COUNT <= 200);
    }

    #[test]
    fn test_discover_weekly_metadata() {
        assert!(!DISCOVER_WEEKLY_NAME.is_empty());
        assert!(!DISCOVER_WEEKLY_DESCRIPTION.is_empty());
        assert!(DISCOVER_WEEKLY_DESCRIPTION.len() > 20); // Should be descriptive
    }

    #[test]
    fn test_weekly_playlist_job_serialization() {
        let job = WeeklyPlaylistJob {
            user_id: Some(Uuid::nil()),
            track_count: Some(50),
        };

        let json = serde_json::to_string(&job).expect("should serialize");
        let deserialized: WeeklyPlaylistJob =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.user_id, job.user_id);
        assert_eq!(deserialized.track_count, job.track_count);
    }
}
