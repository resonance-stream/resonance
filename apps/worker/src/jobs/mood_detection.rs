//! AI mood detection job
//!
//! Analyzes track metadata and audio features to detect mood, generate tags,
//! and create AI descriptions using Ollama LLM.

use std::time::Duration;

use resonance_ollama_client::{ChatMessage, GenerateOptions, MoodAnalysis};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use crate::error::WorkerResult;
use crate::AppState;

/// Job-level timeout for mood detection (3 minutes)
const JOB_TIMEOUT_SECS: u64 = 180;

/// Mood detection job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodDetectionJob {
    /// Track ID to analyze
    pub track_id: String,
    /// Whether to regenerate even if mood data exists
    #[serde(default)]
    pub force: bool,
}

/// Track info with audio features for mood analysis
#[derive(Debug, sqlx::FromRow)]
struct TrackWithFeatures {
    #[allow(dead_code)]
    id: sqlx::types::Uuid,
    title: String,
    artist_name: Option<String>,
    album_title: Option<String>,
    genres: Vec<String>,
    duration_ms: i32,
    audio_features: serde_json::Value,
}

/// Audio features from JSON
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // Fields parsed from JSON for future use
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

/// Execute the mood detection job
pub async fn execute(state: &AppState, job: &MoodDetectionJob) -> WorkerResult<()> {
    let track_id: sqlx::types::Uuid = job
        .track_id
        .parse()
        .map_err(|e| crate::WorkerError::InvalidPayload(format!("Invalid track ID: {}", e)))?;

    tracing::info!(track_id = %track_id, "Detecting mood for track");

    // Wrap in timeout to prevent runaway jobs
    let result = timeout(
        Duration::from_secs(JOB_TIMEOUT_SECS),
        execute_inner(state, track_id, job.force),
    )
    .await;

    match result {
        Ok(inner_result) => inner_result,
        Err(_) => {
            tracing::error!(track_id = %track_id, timeout_secs = JOB_TIMEOUT_SECS, "Mood detection timed out");
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
    // Check if mood already exists (unless force regeneration)
    if !force {
        let has_mood: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM tracks WHERE id = $1 AND ai_description IS NOT NULL AND array_length(ai_mood, 1) > 0)",
        )
        .bind(track_id)
        .fetch_one(&state.db)
        .await?;

        if has_mood.0 {
            tracing::debug!(track_id = %track_id, "Mood data already exists, skipping");
            return Ok(());
        }
    }

    // Load track with audio features
    let track: TrackWithFeatures = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.title,
            a.name as artist_name,
            al.title as album_title,
            t.genres,
            t.duration_ms,
            t.audio_features
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

    // Parse audio features
    let features: AudioFeatures =
        serde_json::from_value(track.audio_features.clone()).unwrap_or_default();

    // Build prompt for LLM
    let prompt = build_mood_prompt(&track, &features);

    tracing::debug!(
        track_id = %track_id,
        title = %track.title,
        prompt_len = prompt.len(),
        "Sending mood analysis prompt"
    );

    // Call Ollama for mood analysis
    let messages = vec![
        ChatMessage::system(MOOD_SYSTEM_PROMPT),
        ChatMessage::user(prompt),
    ];

    let options = Some(GenerateOptions {
        temperature: Some(0.3), // Lower temperature for more consistent output
        num_predict: Some(500), // Enough for JSON response
        ..Default::default()
    });

    let response = state.ollama.chat_with_options(messages, options).await?;

    // Parse the JSON response
    let analysis = parse_mood_response(&response)?;

    tracing::debug!(
        track_id = %track_id,
        moods = ?analysis.moods,
        energy = ?analysis.energy,
        "Mood analysis complete"
    );

    // Update track with mood data
    // Use array deduplication to prevent duplicate tags on re-runs
    sqlx::query(
        r#"
        UPDATE tracks
        SET
            ai_mood = $2,
            ai_tags = (SELECT ARRAY(SELECT DISTINCT unnest(ai_tags || $3))),
            ai_description = $4,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(track_id)
    .bind(&analysis.moods)
    .bind(generate_tags_from_analysis(&analysis))
    .bind(&analysis.description)
    .execute(&state.db)
    .await?;

    tracing::info!(
        track_id = %track_id,
        title = %track.title,
        mood_count = analysis.moods.len(),
        "Mood detection completed"
    );

    Ok(())
}

/// System prompt for mood analysis
const MOOD_SYSTEM_PROMPT: &str = r#"You are a music analysis expert. Analyze tracks based on their metadata and audio features to determine mood, energy level, and emotional valence.

Always respond with valid JSON in exactly this format:
{
    "moods": ["mood1", "mood2", "mood3"],
    "energy": "low" | "medium" | "high",
    "valence": "negative" | "neutral" | "positive",
    "description": "Brief 1-2 sentence description of the track's mood and feel"
}

Use common mood descriptors like: happy, sad, energetic, calm, melancholic, uplifting, aggressive, peaceful, romantic, nostalgic, dark, bright, dreamy, intense, relaxed, groovy, epic, playful, mysterious, ethereal.

Base your analysis on:
- Track title and artist style
- Genre associations
- BPM (tempo): slow < 90, medium 90-120, fast > 120
- Energy level from audio features
- Valence (musical positivity) from audio features

Respond ONLY with the JSON, no additional text."#;

/// Build the user prompt with track context
fn build_mood_prompt(track: &TrackWithFeatures, features: &AudioFeatures) -> String {
    let mut parts = Vec::new();

    // Track identity
    parts.push(format!(
        "Track: \"{}\" by {}",
        track.title,
        track.artist_name.as_deref().unwrap_or("Unknown Artist")
    ));

    // Album
    if let Some(album) = &track.album_title {
        parts.push(format!("Album: \"{}\"", album));
    }

    // Genres
    if !track.genres.is_empty() {
        parts.push(format!("Genres: {}", track.genres.join(", ")));
    }

    // Duration in minutes:seconds
    let duration_secs = track.duration_ms / 1000;
    let mins = duration_secs / 60;
    let secs = duration_secs % 60;
    parts.push(format!("Duration: {}:{:02}", mins, secs));

    // Audio features
    let mut feature_parts = Vec::new();
    if let Some(bpm) = features.bpm {
        feature_parts.push(format!("BPM: {:.0}", bpm));
    }
    if let Some(key) = &features.key {
        let mode = features.mode.as_deref().unwrap_or("");
        feature_parts.push(format!("Key: {} {}", key, mode));
    }
    if let Some(energy) = features.energy {
        feature_parts.push(format!("Energy: {:.2}", energy));
    }
    if let Some(valence) = features.valence {
        feature_parts.push(format!("Valence: {:.2}", valence));
    }
    if let Some(danceability) = features.danceability {
        feature_parts.push(format!("Danceability: {:.2}", danceability));
    }
    if let Some(acousticness) = features.acousticness {
        feature_parts.push(format!("Acousticness: {:.2}", acousticness));
    }
    if let Some(instrumentalness) = features.instrumentalness {
        feature_parts.push(format!("Instrumentalness: {:.2}", instrumentalness));
    }

    if !feature_parts.is_empty() {
        parts.push(format!("Audio Features: {}", feature_parts.join(", ")));
    }

    parts.push("\nAnalyze this track and provide mood analysis as JSON.".to_string());

    parts.join("\n")
}

/// Parse mood response from LLM, handling potential formatting issues
fn parse_mood_response(response: &str) -> WorkerResult<MoodAnalysis> {
    // Try to extract JSON from response (LLM might add extra text)
    let json_str = extract_json(response);

    let analysis: MoodAnalysis = serde_json::from_str(&json_str).map_err(|e| {
        tracing::warn!(
            response = %response,
            error = %e,
            "Failed to parse mood analysis response"
        );
        crate::WorkerError::MoodDetectionFailed(format!("Failed to parse mood analysis: {}", e))
    })?;

    // Validate semantic correctness - moods should not be empty
    if analysis.moods.is_empty() {
        return Err(crate::WorkerError::MoodDetectionFailed(
            "LLM returned empty moods array".to_string(),
        ));
    }

    if analysis.description.trim().is_empty() {
        tracing::warn!("LLM returned empty description for mood analysis");
    }

    Ok(analysis)
}

/// Extract JSON object from response text
fn extract_json(text: &str) -> String {
    // Find the first { and last }
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        if start < end {
            return text[start..=end].to_string();
        }
    }
    text.to_string()
}

/// Generate additional tags from mood analysis
fn generate_tags_from_analysis(analysis: &MoodAnalysis) -> Vec<String> {
    let mut tags = Vec::new();

    // Add energy-based tag
    match analysis.energy {
        resonance_ollama_client::EnergyLevel::Low => tags.push("slow".to_string()),
        resonance_ollama_client::EnergyLevel::Medium => {}
        resonance_ollama_client::EnergyLevel::High => tags.push("fast".to_string()),
    }

    // Add valence-based tag
    match analysis.valence {
        resonance_ollama_client::Valence::Negative => tags.push("dark".to_string()),
        resonance_ollama_client::Valence::Neutral => {}
        resonance_ollama_client::Valence::Positive => tags.push("bright".to_string()),
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_mood_prompt() {
        let track = TrackWithFeatures {
            id: sqlx::types::Uuid::new_v4(),
            title: "Bohemian Rhapsody".to_string(),
            artist_name: Some("Queen".to_string()),
            album_title: Some("A Night at the Opera".to_string()),
            genres: vec!["Rock".to_string(), "Progressive Rock".to_string()],
            duration_ms: 354000,
            audio_features: serde_json::json!({
                "bpm": 145.0,
                "energy": 0.75,
                "valence": 0.45
            }),
        };

        let features = AudioFeatures {
            bpm: Some(145.0),
            energy: Some(0.75),
            valence: Some(0.45),
            ..Default::default()
        };

        let prompt = build_mood_prompt(&track, &features);

        assert!(prompt.contains("Bohemian Rhapsody"));
        assert!(prompt.contains("Queen"));
        assert!(prompt.contains("Rock"));
        assert!(prompt.contains("BPM: 145"));
        assert!(prompt.contains("Energy: 0.75"));
    }

    #[test]
    fn test_extract_json() {
        let text = "Here is the analysis:\n{\"moods\": [\"happy\"]}\nDone.";
        let json = extract_json(text);
        assert_eq!(json, "{\"moods\": [\"happy\"]}");

        let clean = "{\"moods\": [\"calm\"]}";
        assert_eq!(extract_json(clean), clean);
    }

    #[test]
    fn test_parse_mood_response() {
        let response = r#"{"moods": ["energetic", "happy"], "energy": "high", "valence": "positive", "description": "An upbeat track"}"#;
        let analysis = parse_mood_response(response).unwrap();

        assert_eq!(analysis.moods, vec!["energetic", "happy"]);
        assert_eq!(analysis.energy, resonance_ollama_client::EnergyLevel::High);
    }

    #[test]
    fn test_generate_tags_from_analysis() {
        let analysis = MoodAnalysis {
            moods: vec!["happy".to_string()],
            energy: resonance_ollama_client::EnergyLevel::High,
            valence: resonance_ollama_client::Valence::Positive,
            description: "Test".to_string(),
        };

        let tags = generate_tags_from_analysis(&analysis);
        assert!(tags.contains(&"fast".to_string()));
        assert!(tags.contains(&"bright".to_string()));
    }

    #[test]
    fn test_extract_json_no_json() {
        let text = "No JSON here, just text";
        let result = extract_json(text);
        assert_eq!(result, text); // Returns original text
    }

    #[test]
    fn test_extract_json_nested_braces() {
        let text = r#"{"outer": {"inner": "value"}}"#;
        let result = extract_json(text);
        assert_eq!(result, text);
    }

    #[test]
    fn test_parse_mood_response_invalid_json() {
        let response = "not json at all";
        let result = parse_mood_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mood_response_empty_moods() {
        let response =
            r#"{"moods": [], "energy": "low", "valence": "neutral", "description": "Empty"}"#;
        let result = parse_mood_response(response);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("empty moods array"));
    }

    #[test]
    fn test_generate_tags_medium_neutral() {
        let analysis = MoodAnalysis {
            moods: vec!["calm".to_string()],
            energy: resonance_ollama_client::EnergyLevel::Medium,
            valence: resonance_ollama_client::Valence::Neutral,
            description: "Test".to_string(),
        };
        let tags = generate_tags_from_analysis(&analysis);
        assert!(tags.is_empty()); // Medium energy and neutral valence should produce no tags
    }

    #[test]
    fn test_build_mood_prompt_no_features() {
        let track = TrackWithFeatures {
            id: sqlx::types::Uuid::new_v4(),
            title: "Mystery Track".to_string(),
            artist_name: None,
            album_title: None,
            genres: vec![],
            duration_ms: 180000,
            audio_features: serde_json::json!({}),
        };

        let features = AudioFeatures::default();

        let prompt = build_mood_prompt(&track, &features);

        assert!(prompt.contains("Mystery Track"));
        assert!(prompt.contains("Unknown Artist"));
        assert!(prompt.contains("3:00")); // 180000ms = 3:00
        assert!(!prompt.contains("BPM:"));
        assert!(!prompt.contains("Energy:"));
    }
}
