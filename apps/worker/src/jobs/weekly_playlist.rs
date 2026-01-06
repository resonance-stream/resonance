//! Weekly playlist generation job
//!
//! Generates personalized "Discover Weekly" style playlists for users
//! based on their listening history and AI recommendations using pgvector
//! embeddings for semantic similarity.
//!
//! This module also supports taste-clustered playlists that supplement
//! the Discover Weekly by using k-means clustering on listening history
//! embeddings to identify distinct taste groups and generate a playlist
//! per cluster using centroid similarity search.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{WorkerError, WorkerResult};
use crate::jobs::clustering::{
    cluster_user_taste_with_metadata, TasteCluster, TrackClusterMetadata,
};
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
// Cluster Playlist Configuration Constants
// =============================================================================

/// Minimum listening history entries required for clustering
/// Below this threshold, clustering likely won't produce meaningful results
const MIN_HISTORY_FOR_CLUSTERING: usize = 20;

// =============================================================================
// Similarity Weight Constants (aligned with prefetch.rs and SimilarityService)
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

/// Number of tracks to include in each cluster playlist
const CLUSTER_PLAYLIST_TRACK_COUNT: usize = 30;

/// Number of days of listening history to consider for clustering
const CLUSTER_HISTORY_DAYS: i32 = 30;

/// Number of days to filter out recently played tracks from cluster playlists
const CLUSTER_RECENTLY_PLAYED_DAYS: i32 = 7;

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

/// Playlist upsert result with created flag
///
/// Used by `find_or_create_discover_playlist` to detect whether a playlist
/// was newly created (xmax=0) or already existed (xmax>0).
#[derive(Debug, sqlx::FromRow)]
struct PlaylistUpsertResult {
    id: Uuid,
    created: bool,
}

/// Track embedding with metadata for clustering
///
/// The embedding is stored as a string representation from PostgreSQL's
/// pgvector type (e.g., "[0.1, 0.2, 0.3]") and parsed into Vec<f32>.
#[derive(Debug, sqlx::FromRow)]
struct TrackEmbeddingWithMetadata {
    track_id: Uuid,
    /// Embedding as string "[f32, f32, ...]" from vector::text
    description_embedding: String,
    mood: Option<String>,
    genre: Option<String>,
    energy: Option<f32>,
    valence: Option<f32>,
}

/// Cluster upsert result
#[derive(Debug, sqlx::FromRow)]
struct ClusterUpsertResult {
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

    // Step 5: Generate taste-clustered playlists (supplementary)
    if let Err(e) = generate_cluster_playlists(state, user_id, track_count).await {
        tracing::warn!(
            user_id = %user_id,
            error = %e,
            "Failed to generate cluster playlists"
        );
        // Don't fail the whole job, cluster playlists are supplementary
    }

    Ok(())
}

/// Get user's top tracks from listening history as seeds for similarity search
async fn get_seed_tracks(state: &AppState, user_id: Uuid) -> WorkerResult<Vec<Uuid>> {
    let seeds: Vec<TrackIdRecord> = sqlx::query_as(
        r#"
        SELECT lh.track_id as id
        FROM listening_history lh
        WHERE lh.user_id = $1
          AND lh.played_at > NOW() - make_interval(days => $2)
          AND lh.completed = true
          AND EXISTS (
              SELECT 1
              FROM track_embeddings te
              WHERE te.track_id = lh.track_id
                AND te.description_embedding IS NOT NULL
          )
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
/// NOTE: This uses only description_embedding for similarity. The cluster-based
/// playlists (via find_similar_tracks_for_cluster) use the full combined similarity
/// approach with embedding + audio features + categorical metadata. Consider
/// adopting the same approach here for consistency.
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
          AND NOT EXISTS (SELECT 1 FROM recently_played rp WHERE rp.track_id = te.track_id)
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
/// Uses an atomic upsert pattern with INSERT ON CONFLICT to prevent TOCTOU race
/// conditions. The partial unique index `idx_playlists_user_discover_weekly` ensures
/// only one discover playlist per user/name combination exists.
///
/// The `(xmax = 0) AS created` pattern detects whether the row was inserted (xmax=0)
/// or updated (xmax>0), allowing accurate logging without additional queries.
async fn find_or_create_discover_playlist(state: &AppState, user_id: Uuid) -> WorkerResult<Uuid> {
    // Atomic upsert: insert or update if exists
    // The partial unique index on (user_id, name) WHERE playlist_type = 'discover'
    // ensures this is race-condition free
    let result: PlaylistUpsertResult = sqlx::query_as(
        r#"
        INSERT INTO playlists (user_id, name, description, playlist_type, is_public)
        VALUES ($1, $2, $3, 'discover', false)
        ON CONFLICT (user_id, name) WHERE playlist_type = 'discover'
        DO UPDATE SET updated_at = NOW()
        RETURNING id, (xmax = 0) AS created
        "#,
    )
    .bind(user_id)
    .bind(DISCOVER_WEEKLY_NAME)
    .bind(DISCOVER_WEEKLY_DESCRIPTION)
    .fetch_one(&state.db)
    .await?;

    if result.created {
        tracing::info!(
            user_id = %user_id,
            playlist_id = %result.id,
            "Created new Discover Weekly playlist"
        );
    } else {
        tracing::debug!(
            user_id = %user_id,
            playlist_id = %result.id,
            "Found existing Discover Weekly playlist"
        );
    }

    Ok(result.id)
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

    // Ensure we only ever mutate the requesting user's Discover Weekly playlist.
    // Locks the playlist row for the duration of the replacement (FOR UPDATE).
    let authorized: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id
        FROM playlists
        WHERE id = $1
          AND user_id = $2
          AND playlist_type = 'discover'
        FOR UPDATE
        "#,
    )
    .bind(playlist_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    if authorized.is_none() {
        return Err(WorkerError::Internal(format!(
            "Refusing to update playlist {} for user {} (not found/unauthorized)",
            playlist_id, user_id
        )));
    }

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

// =============================================================================
// Cluster-Based Playlist Generation
// =============================================================================

/// Generate taste-clustered playlists for a user
///
/// This function orchestrates the cluster playlist generation process:
/// 1. Fetches listening history embeddings
/// 2. Performs k-means clustering using the clustering module
/// 3. For each cluster: saves metadata, creates playlist, finds similar tracks
///
/// Cluster playlists supplement the Discover Weekly with more targeted
/// recommendations based on distinct taste preferences.
async fn generate_cluster_playlists(
    state: &AppState,
    user_id: Uuid,
    track_count: usize,
) -> WorkerResult<()> {
    tracing::debug!(user_id = %user_id, "Starting cluster playlist generation");

    // Step 1: Fetch listening history embeddings with metadata
    let (embeddings, metadata) =
        get_listening_history_embeddings(state, user_id, CLUSTER_HISTORY_DAYS).await?;

    if embeddings.len() < MIN_HISTORY_FOR_CLUSTERING {
        tracing::info!(
            user_id = %user_id,
            embedding_count = embeddings.len(),
            min_required = MIN_HISTORY_FOR_CLUSTERING,
            "Insufficient listening history for clustering"
        );
        return Ok(());
    }

    tracing::debug!(
        user_id = %user_id,
        embedding_count = embeddings.len(),
        "Fetched embeddings for clustering"
    );

    // Step 2: Perform k-means clustering
    let clusters = cluster_user_taste_with_metadata(&embeddings, &metadata);

    if clusters.is_empty() {
        tracing::info!(
            user_id = %user_id,
            "No distinct clusters found in listening history"
        );
        return Ok(());
    }

    tracing::info!(
        user_id = %user_id,
        cluster_count = clusters.len(),
        "Generated taste clusters"
    );

    // Step 3: Save cluster metadata and generate playlists
    let cluster_db_ids = save_cluster_metadata(state, user_id, &clusters).await?;

    // Clamp track count for cluster playlists
    let playlist_track_count = track_count.min(CLUSTER_PLAYLIST_TRACK_COUNT);

    // Step 4: For each cluster, create playlist and populate with similar tracks
    for (cluster, cluster_db_id) in clusters.iter().zip(cluster_db_ids.iter()) {
        // Find or create playlist for this cluster
        let playlist_id =
            find_or_create_cluster_playlist(state, user_id, cluster, *cluster_db_id).await?;

        // Find tracks similar to the cluster using aggregated cluster data
        let similar_tracks =
            find_similar_tracks_for_cluster(state, user_id, cluster, playlist_track_count).await?;

        if similar_tracks.is_empty() {
            tracing::warn!(
                user_id = %user_id,
                cluster_name = %cluster.suggested_name,
                "No similar tracks found for cluster"
            );
            continue;
        }

        // Replace playlist tracks with new discoveries
        replace_playlist_tracks(state, user_id, playlist_id, &similar_tracks).await?;

        tracing::info!(
            user_id = %user_id,
            cluster_name = %cluster.suggested_name,
            playlist_id = %playlist_id,
            track_count = similar_tracks.len(),
            "Cluster playlist updated successfully"
        );
    }

    Ok(())
}

/// Fetch user's listening history embeddings with metadata for clustering
///
/// Returns embeddings as (track_id, embedding_vector) pairs along with
/// a metadata map for cluster naming purposes.
async fn get_listening_history_embeddings(
    state: &AppState,
    user_id: Uuid,
    days: i32,
) -> WorkerResult<(Vec<(Uuid, Vec<f32>)>, HashMap<Uuid, TrackClusterMetadata>)> {
    // Fetch unique tracks from listening history with embeddings and metadata
    // Cast the embedding to text so we can parse it as a string
    let tracks: Vec<TrackEmbeddingWithMetadata> = sqlx::query_as(
        r#"
        SELECT DISTINCT ON (te.track_id)
            te.track_id,
            te.description_embedding::text as description_embedding,
            t.ai_mood[1] as mood,
            t.genres[1] as genre,
            (t.audio_features->>'energy')::float as energy,
            (t.audio_features->>'valence')::float as valence
        FROM listening_history lh
        JOIN track_embeddings te ON te.track_id = lh.track_id
        JOIN tracks t ON t.id = lh.track_id
        WHERE lh.user_id = $1
          AND lh.played_at > NOW() - make_interval(days => $2)
          AND lh.completed = true
          AND te.description_embedding IS NOT NULL
        ORDER BY te.track_id, lh.played_at DESC
        "#,
    )
    .bind(user_id)
    .bind(days)
    .fetch_all(&state.db)
    .await?;

    // Convert to clustering format
    let mut embeddings: Vec<(Uuid, Vec<f32>)> = Vec::with_capacity(tracks.len());
    let mut metadata: HashMap<Uuid, TrackClusterMetadata> = HashMap::with_capacity(tracks.len());

    for track in tracks {
        // Parse pgvector string representation "[0.1, 0.2, ...]" to Vec<f32>
        let embedding_vec = parse_pgvector_string(&track.description_embedding)?;

        embeddings.push((track.track_id, embedding_vec));

        metadata.insert(
            track.track_id,
            TrackClusterMetadata {
                mood: track.mood,
                genre: track.genre,
                energy: track.energy.unwrap_or(0.5),
                valence: track.valence.unwrap_or(0.5),
            },
        );
    }

    Ok((embeddings, metadata))
}

/// Parse a pgvector string representation "[0.1, 0.2, ...]" into Vec<f32>
fn parse_pgvector_string(s: &str) -> WorkerResult<Vec<f32>> {
    // Remove brackets and split by comma
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| WorkerError::Internal(format!("Invalid pgvector format: {}", s)))?;

    if inner.is_empty() {
        return Ok(Vec::new());
    }

    inner
        .split(',')
        .map(|v| {
            v.trim().parse::<f32>().map_err(|e| {
                WorkerError::Internal(format!("Failed to parse vector element '{}': {}", v, e))
            })
        })
        .collect()
}

/// Format a Vec<f32> as a pgvector string representation "[0.1, 0.2, ...]"
fn format_pgvector_string(embedding: &[f32]) -> WorkerResult<String> {
    // Validate that all values are finite to prevent database errors
    if embedding.iter().any(|v| !v.is_finite()) {
        return Err(WorkerError::Internal(
            "Embedding contains non-finite values (NaN/inf)".to_string(),
        ));
    }

    let values: Vec<String> = embedding.iter().map(|v| format!("{:.6}", v)).collect();
    Ok(format!("[{}]", values.join(",")))
}

/// Save cluster metadata to the database
///
/// Upserts cluster data into user_taste_clusters table using the
/// (user_id, cluster_index) unique constraint for atomic updates.
/// Returns the database IDs of the saved/updated clusters.
///
/// Uses a transaction to ensure atomicity: either all cluster upserts
/// and the cleanup DELETE succeed together, or none of them do.
async fn save_cluster_metadata(
    state: &AppState,
    user_id: Uuid,
    clusters: &[TasteCluster],
) -> WorkerResult<Vec<Uuid>> {
    let mut tx = state.db.begin().await?;
    let mut cluster_ids = Vec::with_capacity(clusters.len());

    for cluster in clusters {
        // Convert centroid to pgvector string format "[0.1, 0.2, ...]"
        let centroid_str = format_pgvector_string(&cluster.centroid)?;

        // Upsert cluster record
        let result: ClusterUpsertResult = sqlx::query_as(
            r#"
            INSERT INTO user_taste_clusters (
                user_id,
                cluster_index,
                centroid_embedding,
                dominant_mood,
                dominant_genre,
                average_energy,
                average_valence,
                track_count,
                cluster_name
            )
            VALUES ($1, $2, $3::vector, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (user_id, cluster_index)
            DO UPDATE SET
                centroid_embedding = EXCLUDED.centroid_embedding,
                dominant_mood = EXCLUDED.dominant_mood,
                dominant_genre = EXCLUDED.dominant_genre,
                average_energy = EXCLUDED.average_energy,
                average_valence = EXCLUDED.average_valence,
                track_count = EXCLUDED.track_count,
                cluster_name = EXCLUDED.cluster_name,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(cluster.index as i16)
        .bind(&centroid_str)
        .bind(&cluster.dominant_mood)
        .bind(&cluster.dominant_genre)
        .bind(cluster.average_energy)
        .bind(cluster.average_valence)
        .bind(cluster.track_ids.len() as i32)
        .bind(&cluster.suggested_name)
        .fetch_one(&mut *tx)
        .await?;

        cluster_ids.push(result.id);

        tracing::debug!(
            user_id = %user_id,
            cluster_index = cluster.index,
            cluster_id = %result.id,
            cluster_name = %cluster.suggested_name,
            "Saved cluster metadata"
        );
    }

    // Clean up old clusters that no longer exist
    // (e.g., if user went from 3 clusters to 2)
    let max_index = clusters.len() as i16;
    sqlx::query(
        r#"
        DELETE FROM user_taste_clusters
        WHERE user_id = $1 AND cluster_index >= $2
        "#,
    )
    .bind(user_id)
    .bind(max_index)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(cluster_ids)
}

/// Find or create a playlist for a taste cluster
///
/// Uses atomic upsert to create or update a cluster playlist.
/// The playlist is linked to its source cluster via the cluster_id column.
async fn find_or_create_cluster_playlist(
    state: &AppState,
    user_id: Uuid,
    cluster: &TasteCluster,
    cluster_db_id: Uuid,
) -> WorkerResult<Uuid> {
    // Generate a description for the cluster playlist
    let description = format!(
        "Tracks matching your {} taste, updated weekly",
        cluster.suggested_name.to_lowercase()
    );

    // Atomic upsert: create or update cluster playlist
    // Use cluster_id to find existing playlist for this cluster
    let result: PlaylistUpsertResult = sqlx::query_as(
        r#"
        INSERT INTO playlists (user_id, name, description, playlist_type, is_public, cluster_id)
        VALUES ($1, $2, $3, 'discover', false, $4)
        ON CONFLICT (cluster_id) WHERE cluster_id IS NOT NULL
        DO UPDATE SET
            name = EXCLUDED.name,
            description = EXCLUDED.description,
            updated_at = NOW()
        RETURNING id, (xmax = 0) AS created
        "#,
    )
    .bind(user_id)
    .bind(&cluster.suggested_name)
    .bind(&description)
    .bind(cluster_db_id)
    .fetch_one(&state.db)
    .await?;

    if result.created {
        tracing::info!(
            user_id = %user_id,
            playlist_id = %result.id,
            cluster_name = %cluster.suggested_name,
            "Created new cluster playlist"
        );
    } else {
        tracing::debug!(
            user_id = %user_id,
            playlist_id = %result.id,
            cluster_name = %cluster.suggested_name,
            "Found existing cluster playlist"
        );
    }

    Ok(result.id)
}

/// Find tracks similar to a cluster centroid using combined similarity scoring.
///
/// Uses a multi-dimensional similarity approach for better recommendations:
/// - Semantic similarity (50%): pgvector cosine distance on description embeddings
/// - Acoustic similarity (30%): Euclidean distance on normalized audio features vs cluster averages
/// - Categorical similarity (20%): Match against cluster's dominant mood and genre
///
/// Falls back to acoustic + categorical only if embeddings aren't available.
async fn find_similar_tracks_for_cluster(
    state: &AppState,
    user_id: Uuid,
    cluster: &TasteCluster,
    limit: usize,
) -> WorkerResult<Vec<Uuid>> {
    if cluster.centroid.is_empty() {
        return Ok(Vec::new());
    }

    // Convert centroid to pgvector string format
    let centroid_str = format_pgvector_string(&cluster.centroid)?;

    // Try combined similarity first (embedding + features + tags)
    let tracks =
        find_similar_tracks_combined(state, user_id, &centroid_str, cluster, limit).await?;

    if !tracks.is_empty() {
        return Ok(tracks);
    }

    // Fallback: use audio features + tags only (for libraries without embeddings)
    tracing::debug!(
        user_id = %user_id,
        "No tracks found with combined similarity, falling back to feature-based matching"
    );
    find_similar_tracks_by_features(state, user_id, limit).await
}

/// Find tracks using combined similarity (embedding + features + tags)
///
/// This approach uses the cluster's aggregated data (average_energy, average_valence,
/// dominant_mood, dominant_genre) directly for acoustic and categorical similarity,
/// rather than finding a proxy track. This provides more representative similarity
/// matching that accounts for the entire cluster's characteristics.
///
/// Aligns with prefetch.rs and the SimilarityService for consistency.
async fn find_similar_tracks_combined(
    state: &AppState,
    user_id: Uuid,
    centroid_str: &str,
    cluster: &TasteCluster,
    limit: usize,
) -> WorkerResult<Vec<Uuid>> {
    // Combined similarity query using all three dimensions:
    // - Semantic: pgvector cosine distance on description embeddings vs centroid
    // - Acoustic: Euclidean distance on normalized audio features vs cluster averages
    // - Categorical: Match against cluster's dominant mood and genre
    let tracks: Vec<TrackIdRecord> = sqlx::query_as(
        r#"
        WITH recently_played AS (
            SELECT DISTINCT track_id
            FROM listening_history
            WHERE user_id = $1
              AND played_at > NOW() - make_interval(days => $2)
        ),
        embedding_scores AS (
            SELECT
                t.id,
                -- Clamp cosine similarity to [0, 1] range, default to 0 if no embedding
                CASE WHEN te.description_embedding IS NOT NULL
                    THEN GREATEST(0.0, LEAST(1.0, 1.0 - (te.description_embedding <=> $3::vector)))
                    ELSE 0.0
                END as score
            FROM tracks t
            LEFT JOIN track_embeddings te ON t.id = te.track_id
            WHERE NOT EXISTS (SELECT 1 FROM recently_played rp WHERE rp.track_id = t.id)
        ),
        feature_scores AS (
            SELECT
                t.id,
                -- Euclidean distance on normalized features vs cluster averages, clamped to [0, 1]
                -- Uses cluster's average_energy and average_valence directly
                GREATEST(0.0, LEAST(1.0, 1.0 - (
                    SQRT(
                        COALESCE(POWER((t.audio_features->>'energy')::float - $9, 2), 0) +
                        COALESCE(POWER((t.audio_features->>'valence')::float - $10, 2), 0)
                    ) / 1.5
                ))) as score
            FROM tracks t
            WHERE t.audio_features->>'energy' IS NOT NULL
              AND NOT EXISTS (SELECT 1 FROM recently_played rp WHERE rp.track_id = t.id)
        ),
        tag_scores AS (
            SELECT
                t.id,
                -- Score based on matching cluster's dominant mood and genre
                -- Mood weighted 2x (more specific than genre), normalized to [0, 1]
                (
                    CASE WHEN $11 IS NOT NULL AND $11 = ANY(t.genres) THEN 1.0 ELSE 0.0 END +
                    CASE WHEN $12 IS NOT NULL AND $12 = ANY(t.ai_mood) THEN 2.0 ELSE 0.0 END
                ) / 3.0 as score
            FROM tracks t
            WHERE NOT EXISTS (SELECT 1 FROM recently_played rp WHERE rp.track_id = t.id)
        ),
        combined_scores AS (
            -- FULL OUTER JOIN allows tracks without embeddings to still be recommended
            -- based on audio features and/or tags alone
            SELECT
                COALESCE(e.id, f.id, g.id) as id,
                (
                    COALESCE(e.score, 0) * $4 +
                    COALESCE(f.score, 0) * $5 +
                    COALESCE(g.score, 0) * $8
                ) as combined_score
            FROM embedding_scores e
            FULL OUTER JOIN feature_scores f ON e.id = f.id
            FULL OUTER JOIN tag_scores g ON COALESCE(e.id, f.id) = g.id
        )
        SELECT cs.id
        FROM combined_scores cs
        JOIN tracks t ON t.id = cs.id
        ORDER BY cs.combined_score DESC
        LIMIT $6
        "#,
    )
    .bind(user_id)
    .bind(CLUSTER_RECENTLY_PLAYED_DAYS)
    .bind(centroid_str)
    .bind(WEIGHT_SEMANTIC)
    .bind(WEIGHT_ACOUSTIC)
    .bind(limit as i64)
    .bind(LOUDNESS_OFFSET)
    .bind(WEIGHT_CATEGORICAL)
    .bind(cluster.average_energy as f64)
    .bind(cluster.average_valence as f64)
    .bind(&cluster.dominant_genre)
    .bind(&cluster.dominant_mood)
    .fetch_all(&state.db)
    .await?;

    Ok(tracks.into_iter().map(|t| t.id).collect())
}

/// Fallback: Find tracks using audio features + tags only (no embeddings).
///
/// Used when the library doesn't have embeddings or combined query returns empty.
/// Finds tracks with similar audio characteristics and categorical metadata.
async fn find_similar_tracks_by_features(
    state: &AppState,
    user_id: Uuid,
    limit: usize,
) -> WorkerResult<Vec<Uuid>> {
    // Fallback query using only audio features and tags
    // Uses user's listening history to derive average features as seed
    let tracks: Vec<TrackIdRecord> = sqlx::query_as(
        r#"
        WITH recently_played AS (
            SELECT DISTINCT track_id
            FROM listening_history
            WHERE user_id = $1
              AND played_at > NOW() - make_interval(days => $2)
        ),
        -- Derive average audio features from user's recent history
        user_profile AS (
            SELECT
                AVG((t.audio_features->>'energy')::float) as energy,
                AVG((t.audio_features->>'loudness')::float) as loudness,
                AVG((t.audio_features->>'valence')::float) as valence,
                AVG((t.audio_features->>'danceability')::float) as danceability,
                AVG((t.audio_features->>'bpm')::float) as bpm,
                -- Collect all genres/moods/tags from history for categorical matching
                array_agg(DISTINCT unnest_genre) FILTER (WHERE unnest_genre IS NOT NULL) as genres,
                array_agg(DISTINCT unnest_mood) FILTER (WHERE unnest_mood IS NOT NULL) as ai_mood,
                array_agg(DISTINCT unnest_tag) FILTER (WHERE unnest_tag IS NOT NULL) as ai_tags
            FROM listening_history lh
            JOIN tracks t ON t.id = lh.track_id
            LEFT JOIN LATERAL unnest(t.genres) as unnest_genre ON true
            LEFT JOIN LATERAL unnest(t.ai_mood) as unnest_mood ON true
            LEFT JOIN LATERAL unnest(t.ai_tags) as unnest_tag ON true
            WHERE lh.user_id = $1
              AND lh.played_at > NOW() - make_interval(days => 30)
              AND lh.completed = true
        ),
        feature_scores AS (
            SELECT
                t.id,
                -- Euclidean distance on normalized features
                GREATEST(0.0, LEAST(1.0, 1.0 - (
                    SQRT(
                        COALESCE(POWER((t.audio_features->>'energy')::float - src.energy, 2), 0) +
                        COALESCE(POWER(((t.audio_features->>'loudness')::float + $4) / $4 - (src.loudness + $4) / $4, 2), 0) +
                        COALESCE(POWER((t.audio_features->>'valence')::float - src.valence, 2), 0) +
                        COALESCE(POWER((t.audio_features->>'danceability')::float - src.danceability, 2), 0) +
                        COALESCE(POWER(((t.audio_features->>'bpm')::float - src.bpm) / $5, 2), 0)
                    ) / 2.0
                ))) as feature_score,
                -- Weighted Jaccard similarity
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
            CROSS JOIN user_profile src
            WHERE t.audio_features->>'energy' IS NOT NULL
              AND NOT EXISTS (SELECT 1 FROM recently_played rp WHERE rp.track_id = t.id)
        )
        SELECT fs.id
        FROM feature_scores fs
        ORDER BY (fs.feature_score * $6 + fs.tag_score * $7) DESC
        LIMIT $3
        "#,
    )
    .bind(user_id)
    .bind(CLUSTER_RECENTLY_PLAYED_DAYS)
    .bind(limit as i64)
    .bind(LOUDNESS_OFFSET)
    .bind(BPM_NORMALIZATION_FACTOR)
    .bind(WEIGHT_FALLBACK_FEATURE)
    .bind(WEIGHT_FALLBACK_TAGS)
    .fetch_all(&state.db)
    .await?;

    Ok(tracks.into_iter().map(|t| t.id).collect())
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

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_cluster_playlist_configuration_constants() {
        // Validate cluster playlist constants are within expected ranges.
        // These assertions serve as documentation and catch accidental changes.

        // Minimum history for clustering should be at least 10 (enough for 2 clusters of 5)
        assert!(
            MIN_HISTORY_FOR_CLUSTERING >= 10,
            "MIN_HISTORY_FOR_CLUSTERING should be at least 10"
        );

        // Cluster playlist track count should be reasonable
        assert!(
            CLUSTER_PLAYLIST_TRACK_COUNT >= 10 && CLUSTER_PLAYLIST_TRACK_COUNT <= 50,
            "CLUSTER_PLAYLIST_TRACK_COUNT should be between 10 and 50"
        );

        // Cluster history days should cover at least 2 weeks
        assert!(
            CLUSTER_HISTORY_DAYS >= 14,
            "CLUSTER_HISTORY_DAYS should be at least 14"
        );

        // Recently played filter should be less than cluster history days
        assert!(
            CLUSTER_RECENTLY_PLAYED_DAYS < CLUSTER_HISTORY_DAYS,
            "CLUSTER_RECENTLY_PLAYED_DAYS should be less than CLUSTER_HISTORY_DAYS"
        );

        // Cluster recently played should be at least 1 day
        assert!(
            CLUSTER_RECENTLY_PLAYED_DAYS >= 1,
            "CLUSTER_RECENTLY_PLAYED_DAYS should be at least 1"
        );
    }

    #[test]
    fn test_cluster_constants_consistency() {
        // Cluster history days should match or exceed seed history days
        // since clustering needs at least as much data as seed-based approach
        const {
            assert!(
                CLUSTER_HISTORY_DAYS >= SEED_HISTORY_DAYS,
                "CLUSTER_HISTORY_DAYS should be >= SEED_HISTORY_DAYS"
            )
        };

        // Cluster playlist track count should be clamped by MAX_TRACK_COUNT
        const {
            assert!(
                CLUSTER_PLAYLIST_TRACK_COUNT <= MAX_TRACK_COUNT,
                "CLUSTER_PLAYLIST_TRACK_COUNT should not exceed MAX_TRACK_COUNT"
            )
        };
    }

    #[test]
    fn test_similarity_weights_sum_to_one() {
        // Combined weights (semantic + acoustic + categorical) should sum to 1.0
        let total = WEIGHT_SEMANTIC + WEIGHT_ACOUSTIC + WEIGHT_CATEGORICAL;
        assert!(
            (total - 1.0).abs() < f64::EPSILON,
            "Combined weights should sum to 1.0, got {}",
            total
        );
    }

    #[test]
    fn test_fallback_weights_sum_to_one() {
        // Fallback weights (feature + tags) should sum to 1.0
        let total = WEIGHT_FALLBACK_FEATURE + WEIGHT_FALLBACK_TAGS;
        assert!(
            (total - 1.0).abs() < f64::EPSILON,
            "Fallback weights should sum to 1.0, got {}",
            total
        );
    }

    #[test]
    fn test_normalization_constants() {
        // BPM normalization: 200 BPM difference should normalize to ~1.0
        assert!(
            (BPM_NORMALIZATION_FACTOR - 200.0).abs() < f64::EPSILON,
            "BPM normalization factor should be 200.0"
        );

        // Loudness normalization: -60 dB to 0 dB range
        assert!(
            (LOUDNESS_OFFSET - 60.0).abs() < f64::EPSILON,
            "Loudness offset should be 60.0"
        );
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_similarity_weight_ranges() {
        // Individual weights should be positive and less than 1
        assert!(
            WEIGHT_SEMANTIC > 0.0 && WEIGHT_SEMANTIC < 1.0,
            "Semantic weight should be between 0 and 1"
        );
        assert!(
            WEIGHT_ACOUSTIC > 0.0 && WEIGHT_ACOUSTIC < 1.0,
            "Acoustic weight should be between 0 and 1"
        );
        assert!(
            WEIGHT_CATEGORICAL > 0.0 && WEIGHT_CATEGORICAL < 1.0,
            "Categorical weight should be between 0 and 1"
        );

        // Semantic should have the highest weight (most discriminative)
        assert!(
            WEIGHT_SEMANTIC >= WEIGHT_ACOUSTIC && WEIGHT_SEMANTIC >= WEIGHT_CATEGORICAL,
            "Semantic weight should be >= other weights"
        );
    }

    #[test]
    fn test_parse_pgvector_string_valid() {
        let result = parse_pgvector_string("[0.1,0.2,0.3]").unwrap();
        assert_eq!(result.len(), 3);
        assert!((result[0] - 0.1).abs() < f32::EPSILON);
        assert!((result[1] - 0.2).abs() < f32::EPSILON);
        assert!((result[2] - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_pgvector_string_with_spaces() {
        let result = parse_pgvector_string("[ 0.1 , 0.2 , 0.3 ]").unwrap();
        assert_eq!(result.len(), 3);
        assert!((result[0] - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_pgvector_string_empty() {
        let result = parse_pgvector_string("[]").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_pgvector_string_invalid_format() {
        let result = parse_pgvector_string("not a vector");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pgvector_string_invalid_number() {
        let result = parse_pgvector_string("[0.1,abc,0.3]");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_pgvector_string_basic() {
        let embedding = vec![0.1, 0.2, 0.3];
        let result = format_pgvector_string(&embedding).unwrap();
        assert!(result.starts_with('['));
        assert!(result.ends_with(']'));
        assert!(result.contains("0.100000"));
    }

    #[test]
    fn test_format_pgvector_string_empty() {
        let embedding: Vec<f32> = vec![];
        let result = format_pgvector_string(&embedding).unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_format_pgvector_string_rejects_nan() {
        let embedding = vec![0.1, f32::NAN, 0.3];
        let result = format_pgvector_string(&embedding);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_pgvector_string_rejects_inf() {
        let embedding = vec![0.1, f32::INFINITY, 0.3];
        let result = format_pgvector_string(&embedding);
        assert!(result.is_err());
    }

    #[test]
    fn test_pgvector_roundtrip() {
        // Test that format and parse are inverses
        let original = vec![0.123456, -0.654321, 0.0, 1.0];
        let formatted = format_pgvector_string(&original).unwrap();
        let parsed = parse_pgvector_string(&formatted).unwrap();

        assert_eq!(original.len(), parsed.len());
        for (orig, parsed) in original.iter().zip(parsed.iter()) {
            // Allow for floating point precision loss from formatting
            assert!((orig - parsed).abs() < 1e-5);
        }
    }
}
