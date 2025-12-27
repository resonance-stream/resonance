//! Smart prefetch job
//!
//! Prefetches upcoming tracks for autoplay and caches them in Redis
//! for faster streaming and reduced database load.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::WorkerResult;
use crate::AppState;

/// Prefetch job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchJob {
    /// User ID to prefetch tracks for
    pub user_id: Uuid,

    /// Current track ID (to predict next tracks)
    pub current_track_id: i64,

    /// Number of tracks to prefetch
    pub prefetch_count: Option<usize>,

    /// Whether this is for queue-based prefetch or autoplay
    pub is_autoplay: bool,
}

impl PrefetchJob {
    pub fn for_autoplay(user_id: Uuid, current_track_id: i64) -> Self {
        Self {
            user_id,
            current_track_id,
            prefetch_count: Some(5),
            is_autoplay: true,
        }
    }
}

/// Track ID record
#[derive(Debug, sqlx::FromRow)]
struct TrackIdRecord {
    id: i64,
}

/// Queue item record
#[derive(Debug, sqlx::FromRow)]
struct QueueItemRecord {
    track_id: i64,
}

/// Execute the prefetch job
pub async fn execute(state: &AppState, job: &PrefetchJob) -> WorkerResult<()> {
    let prefetch_count = job.prefetch_count.unwrap_or(5);

    tracing::info!(
        "Prefetching {} tracks for user {} (current track: {})",
        prefetch_count,
        job.user_id,
        job.current_track_id
    );

    let tracks = if job.is_autoplay {
        predict_next_tracks(state, job.user_id, job.current_track_id, prefetch_count).await?
    } else {
        get_queue_tracks(state, job.user_id, prefetch_count).await?
    };

    // Cache track metadata in Redis for quick access
    cache_tracks(state, job.user_id, &tracks).await?;

    tracing::info!("Prefetched {} tracks", tracks.len());

    Ok(())
}

/// Predict next tracks for autoplay based on current track and user preferences
async fn predict_next_tracks(
    state: &AppState,
    user_id: Uuid,
    current_track_id: i64,
    count: usize,
) -> WorkerResult<Vec<i64>> {
    // TODO: Implement smart prediction
    // 1. Get current track's features and embedding
    // 2. Find similar tracks using pgvector
    // 3. Factor in user's listening history
    // 4. Return ranked list of predicted tracks

    // Placeholder: Simple similarity query
    let tracks: Vec<TrackIdRecord> = sqlx::query_as(
        r#"
        SELECT t2.id
        FROM tracks t1
        JOIN tracks t2 ON t1.id != t2.id
        WHERE t1.id = $1
          AND t2.id NOT IN (
            SELECT track_id FROM listening_history
            WHERE user_id = $2
              AND played_at > NOW() - INTERVAL '7 days'
          )
        ORDER BY RANDOM()
        LIMIT $3
        "#
    )
    .bind(current_track_id)
    .bind(user_id)
    .bind(count as i64)
    .fetch_all(&state.db)
    .await?;

    Ok(tracks.into_iter().map(|t| t.id).collect())
}

/// Get tracks from user's queue
async fn get_queue_tracks(state: &AppState, user_id: Uuid, count: usize) -> WorkerResult<Vec<i64>> {
    let tracks: Vec<QueueItemRecord> = sqlx::query_as(
        r#"
        SELECT track_id
        FROM queue_items
        WHERE user_id = $1
        ORDER BY position ASC
        LIMIT $2
        "#
    )
    .bind(user_id)
    .bind(count as i64)
    .fetch_all(&state.db)
    .await?;

    Ok(tracks.into_iter().map(|t| t.track_id).collect())
}

/// Cache track metadata in Redis
async fn cache_tracks(state: &AppState, user_id: Uuid, track_ids: &[i64]) -> WorkerResult<()> {
    if track_ids.is_empty() {
        return Ok(());
    }

    let mut conn = state.redis.get_multiplexed_async_connection().await?;

    for track_id in track_ids {
        // TODO: Fetch full track metadata and cache it
        let cache_key = format!("prefetch:{}:{}", user_id, track_id);

        // Set with 1 hour TTL
        let _: () = redis::cmd("SETEX")
            .arg(&cache_key)
            .arg(3600)
            .arg(track_id.to_string())
            .query_async(&mut conn)
            .await?;
    }

    Ok(())
}
