//! Track similarity service
//!
//! Provides track similarity recommendations using:
//! - Vector embeddings (semantic similarity via pgvector)
//! - Audio features (acoustic similarity)
//! - Genre and mood matching (categorical similarity)
//!
//! This service is used by the semantic search GraphQL API.

use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, info_span, instrument, warn, Instrument};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// Query timeout in seconds for similarity queries
const QUERY_TIMEOUT_SECONDS: u64 = 5;

/// Maximum number of similar tracks that can be requested
const MAX_SIMILARITY_RESULTS: i32 = 100;

/// Default similarity weights for combined scoring (kept for backward compatibility)
const DEFAULT_WEIGHT_SEMANTIC: f64 = 0.5;
const DEFAULT_WEIGHT_ACOUSTIC: f64 = 0.3;
const DEFAULT_WEIGHT_CATEGORICAL: f64 = 0.2;

/// Epsilon tolerance for weight validation (floating point comparison)
const WEIGHT_EPSILON: f64 = 0.001;

// =============================================================================
// Similarity Configuration
// =============================================================================

/// Configuration for similarity scoring weights
///
/// Weights control how much each similarity dimension contributes to the combined score.
/// By default: semantic (50%), acoustic (30%), categorical (20%).
///
/// Configure via environment variables:
/// - `SIMILARITY_WEIGHT_SEMANTIC` (default: 0.5)
/// - `SIMILARITY_WEIGHT_ACOUSTIC` (default: 0.3)
/// - `SIMILARITY_WEIGHT_CATEGORICAL` (default: 0.2)
#[derive(Debug, Clone)]
pub struct SimilarityConfig {
    /// Weight for semantic (embedding) similarity (0.0 - 1.0)
    pub weight_semantic: f64,
    /// Weight for acoustic (audio feature) similarity (0.0 - 1.0)
    pub weight_acoustic: f64,
    /// Weight for categorical (genre/mood/tags) similarity (0.0 - 1.0)
    pub weight_categorical: f64,
}

impl Default for SimilarityConfig {
    fn default() -> Self {
        Self {
            weight_semantic: DEFAULT_WEIGHT_SEMANTIC,
            weight_acoustic: DEFAULT_WEIGHT_ACOUSTIC,
            weight_categorical: DEFAULT_WEIGHT_CATEGORICAL,
        }
    }
}

impl SimilarityConfig {
    /// Create a new SimilarityConfig with custom weights
    ///
    /// # Errors
    /// Returns an error if the weights don't sum to 1.0 (within epsilon tolerance)
    pub fn new(
        weight_semantic: f64,
        weight_acoustic: f64,
        weight_categorical: f64,
    ) -> Result<Self, SimilarityConfigError> {
        let config = Self {
            weight_semantic,
            weight_acoustic,
            weight_categorical,
        };
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - `SIMILARITY_WEIGHT_SEMANTIC` (default: 0.5)
    /// - `SIMILARITY_WEIGHT_ACOUSTIC` (default: 0.3)
    /// - `SIMILARITY_WEIGHT_CATEGORICAL` (default: 0.2)
    ///
    /// If any environment variable is set, all three should be configured.
    /// The weights must sum to 1.0 (within epsilon tolerance).
    pub fn from_env() -> Result<Self, SimilarityConfigError> {
        let weight_semantic =
            Self::parse_env_weight("SIMILARITY_WEIGHT_SEMANTIC", DEFAULT_WEIGHT_SEMANTIC)?;
        let weight_acoustic =
            Self::parse_env_weight("SIMILARITY_WEIGHT_ACOUSTIC", DEFAULT_WEIGHT_ACOUSTIC)?;
        let weight_categorical =
            Self::parse_env_weight("SIMILARITY_WEIGHT_CATEGORICAL", DEFAULT_WEIGHT_CATEGORICAL)?;

        let config = Self {
            weight_semantic,
            weight_acoustic,
            weight_categorical,
        };

        config.validate()?;

        // Log if custom weights are being used
        if (config.weight_semantic - DEFAULT_WEIGHT_SEMANTIC).abs() > f64::EPSILON
            || (config.weight_acoustic - DEFAULT_WEIGHT_ACOUSTIC).abs() > f64::EPSILON
            || (config.weight_categorical - DEFAULT_WEIGHT_CATEGORICAL).abs() > f64::EPSILON
        {
            info!(
                weight_semantic = config.weight_semantic,
                weight_acoustic = config.weight_acoustic,
                weight_categorical = config.weight_categorical,
                "Using custom similarity weights from environment"
            );
        }

        Ok(config)
    }

    /// Parse a weight value from an environment variable
    fn parse_env_weight(var_name: &str, default: f64) -> Result<f64, SimilarityConfigError> {
        match env::var(var_name) {
            Ok(value) => {
                let weight: f64 =
                    value
                        .parse()
                        .map_err(|_| SimilarityConfigError::InvalidWeight {
                            var_name: var_name.to_string(),
                            value: value.clone(),
                        })?;

                if !(0.0..=1.0).contains(&weight) {
                    return Err(SimilarityConfigError::WeightOutOfRange {
                        var_name: var_name.to_string(),
                        value: weight,
                    });
                }

                Ok(weight)
            }
            Err(_) => Ok(default),
        }
    }

    /// Validate that weights sum to 1.0 (within epsilon tolerance)
    pub fn validate(&self) -> Result<(), SimilarityConfigError> {
        let total = self.weight_semantic + self.weight_acoustic + self.weight_categorical;
        if (total - 1.0).abs() > WEIGHT_EPSILON {
            return Err(SimilarityConfigError::WeightsSumInvalid { total });
        }
        Ok(())
    }
}

/// Errors that can occur when loading similarity configuration
#[derive(Debug, Clone, thiserror::Error)]
pub enum SimilarityConfigError {
    /// Weight value could not be parsed as a float
    #[error("Invalid weight value for {var_name}: '{value}' is not a valid number")]
    InvalidWeight { var_name: String, value: String },

    /// Weight value is out of the valid range [0.0, 1.0]
    #[error("Weight {var_name} value {value} is out of range (must be 0.0 - 1.0)")]
    WeightOutOfRange { var_name: String, value: f64 },

    /// Weights don't sum to 1.0
    #[error("Similarity weights must sum to 1.0 (got {total:.4})")]
    WeightsSumInvalid { total: f64 },
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Validate and clamp the limit parameter to safe bounds
fn validate_limit(limit: i32) -> i32 {
    limit.clamp(1, MAX_SIMILARITY_RESULTS)
}

/// Check if a database error is a query timeout and convert appropriately
fn handle_query_error(error: sqlx::Error, query_name: &str) -> ApiError {
    // Check for PostgreSQL statement timeout error (error code 57014)
    if let sqlx::Error::Database(ref db_error) = error {
        if db_error.code().map_or(false, |code| code == "57014") {
            warn!(
                query = query_name,
                timeout_seconds = QUERY_TIMEOUT_SECONDS,
                "Query timeout exceeded"
            );
            return ApiError::QueryTimeout {
                timeout_seconds: QUERY_TIMEOUT_SECONDS,
            };
        }
    }
    ApiError::from(error)
}

/// Track similarity service
#[derive(Clone)]
pub struct SimilarityService {
    db: PgPool,
    config: Arc<SimilarityConfig>,
}

/// A track with its similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarTrack {
    pub track_id: Uuid,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub score: f64,
    pub similarity_type: SimilarityType,
}

/// Type of similarity used for the match
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SimilarityType {
    /// Based on AI-generated description embeddings
    Semantic,
    /// Based on audio features (loudness, energy, etc.)
    Acoustic,
    /// Based on genre and mood tags
    Categorical,
    /// Combined similarity using multiple factors
    Combined,
}

/// Audio features extracted from JSON.
/// Fields are populated via serde deserialization and accessed for None checks.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // Fields accessed via serde deserialization, not direct field access
struct AudioFeatures {
    bpm: Option<f64>,
    key: Option<String>,
    mode: Option<String>,
    loudness: Option<f64>,
    energy: Option<f64>,
    danceability: Option<f64>,
    valence: Option<f64>,
    acousticness: Option<f64>,
    instrumentalness: Option<f64>,
    speechiness: Option<f64>,
}

impl SimilarityService {
    /// Create a new similarity service with default configuration
    pub fn new(db: PgPool) -> Self {
        Self::with_config(db, SimilarityConfig::default())
    }

    /// Create a new similarity service with custom configuration
    pub fn with_config(db: PgPool, config: SimilarityConfig) -> Self {
        Self {
            db,
            config: Arc::new(config),
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &SimilarityConfig {
        &self.config
    }

    /// Find similar tracks using embedding similarity (pgvector)
    ///
    /// Uses cosine distance on description embeddings for semantic similarity.
    /// Has a 5-second query timeout for protection against slow queries.
    ///
    /// # Errors
    /// - `ApiError::NotFound` - If the track doesn't exist or has no embeddings
    /// - `ApiError::Database` - If the database query fails
    /// - `ApiError::QueryTimeout` - If the query exceeds the timeout
    #[instrument(skip(self), fields(similarity_type = "semantic"))]
    pub async fn find_similar_by_embedding(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let limit = validate_limit(limit);

        // First check if the source track has embeddings (simple query, no timeout needed)
        let has_embedding: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM track_embeddings WHERE track_id = $1 AND description_embedding IS NOT NULL)",
        )
        .bind(track_id)
        .fetch_one(&self.db)
        .await?;

        if !has_embedding.0 {
            return Err(ApiError::not_found(
                "track embedding",
                format!("{} (run embedding generation first)", track_id),
            ));
        }

        // Use a transaction to set statement timeout for this query
        let mut tx = self.db.begin().await?;

        // Set statement timeout for this transaction (in seconds)
        sqlx::query(&format!(
            "SET LOCAL statement_timeout = '{}s'",
            QUERY_TIMEOUT_SECONDS
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| handle_query_error(e, "set_timeout_semantic"))?;

        // Find similar tracks using pgvector cosine distance
        // Lower distance = more similar, so we order ascending and convert to similarity score
        let similar: Vec<SimilarTrackRow> = sqlx::query_as(
            r#"
            SELECT
                t.id as track_id,
                t.title,
                a.name as artist_name,
                al.title as album_title,
                1.0 - (te.description_embedding <=> source.description_embedding) as score
            FROM track_embeddings te
            JOIN track_embeddings source ON source.track_id = $1
            JOIN tracks t ON t.id = te.track_id
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            WHERE te.track_id != $1
              AND te.description_embedding IS NOT NULL
            ORDER BY te.description_embedding <=> source.description_embedding
            LIMIT $2
            "#,
        )
        .bind(track_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| handle_query_error(e, "find_similar_by_embedding"))?;

        // Commit the transaction (timeout is automatically reset)
        tx.commit().await?;

        Ok(similar
            .into_iter()
            .map(|r| SimilarTrack {
                track_id: r.track_id,
                title: r.title,
                artist_name: r.artist_name,
                album_title: r.album_title,
                // Clamp score to [0.0, 1.0] - cosine distance can produce values outside this range
                score: r.score.unwrap_or(0.0).clamp(0.0, 1.0),
                similarity_type: SimilarityType::Semantic,
            })
            .collect())
    }

    /// Find similar tracks based on audio features
    ///
    /// Uses the pre-computed audio_features_vector with pgvector's L2 distance operator (<->)
    /// for O(log n) performance via HNSW index. Falls back to JSONB-based calculation
    /// if the vector is not available.
    ///
    /// Vector dimensions (all normalized to 0-1 range):
    /// - [0] energy (already 0-1)
    /// - [1] loudness_norm ((loudness + 60) / 60, maps -60..0 dB to 0..1)
    /// - [2] valence (already 0-1)
    /// - [3] danceability (already 0-1)
    /// - [4] bpm_norm (bpm / 200, normalizes typical 60-180 BPM range)
    ///
    /// Has a 5-second query timeout for protection against slow queries.
    ///
    /// # Errors
    /// - `ApiError::NotFound` - If the track doesn't exist or has no audio features
    /// - `ApiError::Database` - If the database query fails
    /// - `ApiError::QueryTimeout` - If the query exceeds the timeout
    #[instrument(skip(self), fields(similarity_type = "acoustic"))]
    pub async fn find_similar_by_features(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let limit = validate_limit(limit);

        // First, check if we have a pre-computed audio_features_vector (fast path)
        let has_vector: Option<(bool,)> = sqlx::query_as(
            "SELECT audio_features_vector IS NOT NULL FROM track_embeddings WHERE track_id = $1",
        )
        .bind(track_id)
        .fetch_optional(&self.db)
        .await?;

        let use_vector_path = has_vector.map_or(false, |(has,)| has);

        if use_vector_path {
            info!(
                track_id = %track_id,
                "Using vector-based acoustic similarity (HNSW indexed, O(log n))"
            );
            return self.find_similar_by_features_vector(track_id, limit).await;
        }

        // Fallback: Check JSONB audio features
        info!(
            track_id = %track_id,
            "Using JSONB fallback for acoustic similarity (full table scan, O(n))"
        );

        // Load source track's audio features from JSONB
        let source_features: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT audio_features FROM tracks WHERE id = $1")
                .bind(track_id)
                .fetch_optional(&self.db)
                .await?;

        let source_features =
            source_features.ok_or_else(|| ApiError::not_found("track", track_id.to_string()))?;

        let source: AudioFeatures = match serde_json::from_value(source_features.0.clone()) {
            Ok(features) => features,
            Err(e) => {
                warn!(
                    track_id = %track_id,
                    error = %e,
                    "Failed to parse audio features JSON, using defaults"
                );
                AudioFeatures::default()
            }
        };

        // Check if source has useful features
        if source.energy.is_none() && source.loudness.is_none() {
            return Err(ApiError::not_found(
                "track audio features",
                format!("{} (run feature extraction first)", track_id),
            ));
        }

        self.find_similar_by_features_jsonb(track_id, limit).await
    }

    /// Find similar tracks using pre-computed audio_features_vector with HNSW index
    ///
    /// This is the fast path using pgvector's L2 distance operator (<->).
    /// The HNSW index provides O(log n) performance.
    async fn find_similar_by_features_vector(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        // Use a transaction to set statement timeout for this query
        let mut tx = self.db.begin().await?;

        // Set statement timeout for this transaction
        sqlx::query(&format!(
            "SET LOCAL statement_timeout = '{}s'",
            QUERY_TIMEOUT_SECONDS
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| handle_query_error(e, "set_timeout_acoustic_vector"))?;

        // Find similar tracks using pgvector L2 distance on audio_features_vector
        // Lower distance = more similar, so we order ascending and convert to similarity score
        let similar: Vec<SimilarTrackRow> = sqlx::query_as(
            r#"
            SELECT
                t.id as track_id,
                t.title,
                a.name as artist_name,
                al.title as album_title,
                -- Convert L2 distance to similarity score (0-1 range)
                -- Max theoretical L2 distance for 5D unit vectors is sqrt(5) ≈ 2.236
                -- Using 2.0 as normalization factor for practical range
                GREATEST(0.0, 1.0 - (te.audio_features_vector <-> source.audio_features_vector) / 2.0) as score
            FROM track_embeddings te
            JOIN track_embeddings source ON source.track_id = $1
            JOIN tracks t ON t.id = te.track_id
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            WHERE te.track_id != $1
              AND te.audio_features_vector IS NOT NULL
            ORDER BY te.audio_features_vector <-> source.audio_features_vector
            LIMIT $2
            "#,
        )
        .bind(track_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| handle_query_error(e, "find_similar_by_features_vector"))?;

        // Commit the transaction (timeout is automatically reset)
        tx.commit().await?;

        Ok(similar
            .into_iter()
            .map(|r| SimilarTrack {
                track_id: r.track_id,
                title: r.title,
                artist_name: r.artist_name,
                album_title: r.album_title,
                score: r.score.unwrap_or(0.0).clamp(0.0, 1.0),
                similarity_type: SimilarityType::Acoustic,
            })
            .collect())
    }

    /// Find similar tracks using JSONB-based feature distance (fallback path)
    ///
    /// This is the slow path using a full table scan with manual distance calculation.
    /// Used when audio_features_vector is not available for the source track.
    async fn find_similar_by_features_jsonb(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        // Use a transaction to set statement timeout for this query
        let mut tx = self.db.begin().await?;

        // Set statement timeout for this transaction
        sqlx::query(&format!(
            "SET LOCAL statement_timeout = '{}s'",
            QUERY_TIMEOUT_SECONDS
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| handle_query_error(e, "set_timeout_acoustic_jsonb"))?;

        // Find similar tracks using SQL-based feature distance
        // This calculates Euclidean distance on available features
        let similar: Vec<SimilarTrackRow> = sqlx::query_as(
            r#"
            WITH source_track AS (
                SELECT
                    (audio_features->>'energy')::float as energy,
                    (audio_features->>'loudness')::float as loudness,
                    (audio_features->>'valence')::float as valence,
                    (audio_features->>'danceability')::float as danceability,
                    (audio_features->>'bpm')::float as bpm
                FROM tracks
                WHERE id = $1
            ),
            track_distances AS (
                SELECT
                    t.id as track_id,
                    t.title,
                    a.name as artist_name,
                    al.title as album_title,
                    -- Calculate normalized Euclidean distance
                    SQRT(
                        COALESCE(POWER((t.audio_features->>'energy')::float - src.energy, 2), 0) +
                        COALESCE(POWER(((t.audio_features->>'loudness')::float + 60) / 60 - (src.loudness + 60) / 60, 2), 0) +
                        COALESCE(POWER((t.audio_features->>'valence')::float - src.valence, 2), 0) +
                        COALESCE(POWER((t.audio_features->>'danceability')::float - src.danceability, 2), 0) +
                        COALESCE(POWER(((t.audio_features->>'bpm')::float - src.bpm) / 200, 2), 0)
                    ) as distance
                FROM tracks t
                CROSS JOIN source_track src
                LEFT JOIN artists a ON t.artist_id = a.id
                LEFT JOIN albums al ON t.album_id = al.id
                WHERE t.id != $1
                  AND t.audio_features->>'energy' IS NOT NULL
            )
            SELECT
                track_id,
                title,
                artist_name,
                album_title,
                -- Convert distance to similarity score (0-1 range)
                GREATEST(0, 1.0 - (distance / 2.0)) as score
            FROM track_distances
            ORDER BY distance ASC
            LIMIT $2
            "#,
        )
        .bind(track_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| handle_query_error(e, "find_similar_by_features_jsonb"))?;

        // Commit the transaction (timeout is automatically reset)
        tx.commit().await?;

        Ok(similar
            .into_iter()
            .map(|r| SimilarTrack {
                track_id: r.track_id,
                title: r.title,
                artist_name: r.artist_name,
                album_title: r.album_title,
                score: r.score.unwrap_or(0.0),
                similarity_type: SimilarityType::Acoustic,
            })
            .collect())
    }

    /// Find similar tracks based on genre and mood tags
    ///
    /// Uses weighted Jaccard similarity with mood weighted 2x (mood is more specific).
    /// Has a 5-second query timeout for protection against slow queries.
    ///
    /// # Errors
    /// - `ApiError::Database` - If the database query fails
    /// - `ApiError::QueryTimeout` - If the query exceeds the timeout
    #[instrument(skip(self), fields(similarity_type = "categorical"))]
    pub async fn find_similar_by_tags(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let limit = validate_limit(limit);

        // Use a transaction to set statement timeout for this query
        let mut tx = self.db.begin().await?;

        // Set statement timeout for this transaction (in seconds)
        sqlx::query(&format!(
            "SET LOCAL statement_timeout = '{}s'",
            QUERY_TIMEOUT_SECONDS
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| handle_query_error(e, "set_timeout_categorical"))?;

        // Find tracks with overlapping genres or moods
        let similar: Vec<SimilarTrackRow> = sqlx::query_as(
            r#"
            WITH source_track AS (
                SELECT genres, ai_mood, ai_tags
                FROM tracks
                WHERE id = $1
            )
            SELECT
                t.id as track_id,
                t.title,
                a.name as artist_name,
                al.title as album_title,
                -- Score based on tag overlap (weighted Jaccard similarity)
                -- Mood is weighted 2x in both numerator and denominator
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
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            WHERE t.id != $1
              AND (
                  t.genres && src.genres OR
                  t.ai_mood && src.ai_mood OR
                  t.ai_tags && src.ai_tags
              )
            ORDER BY score DESC
            LIMIT $2
            "#,
        )
        .bind(track_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| handle_query_error(e, "find_similar_by_tags"))?;

        // Commit the transaction (timeout is automatically reset)
        tx.commit().await?;

        Ok(similar
            .into_iter()
            .map(|r| SimilarTrack {
                track_id: r.track_id,
                title: r.title,
                artist_name: r.artist_name,
                album_title: r.album_title,
                score: r.score.unwrap_or(0.0),
                similarity_type: SimilarityType::Categorical,
            })
            .collect())
    }

    /// Find similar tracks using combined similarity (embedding + features + tags)
    ///
    /// Combines semantic (50%), acoustic (30%), and categorical (20%) similarity.
    /// A track appearing in only one dimension receives a proportionally lower score.
    ///
    /// Queries are executed in parallel using tokio::join! for improved latency (~50% reduction).
    ///
    /// # Errors
    /// - Returns an empty result if all similarity methods fail
    #[instrument(skip(self), fields(similarity_type = "combined"))]
    pub async fn find_similar_combined(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let limit = validate_limit(limit);

        // Get results from all methods (get more than we need for merging)
        let fetch_limit = limit * 3;

        // Execute all three similarity queries in parallel for improved latency
        // Using tokio::join! instead of try_join! to continue with other methods if one fails
        let (semantic_result, acoustic_result, categorical_result) = async {
            let semantic_fut = self
                .find_similar_by_embedding(track_id, fetch_limit)
                .instrument(info_span!("semantic_query"));
            let acoustic_fut = self
                .find_similar_by_features(track_id, fetch_limit)
                .instrument(info_span!("acoustic_query"));
            let categorical_fut = self
                .find_similar_by_tags(track_id, fetch_limit)
                .instrument(info_span!("categorical_query"));

            tokio::join!(semantic_fut, acoustic_fut, categorical_fut)
        }
        .instrument(info_span!("parallel_similarity_queries"))
        .await;

        // Process results, logging errors instead of silently discarding
        let semantic = match semantic_result {
            Ok(tracks) => Some(tracks),
            Err(e) => {
                warn!(
                    track_id = %track_id,
                    error = %e,
                    "Semantic similarity lookup failed, continuing with other methods"
                );
                None
            }
        };

        let acoustic = match acoustic_result {
            Ok(tracks) => Some(tracks),
            Err(e) => {
                warn!(
                    track_id = %track_id,
                    error = %e,
                    "Acoustic similarity lookup failed, continuing with other methods"
                );
                None
            }
        };

        let categorical = match categorical_result {
            Ok(tracks) => Some(tracks),
            Err(e) => {
                warn!(
                    track_id = %track_id,
                    error = %e,
                    "Categorical similarity lookup failed, continuing with other methods"
                );
                None
            }
        };

        // Merge and weight results
        let mut combined: HashMap<Uuid, (SimilarTrack, f64)> = HashMap::new();

        // Helper to merge tracks into combined map
        let merge_tracks = |map: &mut HashMap<Uuid, (SimilarTrack, f64)>,
                            tracks: Vec<SimilarTrack>,
                            weight: f64| {
            for track in tracks {
                let entry = map.entry(track.track_id).or_insert_with(|| {
                    (
                        SimilarTrack {
                            track_id: track.track_id,
                            title: track.title.clone(),
                            artist_name: track.artist_name.clone(),
                            album_title: track.album_title.clone(),
                            score: 0.0,
                            similarity_type: SimilarityType::Combined,
                        },
                        0.0,
                    )
                });
                entry.1 += track.score * weight;
            }
        };

        // Apply weights from configuration
        if let Some(tracks) = semantic {
            merge_tracks(&mut combined, tracks, self.config.weight_semantic);
        }
        if let Some(tracks) = acoustic {
            merge_tracks(&mut combined, tracks, self.config.weight_acoustic);
        }
        if let Some(tracks) = categorical {
            merge_tracks(&mut combined, tracks, self.config.weight_categorical);
        }

        // Sort by combined score and take top N
        let mut results: Vec<SimilarTrack> = combined
            .into_values()
            .map(|(mut track, score)| {
                track.score = score;
                track
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit as usize);

        Ok(results)
    }
}

/// Row struct for sqlx queries
#[derive(Debug, sqlx::FromRow)]
struct SimilarTrackRow {
    track_id: Uuid,
    title: String,
    artist_name: Option<String>,
    album_title: Option<String>,
    score: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity_type_serialization() {
        let semantic = SimilarityType::Semantic;
        let json = serde_json::to_string(&semantic).unwrap();
        assert_eq!(json, r#""semantic""#);

        let combined = SimilarityType::Combined;
        let json = serde_json::to_string(&combined).unwrap();
        assert_eq!(json, r#""combined""#);
    }

    #[test]
    fn test_audio_features_default() {
        let features = AudioFeatures::default();
        assert!(features.energy.is_none());
        assert!(features.bpm.is_none());
    }

    #[test]
    fn test_validate_limit() {
        // Normal values are passed through
        assert_eq!(validate_limit(10), 10);
        assert_eq!(validate_limit(50), 50);

        // Values below minimum are clamped to 1
        assert_eq!(validate_limit(0), 1);
        assert_eq!(validate_limit(-10), 1);

        // Values above maximum are clamped to MAX_SIMILARITY_RESULTS
        assert_eq!(validate_limit(200), MAX_SIMILARITY_RESULTS);
        assert_eq!(validate_limit(1000), MAX_SIMILARITY_RESULTS);

        // Edge cases at boundaries
        assert_eq!(validate_limit(1), 1);
        assert_eq!(
            validate_limit(MAX_SIMILARITY_RESULTS),
            MAX_SIMILARITY_RESULTS
        );
    }

    #[test]
    fn test_default_weights_sum_to_one() {
        // Verify default weights are properly balanced
        let total = DEFAULT_WEIGHT_SEMANTIC + DEFAULT_WEIGHT_ACOUSTIC + DEFAULT_WEIGHT_CATEGORICAL;
        assert!(
            (total - 1.0).abs() < f64::EPSILON,
            "Default weights should sum to 1.0"
        );
    }

    // ==========================================================================
    // SimilarityConfig Tests
    // ==========================================================================

    #[test]
    fn test_similarity_config_default() {
        let config = SimilarityConfig::default();
        assert!((config.weight_semantic - 0.5).abs() < f64::EPSILON);
        assert!((config.weight_acoustic - 0.3).abs() < f64::EPSILON);
        assert!((config.weight_categorical - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similarity_config_new_valid() {
        let config = SimilarityConfig::new(0.4, 0.4, 0.2).unwrap();
        assert!((config.weight_semantic - 0.4).abs() < f64::EPSILON);
        assert!((config.weight_acoustic - 0.4).abs() < f64::EPSILON);
        assert!((config.weight_categorical - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similarity_config_new_invalid_sum() {
        let result = SimilarityConfig::new(0.5, 0.5, 0.5);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SimilarityConfigError::WeightsSumInvalid { .. }));
    }

    #[test]
    fn test_similarity_config_validate_valid() {
        let config = SimilarityConfig {
            weight_semantic: 0.6,
            weight_acoustic: 0.3,
            weight_categorical: 0.1,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_similarity_config_validate_invalid() {
        let config = SimilarityConfig {
            weight_semantic: 0.5,
            weight_acoustic: 0.4,
            weight_categorical: 0.2,
        };
        let result = config.validate();
        assert!(result.is_err());
        if let Err(SimilarityConfigError::WeightsSumInvalid { total }) = result {
            assert!((total - 1.1).abs() < 0.01);
        } else {
            panic!("Expected WeightsSumInvalid error");
        }
    }

    #[test]
    fn test_similarity_config_validate_within_epsilon() {
        // Test that small floating point errors within epsilon are accepted
        let config = SimilarityConfig {
            weight_semantic: 0.333333,
            weight_acoustic: 0.333333,
            weight_categorical: 0.333334, // Sum is 1.0 within epsilon
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_similarity_config_from_env_defaults() {
        // When env vars are not set, should use defaults
        // Clear any existing env vars first (for isolation)
        std::env::remove_var("SIMILARITY_WEIGHT_SEMANTIC");
        std::env::remove_var("SIMILARITY_WEIGHT_ACOUSTIC");
        std::env::remove_var("SIMILARITY_WEIGHT_CATEGORICAL");

        let config = SimilarityConfig::from_env().unwrap();
        assert!((config.weight_semantic - 0.5).abs() < f64::EPSILON);
        assert!((config.weight_acoustic - 0.3).abs() < f64::EPSILON);
        assert!((config.weight_categorical - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similarity_config_from_env_custom() {
        // Set custom env vars
        std::env::set_var("SIMILARITY_WEIGHT_SEMANTIC", "0.6");
        std::env::set_var("SIMILARITY_WEIGHT_ACOUSTIC", "0.25");
        std::env::set_var("SIMILARITY_WEIGHT_CATEGORICAL", "0.15");

        let config = SimilarityConfig::from_env().unwrap();
        // Use a larger epsilon for float comparison since we're parsing from strings
        let epsilon = 0.0001;
        assert!(
            (config.weight_semantic - 0.6).abs() < epsilon,
            "weight_semantic was {}, expected 0.6",
            config.weight_semantic
        );
        assert!(
            (config.weight_acoustic - 0.25).abs() < epsilon,
            "weight_acoustic was {}, expected 0.25",
            config.weight_acoustic
        );
        assert!(
            (config.weight_categorical - 0.15).abs() < epsilon,
            "weight_categorical was {}, expected 0.15",
            config.weight_categorical
        );

        // Clean up
        std::env::remove_var("SIMILARITY_WEIGHT_SEMANTIC");
        std::env::remove_var("SIMILARITY_WEIGHT_ACOUSTIC");
        std::env::remove_var("SIMILARITY_WEIGHT_CATEGORICAL");
    }

    #[test]
    fn test_similarity_config_from_env_invalid_format() {
        std::env::set_var("SIMILARITY_WEIGHT_SEMANTIC", "not_a_number");

        let result = SimilarityConfig::from_env();
        assert!(result.is_err());
        if let Err(SimilarityConfigError::InvalidWeight { var_name, value }) = result {
            assert_eq!(var_name, "SIMILARITY_WEIGHT_SEMANTIC");
            assert_eq!(value, "not_a_number");
        } else {
            panic!("Expected InvalidWeight error");
        }

        // Clean up
        std::env::remove_var("SIMILARITY_WEIGHT_SEMANTIC");
    }

    #[test]
    fn test_similarity_config_from_env_out_of_range() {
        std::env::set_var("SIMILARITY_WEIGHT_SEMANTIC", "1.5");
        std::env::remove_var("SIMILARITY_WEIGHT_ACOUSTIC");
        std::env::remove_var("SIMILARITY_WEIGHT_CATEGORICAL");

        let result = SimilarityConfig::from_env();
        assert!(result.is_err());
        if let Err(SimilarityConfigError::WeightOutOfRange { var_name, value }) = result {
            assert_eq!(var_name, "SIMILARITY_WEIGHT_SEMANTIC");
            assert!((value - 1.5).abs() < f64::EPSILON);
        } else {
            panic!("Expected WeightOutOfRange error");
        }

        // Clean up
        std::env::remove_var("SIMILARITY_WEIGHT_SEMANTIC");
    }

    #[test]
    fn test_similarity_config_from_env_negative_weight() {
        std::env::set_var("SIMILARITY_WEIGHT_ACOUSTIC", "-0.1");
        std::env::remove_var("SIMILARITY_WEIGHT_SEMANTIC");
        std::env::remove_var("SIMILARITY_WEIGHT_CATEGORICAL");

        let result = SimilarityConfig::from_env();
        assert!(result.is_err());
        if let Err(SimilarityConfigError::WeightOutOfRange { var_name, value }) = result {
            assert_eq!(var_name, "SIMILARITY_WEIGHT_ACOUSTIC");
            assert!((value + 0.1).abs() < f64::EPSILON);
        } else {
            panic!("Expected WeightOutOfRange error");
        }

        // Clean up
        std::env::remove_var("SIMILARITY_WEIGHT_ACOUSTIC");
    }

    #[test]
    fn test_similarity_config_error_display() {
        let err = SimilarityConfigError::InvalidWeight {
            var_name: "TEST_VAR".to_string(),
            value: "bad".to_string(),
        };
        assert!(err.to_string().contains("TEST_VAR"));
        assert!(err.to_string().contains("bad"));

        let err = SimilarityConfigError::WeightOutOfRange {
            var_name: "TEST_VAR".to_string(),
            value: 1.5,
        };
        assert!(err.to_string().contains("TEST_VAR"));
        assert!(err.to_string().contains("1.5"));

        let err = SimilarityConfigError::WeightsSumInvalid { total: 0.8 };
        assert!(err.to_string().contains("0.8"));
    }
}
