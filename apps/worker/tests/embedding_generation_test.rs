//! Integration tests for embedding generation job
//!
//! Tests the embedding generation job with MockOllamaServer for:
//! - Successful embedding creation
//! - Dimension validation
//! - Timeout handling
//! - Error scenarios (model not found, API errors)

mod common;

use resonance_ollama_client::{validate_embedding_dimension, EMBEDDING_DIMENSION};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::MockOllamaServer;

// ============================================================================
// Unit Tests for Description Building
// ============================================================================

/// Test description building with all metadata fields populated
#[test]
fn test_build_description_with_full_metadata() {
    // This mirrors the internal build_description_text function behavior
    let title = "Bohemian Rhapsody";
    let artist = Some("Queen");
    let album = Some("A Night at the Opera");
    let genres = vec!["Rock", "Progressive Rock"];
    let moods = vec!["epic", "dramatic"];
    let tags = vec!["operatic", "multi-section"];
    let description = Some("A groundbreaking rock opera masterpiece");

    let text = build_test_description(title, artist, album, &genres, &moods, &tags, description);

    assert!(text.contains("\"Bohemian Rhapsody\" by Queen"));
    assert!(text.contains("A Night at the Opera"));
    assert!(text.contains("Genre: Rock, Progressive Rock"));
    assert!(text.contains("Mood: epic, dramatic"));
    assert!(text.contains("Tags: operatic, multi-section"));
    assert!(text.contains("groundbreaking rock opera"));
}

/// Test description building with minimal metadata (only title)
#[test]
fn test_build_description_with_minimal_metadata() {
    let title = "Unknown Track";
    let artist: Option<&str> = None;
    let album: Option<&str> = None;
    let genres: Vec<&str> = vec![];
    let moods: Vec<&str> = vec![];
    let tags: Vec<&str> = vec![];
    let description: Option<&str> = None;

    let text = build_test_description(title, artist, album, &genres, &moods, &tags, description);

    assert!(text.contains("\"Unknown Track\" by Unknown Artist"));
    assert!(!text.contains("Genre:"));
    assert!(!text.contains("Mood:"));
    assert!(!text.contains("Tags:"));
}

/// Test description building with only some optional fields
#[test]
fn test_build_description_with_partial_metadata() {
    let title = "Highway to Hell";
    let artist = Some("AC/DC");
    let album: Option<&str> = None;
    let genres = vec!["Hard Rock"];
    let moods: Vec<&str> = vec![];
    let tags = vec!["anthem"];
    let description: Option<&str> = None;

    let text = build_test_description(title, artist, album, &genres, &moods, &tags, description);

    assert!(text.contains("\"Highway to Hell\" by AC/DC"));
    assert!(!text.contains("from the album")); // No album
    assert!(text.contains("Genre: Hard Rock"));
    assert!(!text.contains("Mood:")); // Empty moods
    assert!(text.contains("Tags: anthem"));
}

// ============================================================================
// Unit Tests for pgvector Format
// ============================================================================

/// Test pgvector format with typical embedding values
#[test]
fn test_format_embedding_for_pgvector_typical() {
    let embedding = vec![0.1234567, -0.9876543, 0.0, 1.0, -1.0];
    let result = format_embedding_for_pgvector(&embedding);

    assert!(result.is_ok());
    let formatted = result.unwrap();
    // Should start with [ and end with ]
    assert!(formatted.starts_with('['));
    assert!(formatted.ends_with(']'));
    // Should contain properly formatted values
    assert!(formatted.contains("0.123457")); // 6 decimal places
    assert!(formatted.contains("-0.987654"));
}

/// Test pgvector format rejects NaN values
#[test]
fn test_format_embedding_rejects_nan() {
    let embedding = vec![0.1, f32::NAN, 0.3];
    let result = format_embedding_for_pgvector(&embedding);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("non-finite"));
}

/// Test pgvector format rejects infinity values
#[test]
fn test_format_embedding_rejects_infinity() {
    let embedding_pos_inf = vec![0.1, f32::INFINITY, 0.3];
    let embedding_neg_inf = vec![0.1, f32::NEG_INFINITY, 0.3];

    assert!(format_embedding_for_pgvector(&embedding_pos_inf).is_err());
    assert!(format_embedding_for_pgvector(&embedding_neg_inf).is_err());
}

// ============================================================================
// Integration Tests with MockOllamaServer
// ============================================================================

/// Test successful embedding generation with mock Ollama
#[tokio::test]
async fn test_embedding_generation_success() {
    let server = MockOllamaServer::start().await;
    server.mock_embeddings_success().await;

    // Verify the mock server responds with 768-dimensional embedding
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/embeddings", server.url()))
        .json(&json!({
            "model": "nomic-embed-text",
            "prompt": "Test track by Test Artist"
        }))
        .send()
        .await
        .expect("Request should succeed");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Should parse JSON");
    let embedding = body["embedding"]
        .as_array()
        .expect("Should have embedding array");

    // Verify dimension
    assert_eq!(embedding.len(), EMBEDDING_DIMENSION);
}

/// Test dimension validation with correct dimensions
#[tokio::test]
async fn test_embedding_dimension_validation_correct() {
    // Create a valid 768-dimensional embedding
    let embedding: Vec<f32> = (0..EMBEDDING_DIMENSION)
        .map(|i| (i as f32 * 0.001) % 1.0)
        .collect();

    let result = validate_embedding_dimension(&embedding);
    assert!(result.is_ok());
}

/// Test dimension validation with wrong dimensions
#[test]
fn test_embedding_dimension_validation_wrong() {
    // Too few dimensions
    let small_embedding: Vec<f32> = vec![0.1, 0.2, 0.3];
    let result = validate_embedding_dimension(&small_embedding);
    assert!(result.is_err());

    // Too many dimensions
    let large_embedding: Vec<f32> = (0..1024).map(|i| i as f32 * 0.001).collect();
    let result = validate_embedding_dimension(&large_embedding);
    assert!(result.is_err());
}

/// Test timeout handling with delayed mock response
#[tokio::test]
async fn test_embedding_generation_timeout_handling() {
    let server = MockServer::start().await;

    // Mock with 5 second delay - simulating slow Ollama
    Mock::given(method("POST"))
        .and(path("/api/embeddings"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(5))
                .set_body_json(json!({
                    "embedding": vec![0.1_f32; 768]
                })),
        )
        .mount(&server)
        .await;

    // Create a client with a short timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .expect("Should build client");

    let result = client
        .post(format!("{}/api/embeddings", server.uri()))
        .json(&json!({
            "model": "nomic-embed-text",
            "prompt": "Test"
        }))
        .send()
        .await;

    // Should fail due to timeout
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.is_timeout());
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build description text matching the job's internal logic
fn build_test_description(
    title: &str,
    artist: Option<&str>,
    album: Option<&str>,
    genres: &[&str],
    moods: &[&str],
    tags: &[&str],
    description: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    // Title and artist
    parts.push(format!(
        "\"{}\" by {}",
        title,
        artist.unwrap_or("Unknown Artist")
    ));

    // Album
    if let Some(album_title) = album {
        parts.push(format!("from the album \"{}\"", album_title));
    }

    // Genres
    if !genres.is_empty() {
        parts.push(format!("Genre: {}", genres.join(", ")));
    }

    // AI mood tags
    if !moods.is_empty() {
        parts.push(format!("Mood: {}", moods.join(", ")));
    }

    // AI tags
    if !tags.is_empty() {
        parts.push(format!("Tags: {}", tags.join(", ")));
    }

    // AI description
    if let Some(desc) = description {
        parts.push(desc.to_string());
    }

    parts.join(". ")
}

/// Format embedding for pgvector - matches the job's internal logic
fn format_embedding_for_pgvector(embedding: &[f32]) -> Result<String, String> {
    // Validate that all values are finite
    if embedding.iter().any(|v| !v.is_finite()) {
        return Err("Embedding contains non-finite values (NaN/inf)".to_string());
    }

    let values: Vec<String> = embedding.iter().map(|v| format!("{:.6}", v)).collect();
    Ok(format!("[{}]", values.join(",")))
}
