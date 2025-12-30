//! Smart prefetch job
//!
//! Prefetches upcoming tracks for autoplay and caches them in Redis
//! for faster streaming and reduced database load. Uses pgvector
//! embeddings and audio features for intelligent track prediction.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{WorkerError, WorkerResult};
use crate::AppState;

/// Prefetch job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchJob {
    /// User ID to prefetch tracks for
    pub user_id: Uuid,

    /// Current track ID (to predict next tracks)
    pub current_track_id: Uuid,

    /// Number of tracks to prefetch
    pub prefetch_count: Option<usize>,

    /// Whether this is for queue-based prefetch or autoplay
    pub is_autoplay: bool,
}

impl PrefetchJob {
    /// Create a prefetch job for autoplay prediction
    #[allow(dead_code)]
    pub fn for_autoplay(user_id: Uuid, current_track_id: Uuid) -> Self {
        Self {
            user_id,
            current_track_id,
            prefetch_count: Some(5),
            is_autoplay: true,
        }
    }

    /// Create a prefetch job for explicit queue (not autoplay)
    ///
    /// This prefetches tracks from the user's explicit play queue stored
    /// in the queue_items table, rather than AI-predicted tracks.
    #[allow(dead_code)]
    pub fn for_queue(user_id: Uuid, current_track_id: Uuid) -> Self {
        Self {
            user_id,
            current_track_id,
            prefetch_count: Some(5),
            is_autoplay: false,
        }
    }
}

/// Track ID record for UUID-based queries
#[derive(Debug, sqlx::FromRow)]
struct TrackIdRecord {
    id: Uuid,
}

/// Cached track metadata for prefetch
///
/// This struct contains the essential track information needed for
/// quick playback initialization without hitting the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CachedTrackMetadata {
    /// Track UUID
    pub id: Uuid,

    /// Track title
    pub title: String,

    /// Artist name (denormalized for quick access)
    pub artist_name: Option<String>,

    /// Album title (denormalized for quick access)
    pub album_title: Option<String>,

    /// Track duration in milliseconds
    pub duration_ms: Option<i32>,

    /// Path to the audio file
    pub file_path: String,

    /// Audio file format (e.g., "flac", "mp3")
    /// Stored as string for JSON serialization (cast from database enum)
    pub file_format: String,
}

// =============================================================================
// Similarity Weight Constants
// =============================================================================

/// Weight for semantic (embedding) similarity in combined scoring
const WEIGHT_SEMANTIC: f64 = 0.5;

/// Weight for acoustic (audio feature) similarity in combined scoring
const WEIGHT_ACOUSTIC: f64 = 0.3;

/// Weight for categorical (genre/mood/tag) similarity in combined scoring
const WEIGHT_CATEGORICAL: f64 = 0.2;

/// Weight for audio features in fallback mode (no embeddings)
const WEIGHT_FALLBACK_FEATURE: f64 = 0.6;

/// Weight for tags in fallback mode (no embeddings)
const WEIGHT_FALLBACK_TAGS: f64 = 0.4;

/// BPM normalization factor (typical BPM range: 60-200)
/// Dividing by 200 normalizes BPM difference to roughly [0, 1] range
const BPM_NORMALIZATION_FACTOR: f64 = 200.0;

/// Loudness normalization offset (typical loudness range: -60 to 0 dB)
const LOUDNESS_OFFSET: f64 = 60.0;

// =============================================================================
// Job Execution
// =============================================================================

/// Execute the prefetch job
pub async fn execute(state: &AppState, job: &PrefetchJob) -> WorkerResult<()> {
    let prefetch_count = job.prefetch_count.unwrap_or(5);

    tracing::info!(
        user_id = %job.user_id,
        current_track_id = %job.current_track_id,
        prefetch_count = prefetch_count,
        is_autoplay = job.is_autoplay,
        "Starting prefetch job"
    );

    // Branch based on prefetch mode
    let tracks = if job.is_autoplay {
        // AI-predicted autoplay: find similar tracks
        predict_next_tracks(state, job.user_id, job.current_track_id, prefetch_count).await?
    } else {
        // Explicit queue: fetch upcoming tracks from queue_items table
        fetch_queue_tracks(state, job.user_id, prefetch_count).await?
    };

    if tracks.is_empty() {
        tracing::debug!(
            user_id = %job.user_id,
            is_autoplay = job.is_autoplay,
            "No tracks to prefetch"
        );
        return Ok(());
    }

    // Cache track metadata in Redis for quick access
    cache_tracks(state, job.user_id, &tracks).await?;

    // For queue-based prefetch, mark tracks as prefetched in the database
    if !job.is_autoplay {
        mark_queue_prefetched(state, job.user_id, &tracks).await?;
    }

    tracing::info!(
        user_id = %job.user_id,
        track_count = tracks.len(),
        is_autoplay = job.is_autoplay,
        "Prefetch job completed"
    );

    Ok(())
}

/// Predict next tracks for autoplay based on current track and user preferences.
///
/// Uses a combined similarity approach:
/// - Semantic similarity (50%): pgvector embedding cosine distance
/// - Acoustic similarity (30%): Audio feature Euclidean distance
/// - Categorical similarity (20%): Genre, mood, and AI tag overlap
///
/// Falls back to audio features + tags only if embeddings aren't available.
async fn predict_next_tracks(
    state: &AppState,
    user_id: Uuid,
    current_track_id: Uuid,
    count: usize,
) -> WorkerResult<Vec<Uuid>> {
    // Ensure the source track exists; otherwise similarity CTEs return zero rows silently
    let track_exists: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM tracks WHERE id = $1)")
        .bind(current_track_id)
        .fetch_one(&state.db)
        .await?;

    if !track_exists.0 {
        return Err(WorkerError::TrackNotFound(current_track_id));
    }

    // Check if current track has embeddings
    let has_embedding: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM track_embeddings WHERE track_id = $1 AND description_embedding IS NOT NULL)",
    )
    .bind(current_track_id)
    .fetch_one(&state.db)
    .await?;

    if has_embedding.0 {
        tracing::debug!(
            track_id = %current_track_id,
            "Using combined similarity (embedding + features + tags)"
        );
        predict_by_combined_similarity(state, user_id, current_track_id, count).await
    } else {
        tracing::debug!(
            track_id = %current_track_id,
            "Falling back to audio feature similarity (no embedding)"
        );
        predict_by_features(state, user_id, current_track_id, count).await
    }
}

/// Predict tracks using combined similarity (embedding + features + tags)
///
/// This aligns with the SimilarityService in apps/api/src/services/similarity.rs
/// using the same scoring approach for consistency.
async fn predict_by_combined_similarity(
    state: &AppState,
    user_id: Uuid,
    current_track_id: Uuid,
    count: usize,
) -> WorkerResult<Vec<Uuid>> {
    // Combined similarity query using all three dimensions:
    // - Semantic: pgvector cosine distance on description embeddings
    // - Acoustic: Euclidean distance on normalized audio features (energy, loudness, valence, danceability, bpm)
    // - Categorical: Weighted Jaccard similarity on genres, moods, and ai_tags
    //
    // The score is clamped to [0.0, 1.0] range for consistency.
    let tracks: Vec<TrackIdRecord> = sqlx::query_as(
        r#"
        WITH embedding_scores AS (
            SELECT
                t.id,
                -- Clamp cosine similarity to [0, 1] range
                GREATEST(0.0, LEAST(1.0, 1.0 - (te.description_embedding <=> source.description_embedding))) as score
            FROM track_embeddings te
            JOIN track_embeddings source ON source.track_id = $1
            JOIN tracks t ON t.id = te.track_id
            WHERE te.track_id != $1
              AND te.description_embedding IS NOT NULL
        ),
        source_track AS (
            SELECT
                (audio_features->>'energy')::float as energy,
                (audio_features->>'loudness')::float as loudness,
                (audio_features->>'valence')::float as valence,
                (audio_features->>'danceability')::float as danceability,
                (audio_features->>'bpm')::float as bpm,
                genres,
                ai_mood,
                ai_tags
            FROM tracks
            WHERE id = $1
        ),
        feature_scores AS (
            SELECT
                t.id,
                -- Euclidean distance on normalized features, clamped to [0, 1]
                GREATEST(0.0, LEAST(1.0, 1.0 - (
                    SQRT(
                        COALESCE(POWER((t.audio_features->>'energy')::float - src.energy, 2), 0) +
                        COALESCE(POWER(((t.audio_features->>'loudness')::float + $7) / $7 - (src.loudness + $7) / $7, 2), 0) +
                        COALESCE(POWER((t.audio_features->>'valence')::float - src.valence, 2), 0) +
                        COALESCE(POWER((t.audio_features->>'danceability')::float - src.danceability, 2), 0) +
                        COALESCE(POWER(((t.audio_features->>'bpm')::float - src.bpm) / $8, 2), 0)
                    ) / 2.0
                ))) as score
            FROM tracks t
            CROSS JOIN source_track src
            WHERE t.id != $1
              AND t.audio_features->>'energy' IS NOT NULL
        ),
        tag_scores AS (
            SELECT
                t.id,
                -- Weighted Jaccard similarity: mood weighted 2x (more specific than genre)
                -- Includes ai_tags for consistency with SimilarityService
                (
                    COALESCE(array_length(t.genres & src.genres, 1), 0) +
                    COALESCE(array_length(t.ai_mood & src.ai_mood, 1), 0) * 2 +
                    COALESCE(array_length(t.ai_tags & src.ai_tags, 1), 0)
                )::float / GREATEST(1,
                    COALESCE(array_length(t.genres | src.genres, 1), 0) +
                    COALESCE(array_length(t.ai_mood | src.ai_mood, 1), 0) * 2 +
                    COALESCE(array_length(t.ai_tags | src.ai_tags, 1), 0)
                ) as score
            FROM tracks t
            CROSS JOIN source_track src
            WHERE t.id != $1
        ),
        combined_scores AS (
            -- FULL OUTER JOIN allows tracks without embeddings to still be recommended
            -- based on audio features and/or tags alone
            SELECT
                COALESCE(e.id, f.id, g.id) as id,
                (
                    COALESCE(e.score, 0) * $4 +
                    COALESCE(f.score, 0) * $5 +
                    COALESCE(g.score, 0) * $6
                ) as combined_score
            FROM embedding_scores e
            FULL OUTER JOIN feature_scores f ON e.id = f.id
            FULL OUTER JOIN tag_scores g ON COALESCE(e.id, f.id) = g.id
        )
        SELECT cs.id
        FROM combined_scores cs
        JOIN tracks t ON t.id = cs.id
        WHERE NOT EXISTS (
            SELECT 1 FROM listening_history lh
            WHERE lh.user_id = $2
              AND lh.played_at > NOW() - INTERVAL '24 hours'
              AND lh.track_id = cs.id
        )
        ORDER BY cs.combined_score DESC
        LIMIT $3
        "#,
    )
    .bind(current_track_id)
    .bind(user_id)
    .bind(count as i64)
    .bind(WEIGHT_SEMANTIC)
    .bind(WEIGHT_ACOUSTIC)
    .bind(WEIGHT_CATEGORICAL)
    .bind(LOUDNESS_OFFSET)
    .bind(BPM_NORMALIZATION_FACTOR)
    .fetch_all(&state.db)
    .await?;

    Ok(tracks.into_iter().map(|t| t.id).collect())
}

/// Predict tracks using audio feature + tag similarity only (fallback when no embeddings)
async fn predict_by_features(
    state: &AppState,
    user_id: Uuid,
    current_track_id: Uuid,
    count: usize,
) -> WorkerResult<Vec<Uuid>> {
    let tracks: Vec<TrackIdRecord> = sqlx::query_as(
        r#"
        WITH source_track AS (
            SELECT
                (audio_features->>'energy')::float as energy,
                (audio_features->>'loudness')::float as loudness,
                (audio_features->>'valence')::float as valence,
                (audio_features->>'danceability')::float as danceability,
                (audio_features->>'bpm')::float as bpm,
                genres,
                ai_mood,
                ai_tags
            FROM tracks
            WHERE id = $1
        ),
        feature_scores AS (
            SELECT
                t.id,
                -- Euclidean distance on normalized features, clamped to [0, 1]
                -- No COALESCE needed since WHERE clause ensures all features are non-NULL
                GREATEST(0.0, LEAST(1.0, 1.0 - (
                    SQRT(
                        POWER((t.audio_features->>'energy')::float - src.energy, 2) +
                        POWER(((t.audio_features->>'loudness')::float + $4) / $4 - (src.loudness + $4) / $4, 2) +
                        POWER((t.audio_features->>'valence')::float - src.valence, 2) +
                        POWER((t.audio_features->>'danceability')::float - src.danceability, 2) +
                        POWER(((t.audio_features->>'bpm')::float - src.bpm) / $5, 2)
                    ) / 2.0
                ))) as feature_score,
                -- Weighted Jaccard similarity including ai_tags
                (
                    COALESCE(array_length(t.genres & src.genres, 1), 0) +
                    COALESCE(array_length(t.ai_mood & src.ai_mood, 1), 0) * 2 +
                    COALESCE(array_length(t.ai_tags & src.ai_tags, 1), 0)
                )::float / GREATEST(1,
                    COALESCE(array_length(t.genres | src.genres, 1), 0) +
                    COALESCE(array_length(t.ai_mood | src.ai_mood, 1), 0) * 2 +
                    COALESCE(array_length(t.ai_tags | src.ai_tags, 1), 0)
                ) as tag_score
            FROM tracks t
            CROSS JOIN source_track src
            WHERE t.id != $1
              -- Ensure source track has complete audio features
              AND src.energy IS NOT NULL
              AND src.loudness IS NOT NULL
              AND src.valence IS NOT NULL
              AND src.danceability IS NOT NULL
              AND src.bpm IS NOT NULL
              -- Ensure candidate track has complete audio features
              AND t.audio_features->>'energy' IS NOT NULL
              AND t.audio_features->>'loudness' IS NOT NULL
              AND t.audio_features->>'valence' IS NOT NULL
              AND t.audio_features->>'danceability' IS NOT NULL
              AND t.audio_features->>'bpm' IS NOT NULL
        )
        SELECT fs.id
        FROM feature_scores fs
        WHERE NOT EXISTS (
            SELECT 1 FROM listening_history lh
            WHERE lh.user_id = $2
              AND lh.played_at > NOW() - INTERVAL '24 hours'
              AND lh.track_id = fs.id
        )
        ORDER BY (fs.feature_score * $6 + fs.tag_score * $7) DESC
        LIMIT $3
        "#,
    )
    .bind(current_track_id)
    .bind(user_id)
    .bind(count as i64)
    .bind(LOUDNESS_OFFSET)
    .bind(BPM_NORMALIZATION_FACTOR)
    .bind(WEIGHT_FALLBACK_FEATURE)
    .bind(WEIGHT_FALLBACK_TAGS)
    .fetch_all(&state.db)
    .await?;

    Ok(tracks.into_iter().map(|t| t.id).collect())
}

/// Cache TTL in seconds (30 minutes)
///
/// Prefetch data is typically consumed within seconds to minutes.
/// A shorter TTL reduces memory waste while still supporting pause-resume scenarios.
const CACHE_TTL_SECONDS: i64 = 30 * 60; // 30 minutes

/// Cache track metadata in Redis for quick access during playback.
///
/// Fetches full track metadata in a single batch query and caches
/// each track as JSON using a Redis pipeline for efficiency.
async fn cache_tracks(state: &AppState, user_id: Uuid, track_ids: &[Uuid]) -> WorkerResult<()> {
    if track_ids.is_empty() {
        return Ok(());
    }

    // Batch fetch track metadata in a single query, preserving the ranking order
    // from predict_next_tracks() using UNNEST WITH ORDINALITY.
    // NOTE: Future optimization - use shared track cache (track:meta:{id}) instead of
    // per-user keys to reduce memory duplication for popular tracks across users.
    let tracks: Vec<CachedTrackMetadata> = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.title,
            ar.name as artist_name,
            al.title as album_title,
            t.duration_ms,
            t.file_path,
            t.file_format::text as file_format
        FROM UNNEST($1::uuid[]) WITH ORDINALITY AS u(id, ord)
        JOIN tracks t ON t.id = u.id
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        ORDER BY u.ord
        "#,
    )
    .bind(track_ids)
    .fetch_all(&state.db)
    .await?;

    if tracks.is_empty() {
        tracing::debug!(
            user_id = %user_id,
            track_count = track_ids.len(),
            "No tracks found to cache"
        );
        return Ok(());
    }

    // Use Redis pipeline for efficient multi-set
    let mut conn = state.redis.get_multiplexed_async_connection().await?;
    let mut pipe = redis::pipe();

    for track in &tracks {
        let cache_key = format!("prefetch:{}:{}", user_id, track.id);
        let json = serde_json::to_string(track).map_err(|e| {
            WorkerError::Internal(format!(
                "Failed to serialize track metadata for track {}: {}",
                track.id, e
            ))
        })?;

        pipe.cmd("SETEX")
            .arg(&cache_key)
            .arg(CACHE_TTL_SECONDS)
            .arg(json);
    }

    // Execute pipeline (ignore result values, we just need to know it succeeded)
    let _: () = pipe.query_async(&mut conn).await?;

    tracing::debug!(
        user_id = %user_id,
        cached_count = tracks.len(),
        "Cached track metadata in Redis"
    );

    Ok(())
}

// =============================================================================
// Queue-Based Prefetch Functions
// =============================================================================

/// Fetch upcoming tracks from the user's explicit queue.
///
/// Queries the queue_items table for tracks that:
/// 1. Are after the current index (upcoming)
/// 2. Haven't been prefetched yet (metadata->'prefetched' IS NOT 'true')
///
/// Uses the partial index `idx_queue_items_unprefetched` for efficient queries.
async fn fetch_queue_tracks(
    state: &AppState,
    user_id: Uuid,
    count: usize,
) -> WorkerResult<Vec<Uuid>> {
    // Query upcoming unprefetched tracks from queue_items
    // Join with queue_state to get current_index
    let tracks: Vec<TrackIdRecord> = sqlx::query_as(
        r#"
        SELECT qi.track_id as id
        FROM queue_items qi
        JOIN queue_state qs ON qs.user_id = qi.user_id
        WHERE qi.user_id = $1
          AND qi.position > qs.current_index
          AND qi.metadata->>'prefetched' IS DISTINCT FROM 'true'
        ORDER BY qi.position ASC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(count as i64)
    .fetch_all(&state.db)
    .await?;

    tracing::debug!(
        user_id = %user_id,
        track_count = tracks.len(),
        "Fetched queue tracks for prefetch"
    );

    Ok(tracks.into_iter().map(|t| t.id).collect())
}

/// Mark tracks as prefetched in the queue_items metadata.
///
/// Updates the metadata JSONB column to set prefetched = true for the given tracks.
/// This prevents re-prefetching tracks that have already been cached.
async fn mark_queue_prefetched(
    state: &AppState,
    user_id: Uuid,
    track_ids: &[Uuid],
) -> WorkerResult<()> {
    if track_ids.is_empty() {
        return Ok(());
    }

    // Update metadata to mark tracks as prefetched
    // Using jsonb_set to preserve any existing metadata
    let result = sqlx::query(
        r#"
        UPDATE queue_items
        SET metadata = COALESCE(metadata, '{}'::jsonb) || '{"prefetched": true}'::jsonb
        WHERE user_id = $1
          AND track_id = ANY($2)
        "#,
    )
    .bind(user_id)
    .bind(track_ids)
    .execute(&state.db)
    .await?;

    tracing::debug!(
        user_id = %user_id,
        updated_count = result.rows_affected(),
        track_count = track_ids.len(),
        "Marked queue tracks as prefetched"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weights_sum_to_one() {
        let total = WEIGHT_SEMANTIC + WEIGHT_ACOUSTIC + WEIGHT_CATEGORICAL;
        assert!(
            (total - 1.0).abs() < f64::EPSILON,
            "Combined weights should sum to 1.0, got {}",
            total
        );
    }

    #[test]
    fn test_fallback_weights_sum_to_one() {
        let total = WEIGHT_FALLBACK_FEATURE + WEIGHT_FALLBACK_TAGS;
        assert!(
            (total - 1.0).abs() < f64::EPSILON,
            "Fallback weights should sum to 1.0, got {}",
            total
        );
    }

    #[test]
    fn test_prefetch_job_for_autoplay() {
        let user_id = Uuid::new_v4();
        let track_id = Uuid::new_v4();
        let job = PrefetchJob::for_autoplay(user_id, track_id);

        assert_eq!(job.user_id, user_id);
        assert_eq!(job.current_track_id, track_id);
        assert_eq!(job.prefetch_count, Some(5));
        assert!(job.is_autoplay);
    }

    #[test]
    fn test_prefetch_job_for_queue() {
        let user_id = Uuid::new_v4();
        let track_id = Uuid::new_v4();
        let job = PrefetchJob::for_queue(user_id, track_id);

        assert_eq!(job.user_id, user_id);
        assert_eq!(job.current_track_id, track_id);
        assert_eq!(job.prefetch_count, Some(5));
        assert!(!job.is_autoplay); // Queue-based prefetch is NOT autoplay
    }

    #[test]
    fn test_normalization_constants() {
        // BPM normalization: 200 BPM difference should normalize to ~1.0
        assert!((BPM_NORMALIZATION_FACTOR - 200.0).abs() < f64::EPSILON);

        // Loudness normalization: -60 dB to 0 dB range
        assert!((LOUDNESS_OFFSET - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_ttl() {
        // Cache TTL should be 30 minutes (1800 seconds)
        assert_eq!(CACHE_TTL_SECONDS, 1800);
    }

    #[test]
    fn test_cached_track_metadata_serialization() {
        let track = CachedTrackMetadata {
            id: Uuid::nil(),
            title: "Test Track".to_string(),
            artist_name: Some("Test Artist".to_string()),
            album_title: Some("Test Album".to_string()),
            duration_ms: Some(180000),
            file_path: "/music/test.flac".to_string(),
            file_format: "flac".to_string(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&track).expect("should serialize");
        assert!(json.contains("Test Track"));
        assert!(json.contains("Test Artist"));
        assert!(json.contains("flac"));

        // Deserialize back
        let deserialized: CachedTrackMetadata =
            serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(deserialized.id, track.id);
        assert_eq!(deserialized.title, track.title);
        assert_eq!(deserialized.artist_name, track.artist_name);
        assert_eq!(deserialized.file_format, track.file_format);
    }

    #[test]
    fn test_cached_track_metadata_optional_fields() {
        let track = CachedTrackMetadata {
            id: Uuid::nil(),
            title: "Minimal Track".to_string(),
            artist_name: None,
            album_title: None,
            duration_ms: None,
            file_path: "/music/unknown.mp3".to_string(),
            file_format: "mp3".to_string(),
        };

        let json = serde_json::to_string(&track).expect("should serialize");
        let deserialized: CachedTrackMetadata =
            serde_json::from_str(&json).expect("should deserialize");

        assert!(deserialized.artist_name.is_none());
        assert!(deserialized.album_title.is_none());
        assert!(deserialized.duration_ms.is_none());
        assert_eq!(deserialized.file_format, "mp3");
    }
}
