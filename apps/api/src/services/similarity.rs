//! Track similarity service
//!
//! Provides track similarity recommendations using:
//! - Vector embeddings (semantic similarity via pgvector)
//! - Audio features (acoustic similarity)
//! - Genre and mood matching (categorical similarity)
//!
//! This service is used by the semantic search GraphQL API.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// Maximum number of similar tracks that can be requested
const MAX_SIMILARITY_RESULTS: i32 = 100;

/// Similarity weights for combined scoring
const WEIGHT_SEMANTIC: f64 = 0.5;
const WEIGHT_ACOUSTIC: f64 = 0.3;
const WEIGHT_CATEGORICAL: f64 = 0.2;

/// Validate and clamp the limit parameter to safe bounds
fn validate_limit(limit: i32) -> i32 {
    limit.clamp(1, MAX_SIMILARITY_RESULTS)
}

/// Track similarity service
#[derive(Clone)]
pub struct SimilarityService {
    db: PgPool,
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

/// Audio features extracted from JSON
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
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
    /// Create a new similarity service
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Find similar tracks using embedding similarity (pgvector)
    ///
    /// Uses cosine distance on description embeddings for semantic similarity.
    ///
    /// # Errors
    /// - `ApiError::NotFound` - If the track doesn't exist or has no embeddings
    /// - `ApiError::Database` - If the database query fails
    #[instrument(skip(self), fields(similarity_type = "semantic"))]
    pub async fn find_similar_by_embedding(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let limit = validate_limit(limit);

        // First check if the source track has embeddings
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
        .fetch_all(&self.db)
        .await?;

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
    /// Uses Euclidean distance on normalized audio features.
    ///
    /// # Errors
    /// - `ApiError::NotFound` - If the track doesn't exist or has no audio features
    /// - `ApiError::Database` - If the database query fails
    #[instrument(skip(self), fields(similarity_type = "acoustic"))]
    pub async fn find_similar_by_features(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let limit = validate_limit(limit);

        // Load source track's audio features
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
        .fetch_all(&self.db)
        .await?;

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
    ///
    /// # Errors
    /// - `ApiError::Database` - If the database query fails
    #[instrument(skip(self), fields(similarity_type = "categorical"))]
    pub async fn find_similar_by_tags(
        &self,
        track_id: Uuid,
        limit: i32,
    ) -> ApiResult<Vec<SimilarTrack>> {
        let limit = validate_limit(limit);

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
        .fetch_all(&self.db)
        .await?;

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

        // Fetch from each method, logging errors instead of silently discarding
        let semantic = match self.find_similar_by_embedding(track_id, fetch_limit).await {
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

        let acoustic = match self.find_similar_by_features(track_id, fetch_limit).await {
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

        let categorical = match self.find_similar_by_tags(track_id, fetch_limit).await {
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

        // Apply weights from constants
        if let Some(tracks) = semantic {
            merge_tracks(&mut combined, tracks, WEIGHT_SEMANTIC);
        }
        if let Some(tracks) = acoustic {
            merge_tracks(&mut combined, tracks, WEIGHT_ACOUSTIC);
        }
        if let Some(tracks) = categorical {
            merge_tracks(&mut combined, tracks, WEIGHT_CATEGORICAL);
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
    fn test_weights_sum_to_one() {
        // Verify weights are properly balanced
        let total = WEIGHT_SEMANTIC + WEIGHT_ACOUSTIC + WEIGHT_CATEGORICAL;
        assert!(
            (total - 1.0).abs() < f64::EPSILON,
            "Weights should sum to 1.0"
        );
    }
}
