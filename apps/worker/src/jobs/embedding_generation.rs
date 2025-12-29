//! AI embedding generation job
//!
//! Generates vector embeddings for tracks using Ollama.
//! Embeddings are used for semantic search and AI recommendations.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use crate::error::WorkerResult;
use crate::AppState;

/// Job-level timeout for embedding generation (2 minutes)
const JOB_TIMEOUT_SECS: u64 = 120;

/// Embedding generation job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingGenerationJob {
    /// Track ID to generate embedding for
    pub track_id: String,
    /// Whether to regenerate even if embeddings exist
    #[serde(default)]
    pub force: bool,
}

/// Track info for embedding generation
#[derive(Debug, sqlx::FromRow)]
struct TrackMetadata {
    #[allow(dead_code)]
    id: sqlx::types::Uuid,
    title: String,
    artist_name: Option<String>,
    album_title: Option<String>,
    genres: Vec<String>,
    ai_mood: Vec<String>,
    ai_tags: Vec<String>,
    ai_description: Option<String>,
}

/// Execute the embedding generation job
pub async fn execute(state: &AppState, job: &EmbeddingGenerationJob) -> WorkerResult<()> {
    let track_id: sqlx::types::Uuid = job
        .track_id
        .parse()
        .map_err(|e| crate::WorkerError::InvalidPayload(format!("Invalid track ID: {}", e)))?;

    tracing::info!(track_id = %track_id, "Generating embedding for track");

    // Wrap in timeout to prevent runaway jobs
    let result = timeout(
        Duration::from_secs(JOB_TIMEOUT_SECS),
        execute_inner(state, track_id, job.force),
    )
    .await;

    match result {
        Ok(inner_result) => inner_result,
        Err(_) => {
            tracing::error!(track_id = %track_id, timeout_secs = JOB_TIMEOUT_SECS, "Embedding generation timed out");
            Err(crate::WorkerError::Timeout {
                seconds: JOB_TIMEOUT_SECS,
            })
        }
    }
}

/// Inner execution logic (called within timeout)
async fn execute_inner(
    state: &AppState,
    track_id: sqlx::types::Uuid,
    force: bool,
) -> WorkerResult<()> {
    // Ensure Ollama client is available
    let ollama = state.ollama.as_ref().ok_or_else(|| {
        crate::WorkerError::OllamaUnavailable(
            "Ollama client not initialized - is Ollama running?".to_string(),
        )
    })?;

    // Check if both embeddings already exist (unless force regeneration)
    // Using EXISTS for efficiency - avoids fetching data
    // We check for both title AND description embeddings to ensure completeness
    if !force {
        let exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM track_embeddings WHERE track_id = $1 AND title_embedding IS NOT NULL AND description_embedding IS NOT NULL)",
        )
        .bind(track_id)
        .fetch_one(&state.db)
        .await?;

        if exists.0 {
            tracing::debug!(track_id = %track_id, "Both embeddings already exist, skipping");
            return Ok(());
        }
    }

    // Load track metadata
    let track: TrackMetadata = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.title,
            a.name as artist_name,
            al.title as album_title,
            t.genres,
            t.ai_mood,
            t.ai_tags,
            t.ai_description
        FROM tracks t
        LEFT JOIN artists a ON t.artist_id = a.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.id = $1
        "#,
    )
    .bind(track_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::WorkerError::NotFound(format!("Track not found: {}", track_id)))?;

    // Construct text for title embedding
    let title_text = format!(
        "{} by {}",
        track.title,
        track.artist_name.as_deref().unwrap_or("Unknown Artist")
    );

    // Construct rich description text for semantic embedding
    let description_text = build_description_text(&track);

    tracing::debug!(
        track_id = %track_id,
        title_text = %title_text,
        description_len = description_text.len(),
        "Generating embeddings"
    );

    // Generate embeddings via Ollama (parallel for performance)
    let (title_embedding, description_embedding) = tokio::try_join!(
        ollama.generate_embedding(&title_text),
        ollama.generate_embedding(&description_text)
    )?;

    // Validate embedding dimensions
    resonance_ollama_client::validate_embedding_dimension(&title_embedding)?;
    resonance_ollama_client::validate_embedding_dimension(&description_embedding)?;

    // Convert embeddings to pgvector format (text representation)
    let title_vec_str = format_embedding_for_pgvector(&title_embedding)?;
    let description_vec_str = format_embedding_for_pgvector(&description_embedding)?;

    // Upsert embeddings into track_embeddings table
    sqlx::query(
        r#"
        INSERT INTO track_embeddings (track_id, title_embedding, description_embedding, created_at, updated_at)
        VALUES ($1, $2::vector, $3::vector, NOW(), NOW())
        ON CONFLICT (track_id) DO UPDATE SET
            title_embedding = EXCLUDED.title_embedding,
            description_embedding = EXCLUDED.description_embedding,
            updated_at = NOW()
        "#,
    )
    .bind(track_id)
    .bind(&title_vec_str)
    .bind(&description_vec_str)
    .execute(&state.db)
    .await?;

    tracing::info!(
        track_id = %track_id,
        title = %track.title,
        "Embedding generation completed"
    );

    Ok(())
}

/// Build rich description text from track metadata
fn build_description_text(track: &TrackMetadata) -> String {
    let mut parts = Vec::new();

    // Title and artist
    parts.push(format!(
        "\"{}\" by {}",
        track.title,
        track.artist_name.as_deref().unwrap_or("Unknown Artist")
    ));

    // Album
    if let Some(album) = &track.album_title {
        parts.push(format!("from the album \"{}\"", album));
    }

    // Genres
    if !track.genres.is_empty() {
        parts.push(format!("Genre: {}", track.genres.join(", ")));
    }

    // AI mood tags
    if !track.ai_mood.is_empty() {
        parts.push(format!("Mood: {}", track.ai_mood.join(", ")));
    }

    // AI tags
    if !track.ai_tags.is_empty() {
        parts.push(format!("Tags: {}", track.ai_tags.join(", ")));
    }

    // AI description
    if let Some(desc) = &track.ai_description {
        parts.push(desc.clone());
    }

    parts.join(". ")
}

/// Format embedding vector as pgvector string representation
/// Returns an error if any values are non-finite (NaN/inf)
fn format_embedding_for_pgvector(embedding: &[f32]) -> WorkerResult<String> {
    // Validate that all values are finite to prevent database errors
    if embedding.iter().any(|v| !v.is_finite()) {
        return Err(crate::WorkerError::InvalidPayload(
            "Embedding contains non-finite values (NaN/inf)".to_string(),
        ));
    }

    let values: Vec<String> = embedding.iter().map(|v| format!("{:.6}", v)).collect();
    Ok(format!("[{}]", values.join(",")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_description_text_full() {
        let track = TrackMetadata {
            id: sqlx::types::Uuid::new_v4(),
            title: "Bohemian Rhapsody".to_string(),
            artist_name: Some("Queen".to_string()),
            album_title: Some("A Night at the Opera".to_string()),
            genres: vec!["Rock".to_string(), "Progressive Rock".to_string()],
            ai_mood: vec!["epic".to_string(), "dramatic".to_string()],
            ai_tags: vec!["operatic".to_string(), "multi-section".to_string()],
            ai_description: Some("A groundbreaking rock opera masterpiece".to_string()),
        };

        let text = build_description_text(&track);

        assert!(text.contains("\"Bohemian Rhapsody\" by Queen"));
        assert!(text.contains("A Night at the Opera"));
        assert!(text.contains("Rock, Progressive Rock"));
        assert!(text.contains("epic, dramatic"));
        assert!(text.contains("operatic, multi-section"));
        assert!(text.contains("groundbreaking rock opera"));
    }

    #[test]
    fn test_build_description_text_minimal() {
        let track = TrackMetadata {
            id: sqlx::types::Uuid::new_v4(),
            title: "Unknown Track".to_string(),
            artist_name: None,
            album_title: None,
            genres: vec![],
            ai_mood: vec![],
            ai_tags: vec![],
            ai_description: None,
        };

        let text = build_description_text(&track);

        assert!(text.contains("\"Unknown Track\" by Unknown Artist"));
        assert!(!text.contains("Genre:"));
        assert!(!text.contains("Mood:"));
    }

    #[test]
    fn test_format_embedding_for_pgvector() {
        let embedding = vec![0.1, 0.2, -0.3, 0.0];
        let result = format_embedding_for_pgvector(&embedding).unwrap();

        assert_eq!(result, "[0.100000,0.200000,-0.300000,0.000000]");
    }

    #[test]
    fn test_format_embedding_for_pgvector_empty() {
        let embedding: Vec<f32> = vec![];
        let result = format_embedding_for_pgvector(&embedding).unwrap();

        assert_eq!(result, "[]");
    }

    #[test]
    fn test_format_embedding_for_pgvector_rejects_nan() {
        let embedding = vec![0.1, f32::NAN, 0.3];
        let result = format_embedding_for_pgvector(&embedding);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-finite"));
    }

    #[test]
    fn test_format_embedding_for_pgvector_rejects_inf() {
        let embedding = vec![0.1, f32::INFINITY, 0.3];
        let result = format_embedding_for_pgvector(&embedding);
        assert!(result.is_err());
    }
}
