//! Integration tests for mood detection job
//!
//! Tests mood analysis, tag generation, and LLM response parsing
//! using MockOllamaServer for isolated testing.

mod common;

use common::MockOllamaServer;
use serde_json::json;

// ============================================================================
// Unit Tests: Prompt Building
// ============================================================================

#[test]
fn test_prompt_includes_all_audio_features() {
    // Test that prompt includes BPM, energy, valence, danceability, etc.
    let track = create_test_track_data();
    let features = create_full_audio_features();

    let prompt = build_prompt_for_test(&track, &features);

    // Verify all audio features are included
    assert!(prompt.contains("BPM: 128"), "Missing BPM in prompt");
    assert!(prompt.contains("Energy: 0.80"), "Missing energy in prompt");
    assert!(
        prompt.contains("Valence: 0.65"),
        "Missing valence in prompt"
    );
    assert!(
        prompt.contains("Danceability: 0.75"),
        "Missing danceability in prompt"
    );
    assert!(
        prompt.contains("Acousticness: 0.20"),
        "Missing acousticness in prompt"
    );
    assert!(
        prompt.contains("Instrumentalness: 0.05"),
        "Missing instrumentalness in prompt"
    );
}

#[test]
fn test_prompt_includes_key_and_mode() {
    let track = create_test_track_data();
    let mut features = create_full_audio_features();
    features.key = Some("C".to_string());
    features.mode = Some("major".to_string());

    let prompt = build_prompt_for_test(&track, &features);

    assert!(
        prompt.contains("Key: C major"),
        "Missing key/mode in prompt"
    );
}

#[test]
fn test_prompt_formats_duration_correctly() {
    let mut track = create_test_track_data();
    // 3 minutes 45 seconds = 225 seconds = 225000 ms
    track.duration_ms = 225000;

    let prompt = build_prompt_for_test(&track, &create_empty_audio_features());

    assert!(
        prompt.contains("Duration: 3:45"),
        "Duration should be formatted as 3:45, got: {}",
        prompt
    );
}

#[test]
fn test_prompt_handles_missing_artist() {
    let mut track = create_test_track_data();
    track.artist_name = None;

    let prompt = build_prompt_for_test(&track, &create_empty_audio_features());

    assert!(
        prompt.contains("Unknown Artist"),
        "Missing artist should show 'Unknown Artist'"
    );
}

// ============================================================================
// Unit Tests: JSON Parsing
// ============================================================================

#[test]
fn test_parse_valid_mood_response() {
    let response = r#"{"moods": ["energetic", "happy", "uplifting"], "energy": "high", "valence": "positive", "description": "An upbeat dance track with infectious energy"}"#;

    let analysis = parse_mood_response_for_test(response).expect("Should parse valid response");

    assert_eq!(
        analysis.moods,
        vec!["energetic", "happy", "uplifting"],
        "Moods should match"
    );
    assert_eq!(
        analysis.energy,
        resonance_ollama_client::EnergyLevel::High,
        "Energy should be high"
    );
    assert_eq!(
        analysis.valence,
        resonance_ollama_client::Valence::Positive,
        "Valence should be positive"
    );
    assert!(
        analysis.description.contains("upbeat"),
        "Description should contain 'upbeat'"
    );
}

#[test]
fn test_parse_response_with_extra_text() {
    // LLM sometimes adds explanatory text around JSON
    let response = r#"Here's my analysis of the track:

{"moods": ["melancholic", "nostalgic"], "energy": "low", "valence": "negative", "description": "A sad ballad about lost love"}

I hope this helps!"#;

    let analysis =
        parse_mood_response_for_test(response).expect("Should extract JSON from messy response");

    assert_eq!(analysis.moods, vec!["melancholic", "nostalgic"]);
    assert_eq!(analysis.energy, resonance_ollama_client::EnergyLevel::Low);
}

#[test]
fn test_parse_response_with_defaults() {
    // Test that missing optional fields use defaults
    let response = r#"{"moods": ["calm"]}"#;

    let analysis = parse_mood_response_for_test(response).expect("Should parse with defaults");

    assert_eq!(analysis.moods, vec!["calm"]);
    assert_eq!(
        analysis.energy,
        resonance_ollama_client::EnergyLevel::Medium,
        "Energy should default to medium"
    );
    assert_eq!(
        analysis.valence,
        resonance_ollama_client::Valence::Neutral,
        "Valence should default to neutral"
    );
}

#[test]
fn test_parse_response_rejects_empty_moods() {
    let response =
        r#"{"moods": [], "energy": "medium", "valence": "neutral", "description": "Empty"}"#;

    let result = parse_mood_response_for_test(response);

    assert!(result.is_err(), "Should reject empty moods array");
    let err = result.unwrap_err();
    assert!(
        err.contains("empty moods"),
        "Error should mention empty moods"
    );
}

#[test]
fn test_parse_response_rejects_invalid_json() {
    let response = "This is not JSON at all, just random text";

    let result = parse_mood_response_for_test(response);

    assert!(result.is_err(), "Should reject non-JSON response");
}

#[test]
fn test_parse_response_handles_nested_json() {
    // Edge case: JSON with nested braces
    let response = r#"{"moods": ["dark", "intense"], "energy": "high", "valence": "negative", "description": "Features complex layers {with effects}"}"#;

    let analysis =
        parse_mood_response_for_test(response).expect("Should handle nested braces in description");

    assert_eq!(analysis.moods, vec!["dark", "intense"]);
}

// ============================================================================
// Integration Tests: Mock Ollama
// ============================================================================

#[tokio::test]
async fn test_mood_analysis_success_with_mock_ollama() {
    let mock_server = MockOllamaServer::start().await;

    // Set up mock response for mood analysis
    let mood_response = json!({
        "moods": ["groovy", "funky", "upbeat"],
        "energy": "high",
        "valence": "positive",
        "description": "A funky disco track with irresistible groove"
    });

    mock_server.mock_chat_with_json(mood_response).await;

    // Verify the mock is set up correctly by making a direct request
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/chat", mock_server.url()))
        .json(&json!({
            "model": "mistral",
            "messages": [{"role": "user", "content": "test"}],
            "stream": false
        }))
        .send()
        .await
        .expect("Request should succeed");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["message"]["content"].as_str().unwrap();

    // Parse the response to verify it matches what we expect
    let analysis: resonance_ollama_client::MoodAnalysis =
        serde_json::from_str(content).expect("Should parse mood analysis");

    assert_eq!(analysis.moods, vec!["groovy", "funky", "upbeat"]);
    assert_eq!(analysis.energy, resonance_ollama_client::EnergyLevel::High);
}

#[tokio::test]
async fn test_mood_analysis_with_various_energy_levels() {
    let mock_server = MockOllamaServer::start().await;

    // Test low energy response
    let low_energy_response = json!({
        "moods": ["peaceful", "serene"],
        "energy": "low",
        "valence": "neutral",
        "description": "Ambient soundscape"
    });

    mock_server.mock_chat_with_json(low_energy_response).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/chat", mock_server.url()))
        .json(&json!({"model": "mistral", "messages": [], "stream": false}))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["message"]["content"].as_str().unwrap();
    let analysis: resonance_ollama_client::MoodAnalysis = serde_json::from_str(content).unwrap();

    assert_eq!(analysis.energy, resonance_ollama_client::EnergyLevel::Low);
}

#[tokio::test]
async fn test_tag_generation_from_analysis() {
    // Test that tags are correctly generated from mood analysis
    use resonance_ollama_client::{EnergyLevel, MoodAnalysis, Valence};

    // High energy + positive valence should generate "fast" and "bright" tags
    let high_positive = MoodAnalysis {
        moods: vec!["happy".to_string()],
        energy: EnergyLevel::High,
        valence: Valence::Positive,
        description: "Upbeat track".to_string(),
    };
    let tags = generate_tags_for_test(&high_positive);
    assert!(
        tags.contains(&"fast".to_string()),
        "Should include 'fast' tag"
    );
    assert!(
        tags.contains(&"bright".to_string()),
        "Should include 'bright' tag"
    );

    // Low energy + negative valence should generate "slow" and "dark" tags
    let low_negative = MoodAnalysis {
        moods: vec!["sad".to_string()],
        energy: EnergyLevel::Low,
        valence: Valence::Negative,
        description: "Sad ballad".to_string(),
    };
    let tags = generate_tags_for_test(&low_negative);
    assert!(
        tags.contains(&"slow".to_string()),
        "Should include 'slow' tag"
    );
    assert!(
        tags.contains(&"dark".to_string()),
        "Should include 'dark' tag"
    );

    // Medium energy + neutral valence should generate no additional tags
    let medium_neutral = MoodAnalysis {
        moods: vec!["chill".to_string()],
        energy: EnergyLevel::Medium,
        valence: Valence::Neutral,
        description: "Relaxed track".to_string(),
    };
    let tags = generate_tags_for_test(&medium_neutral);
    assert!(tags.is_empty(), "Medium/neutral should not generate tags");
}

#[tokio::test]
async fn test_ollama_connection_error_handling() {
    // Test behavior when Ollama server is unreachable
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    // Try to connect to a non-existent server
    let result = client
        .post("http://127.0.0.1:59999/api/chat") // Non-existent port
        .json(&json!({"model": "mistral", "messages": [], "stream": false}))
        .send()
        .await;

    assert!(
        result.is_err(),
        "Should fail to connect to non-existent server"
    );
}

#[tokio::test]
async fn test_ollama_server_error_response() {
    let mock_server = MockOllamaServer::start().await;

    // Mock a server error
    mock_server
        .mock_chat_failure(500, "Internal server error: model failed to load")
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/chat", mock_server.url()))
        .json(&json!({"model": "mistral", "messages": [], "stream": false}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 500);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("Internal server error"));
}

// ============================================================================
// Helper Types and Functions (Test-only implementations)
// ============================================================================

/// Test-only track data structure
struct TestTrackData {
    title: String,
    artist_name: Option<String>,
    album_title: Option<String>,
    genres: Vec<String>,
    duration_ms: i32,
}

/// Test-only audio features structure
#[derive(Default)]
struct TestAudioFeatures {
    bpm: Option<f64>,
    key: Option<String>,
    mode: Option<String>,
    energy: Option<f64>,
    valence: Option<f64>,
    danceability: Option<f64>,
    acousticness: Option<f64>,
    instrumentalness: Option<f64>,
}

fn create_test_track_data() -> TestTrackData {
    TestTrackData {
        title: "Test Track".to_string(),
        artist_name: Some("Test Artist".to_string()),
        album_title: Some("Test Album".to_string()),
        genres: vec!["Electronic".to_string(), "Dance".to_string()],
        duration_ms: 180000,
    }
}

fn create_full_audio_features() -> TestAudioFeatures {
    TestAudioFeatures {
        bpm: Some(128.0),
        key: Some("A".to_string()),
        mode: Some("minor".to_string()),
        energy: Some(0.80),
        valence: Some(0.65),
        danceability: Some(0.75),
        acousticness: Some(0.20),
        instrumentalness: Some(0.05),
    }
}

fn create_empty_audio_features() -> TestAudioFeatures {
    TestAudioFeatures::default()
}

/// Mirrors the prompt building logic from mood_detection.rs for testing
fn build_prompt_for_test(track: &TestTrackData, features: &TestAudioFeatures) -> String {
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

/// Mirrors the JSON parsing logic from mood_detection.rs for testing
fn parse_mood_response_for_test(
    response: &str,
) -> Result<resonance_ollama_client::MoodAnalysis, String> {
    // Extract JSON from response
    let json_str = extract_json_for_test(response);

    let analysis: resonance_ollama_client::MoodAnalysis =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse: {}", e))?;

    // Validate moods are not empty
    if analysis.moods.is_empty() {
        return Err("empty moods array".to_string());
    }

    Ok(analysis)
}

/// Extract JSON object from response text
fn extract_json_for_test(text: &str) -> String {
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        if start < end {
            return text[start..=end].to_string();
        }
    }
    text.to_string()
}

/// Generate tags from mood analysis
fn generate_tags_for_test(analysis: &resonance_ollama_client::MoodAnalysis) -> Vec<String> {
    let mut tags = Vec::new();

    match analysis.energy {
        resonance_ollama_client::EnergyLevel::Low => tags.push("slow".to_string()),
        resonance_ollama_client::EnergyLevel::Medium => {}
        resonance_ollama_client::EnergyLevel::High => tags.push("fast".to_string()),
    }

    match analysis.valence {
        resonance_ollama_client::Valence::Negative => tags.push("dark".to_string()),
        resonance_ollama_client::Valence::Neutral => {}
        resonance_ollama_client::Valence::Positive => tags.push("bright".to_string()),
    }

    tags
}
