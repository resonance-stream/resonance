//! Semantic search service
//!
//! Provides AI-powered semantic search capabilities:
//! - Natural language query search using embeddings
//! - Mood-based track discovery
//! - Combined with existing similarity features
//!
//! Uses pgvector for efficient vector similarity search.

// Service is used via GraphQL schema builder, not direct crate imports
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// Maximum number of search results
const MAX_SEARCH_RESULTS: i32 = 100;

/// Expected embedding dimension for nomic-embed-text model
/// Must match the dimension used by the Ollama client
const EXPECTED_EMBEDDING_DIMENSION: usize = 768;

/// Validate and clamp the limit parameter
fn validate_limit(limit: i32) -> i32 {
    limit.clamp(1, MAX_SEARCH_RESULTS)
}

/// Semantic search service
#[derive(Clone)]
pub struct SearchService {
    db: PgPool,
}

/// A track with its search relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredTrack {
    pub track_id: Uuid,
    pub title: String,
    pub artist_id: Uuid,
    pub artist_name: Option<String>,
    pub album_id: Option<Uuid>,
    pub album_title: Option<String>,
    pub score: f64,
}

/// Semantic search result containing tracks and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    /// Matching tracks with scores
    pub tracks: Vec<ScoredTrack>,
    /// How the AI interpreted the query (for display)
    pub interpretation: Option<String>,
}

impl SearchService {
    /// Create a new search service
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Perform semantic search using a pre-computed query embedding
    ///
    /// The embedding should be generated from the user's query using Ollama.
    /// This method finds tracks whose description embeddings are most similar.
    ///
    /// # Arguments
    /// * `query_embedding` - 768-dimensional embedding vector from Ollama
    /// * `limit` - Maximum number of results to return
    ///
    /// # Errors
    /// - `ApiError::ValidationError` - If embedding dimension is incorrect
    /// - `ApiError::Database` - If the query fails
    #[instrument(skip(self, query_embedding))]
    pub async fn search_by_embedding(
        &self,
        query_embedding: &[f32],
        limit: i32,
    ) -> ApiResult<Vec<ScoredTrack>> {
        // Validate embedding dimension
        if query_embedding.len() != EXPECTED_EMBEDDING_DIMENSION {
            return Err(ApiError::ValidationError(format!(
                "Invalid embedding dimension: expected {}, got {}",
                EXPECTED_EMBEDDING_DIMENSION,
                query_embedding.len()
            )));
        }

        let limit = validate_limit(limit);

        // Format embedding as pgvector string for parameterized query
        let embedding_str = format_embedding(query_embedding);

        // Search using cosine distance on description embeddings
        // Uses parameterized query with $1::vector cast to prevent SQL injection
        let tracks: Vec<ScoredTrackRow> = sqlx::query_as(
            r#"
            SELECT
                t.id as track_id,
                t.title,
                t.artist_id,
                a.name as artist_name,
                t.album_id,
                al.title as album_title,
                1.0 - (te.description_embedding <=> $1::vector) as score
            FROM track_embeddings te
            JOIN tracks t ON t.id = te.track_id
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            WHERE te.description_embedding IS NOT NULL
            ORDER BY te.description_embedding <=> $1::vector
            LIMIT $2
            "#,
        )
        .bind(&embedding_str)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(tracks
            .into_iter()
            .map(|r| ScoredTrack {
                track_id: r.track_id,
                title: r.title,
                artist_id: r.artist_id,
                artist_name: r.artist_name,
                album_id: r.album_id,
                album_title: r.album_title,
                score: r.score.unwrap_or(0.0),
            })
            .collect())
    }

    /// Search tracks by mood tags
    ///
    /// Finds tracks that have any of the specified moods in their ai_mood field.
    /// Results are scored by the number of matching moods.
    ///
    /// # Arguments
    /// * `moods` - List of mood tags to search for (e.g., ["happy", "energetic"])
    /// * `limit` - Maximum number of results
    ///
    /// # Errors
    /// - `ApiError::ValidationError` - If no moods are provided
    /// - `ApiError::Database` - If the query fails
    #[instrument(skip(self))]
    pub async fn search_by_mood(
        &self,
        moods: &[String],
        limit: i32,
    ) -> ApiResult<Vec<ScoredTrack>> {
        if moods.is_empty() {
            return Err(ApiError::ValidationError(
                "At least one mood must be specified".into(),
            ));
        }

        let limit = validate_limit(limit);

        // Normalize moods to lowercase for case-insensitive matching
        let moods_lower: Vec<String> = moods.iter().map(|m| m.to_lowercase()).collect();

        // Find tracks with matching moods, scored by number of matches
        // Uses LATERAL JOIN to correctly unnest and compare track moods
        let tracks: Vec<ScoredTrackRow> = sqlx::query_as(
            r#"
            WITH mood_search AS (
                SELECT unnest($1::text[]) as mood
            )
            SELECT
                t.id as track_id,
                t.title,
                t.artist_id,
                a.name as artist_name,
                t.album_id,
                al.title as album_title,
                -- Score based on number of matching moods / total query moods
                COUNT(DISTINCT ms.mood)::float / $2::float as score
            FROM tracks t
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            JOIN LATERAL (
                SELECT LOWER(unnest(t.ai_mood)) AS mood
            ) tm ON TRUE
            JOIN mood_search ms ON LOWER(ms.mood) = tm.mood
            GROUP BY t.id, t.title, t.artist_id, a.name, t.album_id, al.title
            ORDER BY score DESC, t.play_count DESC
            LIMIT $3
            "#,
        )
        .bind(&moods_lower)
        .bind(moods_lower.len() as i32)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(tracks
            .into_iter()
            .map(|r| ScoredTrack {
                track_id: r.track_id,
                title: r.title,
                artist_id: r.artist_id,
                artist_name: r.artist_name,
                album_id: r.album_id,
                album_title: r.album_title,
                score: r.score.unwrap_or(0.0),
            })
            .collect())
    }

    /// Get available mood tags in the library
    ///
    /// Returns a list of unique mood tags with their track counts.
    #[instrument(skip(self))]
    pub async fn get_available_moods(&self) -> ApiResult<Vec<MoodTag>> {
        let moods: Vec<MoodTagRow> = sqlx::query_as(
            r#"
            SELECT
                mood,
                COUNT(*) as track_count
            FROM (
                SELECT unnest(ai_mood) as mood
                FROM tracks
                WHERE array_length(ai_mood, 1) > 0
            ) moods
            GROUP BY mood
            ORDER BY track_count DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(moods
            .into_iter()
            .map(|r| MoodTag {
                name: r.mood,
                track_count: r.track_count,
            })
            .collect())
    }

    /// Check if any tracks have embeddings for semantic search
    #[instrument(skip(self))]
    pub async fn has_embeddings(&self) -> ApiResult<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM track_embeddings WHERE description_embedding IS NOT NULL",
        )
        .fetch_one(&self.db)
        .await?;
        Ok(count.0 > 0)
    }
}

/// Mood tag with usage count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodTag {
    pub name: String,
    pub track_count: i64,
}

/// Format a vector as pgvector literal string with fixed precision
/// Non-finite values (NaN/inf) are sanitized to 0.0 to prevent database errors
fn format_embedding(embedding: &[f32]) -> String {
    let values: Vec<String> = embedding
        .iter()
        .map(|v| {
            // Sanitize non-finite values to prevent database casting errors
            let v = if v.is_finite() { *v } else { 0.0 };
            format!("{:.6}", v)
        })
        .collect();
    format!("[{}]", values.join(","))
}

/// Row struct for sqlx queries
#[derive(Debug, sqlx::FromRow)]
struct ScoredTrackRow {
    track_id: Uuid,
    title: String,
    artist_id: Uuid,
    artist_name: Option<String>,
    album_id: Option<Uuid>,
    album_title: Option<String>,
    score: Option<f64>,
}

/// Row struct for mood tag queries
#[derive(Debug, sqlx::FromRow)]
struct MoodTagRow {
    mood: String,
    track_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_embedding() {
        let embedding = vec![0.1, 0.2, 0.3];
        let result = format_embedding(&embedding);
        assert_eq!(result, "[0.100000,0.200000,0.300000]");
    }

    #[test]
    fn test_format_embedding_empty() {
        let embedding: Vec<f32> = vec![];
        let result = format_embedding(&embedding);
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_format_embedding_precision() {
        // Verify consistent precision for pgvector compatibility
        let embedding = vec![0.123_456_78, -0.987_654_3];
        let result = format_embedding(&embedding);
        // Should format to 6 decimal places
        assert_eq!(result, "[0.123457,-0.987654]");
    }

    #[test]
    fn test_format_embedding_sanitizes_nan() {
        let embedding = vec![0.1, f32::NAN, 0.3];
        let result = format_embedding(&embedding);
        // NaN should be replaced with 0.0
        assert_eq!(result, "[0.100000,0.000000,0.300000]");
    }

    #[test]
    fn test_format_embedding_sanitizes_inf() {
        let embedding = vec![f32::INFINITY, -f32::INFINITY, 0.5];
        let result = format_embedding(&embedding);
        // Infinity should be replaced with 0.0
        assert_eq!(result, "[0.000000,0.000000,0.500000]");
    }

    #[test]
    fn test_validate_limit() {
        assert_eq!(validate_limit(10), 10);
        assert_eq!(validate_limit(0), 1);
        assert_eq!(validate_limit(-5), 1);
        assert_eq!(validate_limit(200), MAX_SEARCH_RESULTS);
    }
}
