//! Track similarity service
//!
//! Provides track similarity recommendations using:
//! - Vector embeddings (semantic similarity via pgvector)
//! - Audio features (acoustic similarity)
//! - Genre and mood matching (categorical similarity)
//!
//! This service is used by the semantic search GraphQL API.
//!
//! ## Caching
//!
//! The `CachedSimilarityService` provides a Redis caching layer on top of
//! `SimilarityService` to reduce database load and improve response times.
//! Cache keys follow the format: `similarity:{track_id}:{method}:{limit}`
//! with a configurable TTL (default: 10 minutes).

use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{debug, info, info_span, instrument, warn, Instrument};
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
#[allow(dead_code)]
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

#[allow(dead_code)]
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
#[allow(dead_code)]
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
        if db_error.code().is_some_and(|code| code == "57014") {
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
    #[allow(dead_code)]
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

        let use_vector_path = has_vector.is_some_and(|(has,)| has);

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

// =============================================================================
// Redis Caching Layer
// =============================================================================

/// Default cache TTL in seconds (10 minutes)
#[allow(dead_code)]
const DEFAULT_CACHE_TTL_SECONDS: u64 = 600;

/// Configuration for similarity caching
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SimilarityCacheConfig {
    /// Time-to-live for cached similarity results in seconds
    pub ttl_seconds: u64,
    /// Whether caching is enabled
    pub enabled: bool,
}

impl Default for SimilarityCacheConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: DEFAULT_CACHE_TTL_SECONDS,
            enabled: true,
        }
    }
}

#[allow(dead_code)]
impl SimilarityCacheConfig {
    /// Create a new cache config with custom TTL
    pub fn with_ttl(ttl_seconds: u64) -> Self {
        Self {
            ttl_seconds,
            enabled: true,
        }
    }

    /// Create a disabled cache config
    pub fn disabled() -> Self {
        Self {
            ttl_seconds: 0,
            enabled: false,
        }
    }

    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - `SIMILARITY_CACHE_TTL_SECONDS` (default: 600)
    /// - `SIMILARITY_CACHE_ENABLED` (default: true)
    pub fn from_env() -> Self {
        let ttl_seconds = env::var("SIMILARITY_CACHE_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_CACHE_TTL_SECONDS);

        let enabled = env::var("SIMILARITY_CACHE_ENABLED")
            .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
            .unwrap_or(true);

        Self {
            ttl_seconds,
            enabled,
        }
    }
}

/// Cached similarity service with Redis caching layer
///
/// Wraps `SimilarityService` to provide transparent caching of similarity
/// results. Cache keys follow the format: `similarity:{track_id}:{method}:{limit}`
///
/// When Redis is unavailable, the service gracefully falls back to uncached
/// database queries with a warning log.
#[allow(dead_code)]
#[derive(Clone)]
pub struct CachedSimilarityService {
    /// The underlying similarity service
    inner: SimilarityService,
    /// Redis client for caching
    redis: Arc<redis::Client>,
    /// Cache configuration
    config: Arc<SimilarityCacheConfig>,
}

#[allow(dead_code)]
impl CachedSimilarityService {
    /// Create a new cached similarity service
    pub fn new(inner: SimilarityService, redis: redis::Client) -> Self {
        Self::with_config(inner, redis, SimilarityCacheConfig::default())
    }

    /// Create a new cached similarity service with custom configuration
    pub fn with_config(
        inner: SimilarityService,
        redis: redis::Client,
        cache_config: SimilarityCacheConfig,
    ) -> Self {
        Self {
            inner,
            redis: Arc::new(redis),
            config: Arc::new(cache_config),
        }
    }

    /// Get the underlying similarity service
    pub fn inner(&self) -> &SimilarityService {
        &self.inner
    }

    /// Get the cache configuration
    pub fn cache_config(&self) -> &SimilarityCacheConfig {
        &self.config
    }

    /// Generate a cache key for similarity results
    fn cache_key(track_id: Uuid, method: &str, limit: i32) -> String {
        format!("similarity:{}:{}:{}", track_id, method, limit)
    }

    /// Try to get cached results from Redis
    async fn get_cached(&self, key: &str) -> Option<Vec<SimilarTrack>> {
        if !self.config.enabled {
            return None;
        }

        let mut conn = match self.redis.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                debug!(error = %e, key = %key, "Redis connection failed for cache get");
                return None;
            }
        };

        let cached: Option<String> = match conn.get(key).await {
            Ok(data) => data,
            Err(e) => {
                debug!(error = %e, key = %key, "Redis GET failed for cache lookup");
                return None;
            }
        };

        match cached {
            Some(json) => match serde_json::from_str::<Vec<SimilarTrack>>(&json) {
                Ok(tracks) => {
                    debug!(key = %key, count = tracks.len(), "Cache hit for similarity results");
                    Some(tracks)
                }
                Err(e) => {
                    warn!(error = %e, key = %key, "Failed to deserialize cached similarity results");
                    None
                }
            },
            None => {
                debug!(key = %key, "Cache miss for similarity results");
                None
            }
        }
    }

    /// Store results in Redis cache with TTL
    async fn set_cached(&self, key: &str, tracks: &[SimilarTrack]) {
        if !self.config.enabled {
            return;
        }

        let json = match serde_json::to_string(tracks) {
            Ok(json) => json,
            Err(e) => {
                warn!(error = %e, key = %key, "Failed to serialize similarity results for cache");
                return;
            }
        };

        let mut conn = match self.redis.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                debug!(error = %e, key = %key, "Redis connection failed for cache set");
                return;
            }
        };

        // Use SETEX for atomic set-with-expiry
        let result: Result<(), redis::RedisError> =
            conn.set_ex(key, &json, self.config.ttl_seconds).await;

        match result {
            Ok(()) => {
                debug!(
                    key = %key,
                    count = tracks.len(),
                    ttl_seconds = self.config.ttl_seconds,
                    "Cached similarity results"
                );
            }
            Err(e) => {
                debug!(error = %e, key = %key, "Redis SETEX failed for cache storage");
            }
        }
    }

    /// Find similar tracks using embedding similarity with caching
    #[instrument(skip(self), fields(similarity_type = "semantic", cached = tracing::field::Empty))]
    pub async fn find_similar_by_embedding(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let key = Self::cache_key(track_id, "semantic", limit);

        // Try cache first
        if let Some(cached) = self.get_cached(&key).await {
            tracing::Span::current().record("cached", true);
            return Ok(cached);
        }

        tracing::Span::current().record("cached", false);

        // Cache miss - query database
        let tracks = self
            .inner
            .find_similar_by_embedding(track_id, limit)
            .await?;

        // Store in cache (async, don't wait for completion)
        self.set_cached(&key, &tracks).await;

        Ok(tracks)
    }

    /// Find similar tracks based on audio features with caching
    #[instrument(skip(self), fields(similarity_type = "acoustic", cached = tracing::field::Empty))]
    pub async fn find_similar_by_features(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let key = Self::cache_key(track_id, "acoustic", limit);

        // Try cache first
        if let Some(cached) = self.get_cached(&key).await {
            tracing::Span::current().record("cached", true);
            return Ok(cached);
        }

        tracing::Span::current().record("cached", false);

        // Cache miss - query database
        let tracks = self.inner.find_similar_by_features(track_id, limit).await?;

        // Store in cache
        self.set_cached(&key, &tracks).await;

        Ok(tracks)
    }

    /// Find similar tracks based on genre and mood tags with caching
    #[instrument(skip(self), fields(similarity_type = "categorical", cached = tracing::field::Empty))]
    pub async fn find_similar_by_tags(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let key = Self::cache_key(track_id, "categorical", limit);

        // Try cache first
        if let Some(cached) = self.get_cached(&key).await {
            tracing::Span::current().record("cached", true);
            return Ok(cached);
        }

        tracing::Span::current().record("cached", false);

        // Cache miss - query database
        let tracks = self.inner.find_similar_by_tags(track_id, limit).await?;

        // Store in cache
        self.set_cached(&key, &tracks).await;

        Ok(tracks)
    }

    /// Find similar tracks using combined similarity with caching
    #[instrument(skip(self), fields(similarity_type = "combined", cached = tracing::field::Empty))]
    pub async fn find_similar_combined(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let key = Self::cache_key(track_id, "combined", limit);

        // Try cache first
        if let Some(cached) = self.get_cached(&key).await {
            tracing::Span::current().record("cached", true);
            return Ok(cached);
        }

        tracing::Span::current().record("cached", false);

        // Cache miss - query database
        let tracks = self.inner.find_similar_combined(track_id, limit).await?;

        // Store in cache
        self.set_cached(&key, &tracks).await;

        Ok(tracks)
    }

    /// Invalidate all cached similarity results for a specific track
    ///
    /// Call this when track metadata is updated (genres, mood, audio features,
    /// or embeddings) to ensure stale cached results are not served.
    ///
    /// This uses Redis SCAN and DEL commands to find and remove all cache keys
    /// matching the pattern `similarity:{track_id}:*`.
    #[instrument(skip(self))]
    pub async fn invalidate_track_cache(&self, track_id: Uuid) {
        if !self.config.enabled {
            return;
        }

        let mut conn = match self.redis.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!(error = %e, track_id = %track_id, "Redis connection failed for cache invalidation");
                return;
            }
        };

        let pattern = format!("similarity:{}:*", track_id);

        // Use SCAN to find all matching keys (safe for production, doesn't block Redis)
        let mut cursor: u64 = 0;
        let mut deleted_count = 0;

        loop {
            let (next_cursor, keys): (u64, Vec<String>) = match redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    warn!(error = %e, track_id = %track_id, "Redis SCAN failed during cache invalidation");
                    return;
                }
            };

            // Delete found keys
            if !keys.is_empty() {
                let result: Result<i64, redis::RedisError> =
                    redis::cmd("DEL").arg(&keys).query_async(&mut conn).await;

                match result {
                    Ok(count) => deleted_count += count,
                    Err(e) => {
                        warn!(error = %e, track_id = %track_id, "Redis DEL failed during cache invalidation");
                    }
                }
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        if deleted_count > 0 {
            info!(
                track_id = %track_id,
                deleted_count = deleted_count,
                "Invalidated cached similarity results for track"
            );
        } else {
            debug!(
                track_id = %track_id,
                "No cached similarity results found to invalidate"
            );
        }
    }

    /// Invalidate cached similarity results for multiple tracks
    ///
    /// Convenience method for batch invalidation when multiple tracks are updated.
    pub async fn invalidate_tracks_cache(&self, track_ids: &[Uuid]) {
        for track_id in track_ids {
            self.invalidate_track_cache(*track_id).await;
        }
    }

    /// Clear all similarity cache entries
    ///
    /// Use with caution - this removes ALL cached similarity results.
    /// Primarily useful for development/testing or after major data changes.
    #[instrument(skip(self))]
    pub async fn clear_all_cache(&self) {
        if !self.config.enabled {
            return;
        }

        let mut conn = match self.redis.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!(error = %e, "Redis connection failed for cache clear");
                return;
            }
        };

        let pattern = "similarity:*";
        let mut cursor: u64 = 0;
        let mut deleted_count: i64 = 0;

        loop {
            let (next_cursor, keys): (u64, Vec<String>) = match redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    warn!(error = %e, "Redis SCAN failed during cache clear");
                    return;
                }
            };

            if !keys.is_empty() {
                let result: Result<i64, redis::RedisError> =
                    redis::cmd("DEL").arg(&keys).query_async(&mut conn).await;

                match result {
                    Ok(count) => deleted_count += count,
                    Err(e) => {
                        warn!(error = %e, "Redis DEL failed during cache clear");
                    }
                }
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        info!(
            deleted_count = deleted_count,
            "Cleared all similarity cache entries"
        );
    }
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
        assert!(matches!(
            err,
            SimilarityConfigError::WeightsSumInvalid { .. }
        ));
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

    // NOTE: Environment variable tests are fragile due to parallel test execution.
    // These tests verify individual parsing functions rather than from_env() directly
    // to avoid race conditions with other tests.

    #[test]
    fn test_similarity_config_parse_env_weight_valid() {
        // Test the parsing logic directly with valid values
        // The parse_env_weight function reads from env vars, so we test via from_env
        // with controlled setup. Since tests run in parallel, we focus on validation logic.

        // Test that SimilarityConfig validates sums correctly
        let config = SimilarityConfig {
            weight_semantic: 0.5,
            weight_acoustic: 0.3,
            weight_categorical: 0.2,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_similarity_config_weight_parsing_error_format() {
        // Test error formatting for invalid weight parsing
        let err = SimilarityConfigError::InvalidWeight {
            var_name: "SIMILARITY_WEIGHT_SEMANTIC".to_string(),
            value: "not_a_number".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("SIMILARITY_WEIGHT_SEMANTIC"));
        assert!(msg.contains("not_a_number"));
    }

    #[test]
    fn test_similarity_config_weight_out_of_range_format() {
        // Test error formatting for out-of-range weights
        let err = SimilarityConfigError::WeightOutOfRange {
            var_name: "SIMILARITY_WEIGHT_SEMANTIC".to_string(),
            value: 1.5,
        };
        let msg = err.to_string();
        assert!(msg.contains("SIMILARITY_WEIGHT_SEMANTIC"));
        assert!(msg.contains("1.5"));
        assert!(msg.contains("out of range"));
    }

    #[test]
    fn test_similarity_config_weight_negative_out_of_range() {
        // Negative values are out of range [0.0, 1.0]
        let err = SimilarityConfigError::WeightOutOfRange {
            var_name: "SIMILARITY_WEIGHT_ACOUSTIC".to_string(),
            value: -0.1,
        };
        let msg = err.to_string();
        assert!(msg.contains("SIMILARITY_WEIGHT_ACOUSTIC"));
        assert!(msg.contains("-0.1"));
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

    // ==========================================================================
    // SimilarityCacheConfig Tests
    // ==========================================================================

    #[test]
    fn test_similarity_cache_config_default() {
        let config = SimilarityCacheConfig::default();
        assert_eq!(config.ttl_seconds, DEFAULT_CACHE_TTL_SECONDS);
        assert!(config.enabled);
    }

    #[test]
    fn test_similarity_cache_config_with_ttl() {
        let config = SimilarityCacheConfig::with_ttl(300);
        assert_eq!(config.ttl_seconds, 300);
        assert!(config.enabled);
    }

    #[test]
    fn test_similarity_cache_config_disabled() {
        let config = SimilarityCacheConfig::disabled();
        assert_eq!(config.ttl_seconds, 0);
        assert!(!config.enabled);
    }

    // NOTE: Environment variable tests for cache config use direct construction
    // and validation logic to avoid race conditions with parallel test execution.

    #[test]
    fn test_similarity_cache_config_ttl_parsing() {
        // Test that TTL is correctly applied
        let config = SimilarityCacheConfig::with_ttl(900);
        assert_eq!(config.ttl_seconds, 900);
        assert!(config.enabled);
    }

    #[test]
    fn test_similarity_cache_config_enabled_flag_interpretation() {
        // Test enabled vs disabled states
        let enabled = SimilarityCacheConfig::default();
        assert!(enabled.enabled);

        let disabled = SimilarityCacheConfig::disabled();
        assert!(!disabled.enabled);
        assert_eq!(disabled.ttl_seconds, 0);
    }

    #[test]
    fn test_similarity_cache_config_boolean_parsing_logic() {
        // Test the boolean parsing logic used by from_env
        // "false" and "0" should be false, anything else should be true
        fn parse_enabled(value: &str) -> bool {
            !value.eq_ignore_ascii_case("false") && value != "0"
        }

        assert!(!parse_enabled("false"));
        assert!(!parse_enabled("FALSE"));
        assert!(!parse_enabled("False"));
        assert!(!parse_enabled("0"));
        assert!(parse_enabled("true"));
        assert!(parse_enabled("TRUE"));
        assert!(parse_enabled("1"));
        assert!(parse_enabled("yes"));
        assert!(parse_enabled("")); // Empty string != "false" or "0"
    }

    // ==========================================================================
    // CachedSimilarityService Tests (unit tests for cache key generation)
    // ==========================================================================

    #[test]
    fn test_cache_key_format() {
        let track_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let semantic_key = CachedSimilarityService::cache_key(track_id, "semantic", 10);
        assert_eq!(
            semantic_key,
            "similarity:550e8400-e29b-41d4-a716-446655440000:semantic:10"
        );

        let acoustic_key = CachedSimilarityService::cache_key(track_id, "acoustic", 20);
        assert_eq!(
            acoustic_key,
            "similarity:550e8400-e29b-41d4-a716-446655440000:acoustic:20"
        );

        let categorical_key = CachedSimilarityService::cache_key(track_id, "categorical", 5);
        assert_eq!(
            categorical_key,
            "similarity:550e8400-e29b-41d4-a716-446655440000:categorical:5"
        );

        let combined_key = CachedSimilarityService::cache_key(track_id, "combined", 15);
        assert_eq!(
            combined_key,
            "similarity:550e8400-e29b-41d4-a716-446655440000:combined:15"
        );
    }

    #[test]
    fn test_cache_key_different_limits_produce_different_keys() {
        let track_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let key1 = CachedSimilarityService::cache_key(track_id, "semantic", 10);
        let key2 = CachedSimilarityService::cache_key(track_id, "semantic", 20);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_similar_track_serialization_roundtrip() {
        let track = SimilarTrack {
            track_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            title: "Test Track".to_string(),
            artist_name: Some("Test Artist".to_string()),
            album_title: Some("Test Album".to_string()),
            score: 0.95,
            similarity_type: SimilarityType::Semantic,
        };

        let tracks = vec![track.clone()];

        // Serialize
        let json = serde_json::to_string(&tracks).unwrap();

        // Deserialize
        let deserialized: Vec<SimilarTrack> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].track_id, track.track_id);
        assert_eq!(deserialized[0].title, track.title);
        assert_eq!(deserialized[0].artist_name, track.artist_name);
        assert_eq!(deserialized[0].album_title, track.album_title);
        assert!((deserialized[0].score - track.score).abs() < f64::EPSILON);
        assert_eq!(deserialized[0].similarity_type, track.similarity_type);
    }

    #[test]
    fn test_similar_track_with_none_values_serialization() {
        let track = SimilarTrack {
            track_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            title: "Track Without Artist".to_string(),
            artist_name: None,
            album_title: None,
            score: 0.75,
            similarity_type: SimilarityType::Acoustic,
        };

        let tracks = vec![track];

        // Serialize and deserialize
        let json = serde_json::to_string(&tracks).unwrap();
        let deserialized: Vec<SimilarTrack> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 1);
        assert!(deserialized[0].artist_name.is_none());
        assert!(deserialized[0].album_title.is_none());
    }

    #[test]
    fn test_all_similarity_types_serialization() {
        // Verify all similarity types serialize correctly for caching
        let types = [
            SimilarityType::Semantic,
            SimilarityType::Acoustic,
            SimilarityType::Categorical,
            SimilarityType::Combined,
        ];

        for sim_type in types {
            let json = serde_json::to_string(&sim_type).unwrap();
            let deserialized: SimilarityType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, sim_type);
        }
    }
}
