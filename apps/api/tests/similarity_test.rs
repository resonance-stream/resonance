//! Integration tests for the Similarity Service
//!
//! Tests track similarity recommendations using various methods:
//! - Vector embeddings (semantic similarity via pgvector)
//! - Audio features (acoustic similarity)
//! - Genre and mood matching (categorical similarity)
//! - Combined similarity (weighted blend)
//!
//! # Requirements
//!
//! These tests require a PostgreSQL database with pgvector extension to be running.
//! Set the `DATABASE_URL` environment variable or have a local database at
//! `postgres://resonance:resonance@localhost:5432/resonance_test`.
//!
//! To run the tests:
//! ```bash
//! # Start the test database (from project root)
//! docker compose up -d postgres
//!
//! # Run the tests
//! DATABASE_URL="postgres://resonance:resonance@localhost:5432/resonance" cargo test --test similarity_test -p resonance-api
//! ```
//!
//! If the database is not available, tests will be skipped automatically.

mod common;

use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use resonance_api::error::ApiError;
use resonance_api::services::similarity::{SimilarityService, SimilarityType};

// Import our comprehensive fixtures
#[allow(unused_imports)]
use common::similarity_fixtures::{
    generate_dissimilar_embedding, generate_test_embedding, AudioFeaturesFixture,
    SimilarityTestFixtures, TrackFixture, TrackFixtureBuilder, EMBEDDING_DIMENSION,
};

// ========== Test Constants ==========

/// Test JWT secret for authentication (must be at least 32 characters)
#[allow(dead_code)]
const TEST_JWT_SECRET: &str = "test-jwt-secret-for-integration-tests-minimum-32-chars";

// ========== Test Fixtures ==========

/// Create a test database pool connected to test database.
/// Returns None if the database is not available, allowing tests to be skipped.
async fn try_create_test_pool() -> Option<PgPool> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://resonance:resonance@localhost:5432/resonance_test".to_string()
    });

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .ok()
}

/// Macro to skip tests if the database is not available
macro_rules! require_db {
    ($pool_var:ident) => {
        let $pool_var = match try_create_test_pool().await {
            Some(p) => p,
            None => {
                eprintln!("Skipping test: database not available");
                return;
            }
        };
    };
}

/// Test data context containing created entities for cleanup
struct TestContext {
    pool: PgPool,
    artist_id: Uuid,
    album_id: Uuid,
    track_ids: Vec<Uuid>,
}

#[allow(dead_code)]
impl TestContext {
    /// Create test context with sample tracks for similarity testing
    async fn new(pool: PgPool) -> Self {
        let artist_id = Uuid::new_v4();
        let album_id = Uuid::new_v4();

        // Create a test artist
        sqlx::query(
            r#"
            INSERT INTO artists (id, name, genres)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(artist_id)
        .bind("Test Artist for Similarity")
        .bind(&["rock", "indie"] as &[&str])
        .execute(&pool)
        .await
        .expect("Failed to create test artist");

        // Create a test album
        sqlx::query(
            r#"
            INSERT INTO albums (id, title, artist_id, genres)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(album_id)
        .bind("Test Album for Similarity")
        .bind(artist_id)
        .bind(&["rock", "indie"] as &[&str])
        .execute(&pool)
        .await
        .expect("Failed to create test album");

        Self {
            pool,
            artist_id,
            album_id,
            track_ids: Vec::new(),
        }
    }

    /// Add a test track with specified attributes
    async fn add_track(
        &mut self,
        title: &str,
        genres: &[&str],
        moods: &[&str],
        tags: &[&str],
        audio_features: serde_json::Value,
    ) -> Uuid {
        let track_id = Uuid::new_v4();
        let file_path = format!("/test/similarity/{}.flac", track_id);

        sqlx::query(
            r#"
            INSERT INTO tracks (
                id, title, artist_id, album_id, file_path, file_size,
                file_format, duration_ms, genres, ai_mood, ai_tags, audio_features
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(track_id)
        .bind(title)
        .bind(self.artist_id)
        .bind(self.album_id)
        .bind(&file_path)
        .bind(1024000i64) // file_size
        .bind("flac") // file_format - using string to match audio_format enum
        .bind(180000i32) // duration_ms (3 minutes)
        .bind(genres)
        .bind(moods)
        .bind(tags)
        .bind(audio_features)
        .execute(&self.pool)
        .await
        .expect("Failed to create test track");

        self.track_ids.push(track_id);
        track_id
    }

    /// Add an embedding for a track using the standard 768-dimension vector
    async fn add_embedding(&self, track_id: Uuid, embedding: &[f32; EMBEDDING_DIMENSION]) {
        // Convert f32 array to pgvector format string
        let embedding_str = format!(
            "[{}]",
            embedding
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        sqlx::query(
            r#"
            INSERT INTO track_embeddings (track_id, description_embedding)
            VALUES ($1, $2::vector)
            ON CONFLICT (track_id) DO UPDATE SET description_embedding = $2::vector
            "#,
        )
        .bind(track_id)
        .bind(&embedding_str)
        .execute(&self.pool)
        .await
        .expect("Failed to create track embedding");
    }

    // ========== Helper methods for creating tracks from fixtures ==========

    /// Create a track from a TrackFixture
    ///
    /// This creates the track with all its attributes (genres, moods, tags, audio features)
    /// and optionally adds an embedding if the fixture has one configured.
    async fn create_track_from_fixture(&mut self, fixture: &TrackFixture) -> Uuid {
        let track_id = self
            .add_track(
                &fixture.title,
                &fixture.genres_as_refs(),
                &fixture.moods_as_refs(),
                &fixture.tags_as_refs(),
                fixture.audio_features.to_json(),
            )
            .await;

        // Add embedding if configured
        if let Some(embedding) = fixture.get_embedding() {
            self.add_embedding(track_id, &embedding).await;
        }

        track_id
    }

    /// Create a complete test track with embedding, audio features, and tags
    ///
    /// Convenience method that creates a track with standard characteristics.
    async fn create_complete_test_track(
        &mut self,
        title: &str,
        genres: &[&str],
        moods: &[&str],
        tags: &[&str],
        features: AudioFeaturesFixture,
        embedding_seed: u8,
    ) -> Uuid {
        let track_id = self
            .add_track(title, genres, moods, tags, features.to_json())
            .await;

        let embedding = generate_test_embedding(embedding_seed);
        self.add_embedding(track_id, &embedding).await;

        track_id
    }

    /// Create a track with only audio features (no embedding)
    async fn create_track_with_features(
        &mut self,
        title: &str,
        genres: &[&str],
        moods: &[&str],
        tags: &[&str],
        features: AudioFeaturesFixture,
    ) -> Uuid {
        self.add_track(title, genres, moods, tags, features.to_json())
            .await
    }

    /// Create a track with only embedding (default audio features)
    async fn create_track_with_embedding(
        &mut self,
        title: &str,
        genres: &[&str],
        moods: &[&str],
        tags: &[&str],
        embedding_seed: u8,
    ) -> Uuid {
        let track_id = self
            .add_track(
                title,
                genres,
                moods,
                tags,
                AudioFeaturesFixture::default().to_json(),
            )
            .await;

        let embedding = generate_test_embedding(embedding_seed);
        self.add_embedding(track_id, &embedding).await;

        track_id
    }

    /// Create a track with a dissimilar embedding (for testing negative cases)
    async fn create_track_with_dissimilar_embedding(
        &mut self,
        title: &str,
        genres: &[&str],
        moods: &[&str],
        tags: &[&str],
        embedding_seed: u8,
    ) -> Uuid {
        let track_id = self
            .add_track(
                title,
                genres,
                moods,
                tags,
                AudioFeaturesFixture::default().to_json(),
            )
            .await;

        let embedding = generate_dissimilar_embedding(embedding_seed);
        self.add_embedding(track_id, &embedding).await;

        track_id
    }

    /// Create a minimal track with only required fields (no features, no embedding)
    async fn create_minimal_track(&mut self, title: &str) -> Uuid {
        self.add_track(
            title,
            &["unknown"],
            &[],
            &[],
            AudioFeaturesFixture::empty().to_json(),
        )
        .await
    }

    /// Create all tracks from a SimilarityTestFixtures collection
    ///
    /// Returns a map of fixture title to track ID for easy lookup.
    async fn create_all_fixtures(&mut self, fixtures: &SimilarityTestFixtures) -> std::collections::HashMap<String, Uuid> {
        let mut ids = std::collections::HashMap::new();

        for fixture in fixtures.all() {
            let track_id = self.create_track_from_fixture(fixture).await;
            ids.insert(fixture.title.clone(), track_id);
        }

        ids
    }

    /// Create tracks for a specific cluster (rock, electronic, classical, jazz)
    async fn create_cluster(
        &mut self,
        fixtures: &[&TrackFixture],
    ) -> Vec<Uuid> {
        let mut ids = Vec::new();
        for fixture in fixtures {
            let track_id = self.create_track_from_fixture(fixture).await;
            ids.push(track_id);
        }
        ids
    }

    /// Clean up all test data
    async fn cleanup(&self) {
        // Delete in order respecting foreign keys
        for track_id in &self.track_ids {
            let _ = sqlx::query("DELETE FROM track_embeddings WHERE track_id = $1")
                .bind(track_id)
                .execute(&self.pool)
                .await;

            let _ = sqlx::query("DELETE FROM tracks WHERE id = $1")
                .bind(track_id)
                .execute(&self.pool)
                .await;
        }

        let _ = sqlx::query("DELETE FROM albums WHERE id = $1")
            .bind(self.album_id)
            .execute(&self.pool)
            .await;

        let _ = sqlx::query("DELETE FROM artists WHERE id = $1")
            .bind(self.artist_id)
            .execute(&self.pool)
            .await;
    }
}

/// Standard audio features for testing (convenience function using fixtures)
fn standard_audio_features() -> serde_json::Value {
    AudioFeaturesFixture::default().to_json()
}

// ========== Embedding Similarity Tests ==========

#[tokio::test]
async fn test_find_similar_by_embedding() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create source track with embedding
    let source_id = ctx
        .add_track(
            "Source Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    ctx.add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create similar track (similar embedding)
    let similar_id = ctx
        .add_track(
            "Similar Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    ctx.add_embedding(similar_id, &generate_test_embedding(2))
        .await;

    // Create different track (different embedding)
    let different_id = ctx
        .add_track(
            "Different Track",
            &["jazz"],
            &["mellow"],
            &["piano"],
            standard_audio_features(),
        )
        .await;
    ctx.add_embedding(different_id, &generate_test_embedding(100))
        .await;

    let service = SimilarityService::new(pool);
    let results = service.find_similar_by_embedding(source_id, 10).await;

    assert!(results.is_ok(), "Should find similar tracks by embedding");
    let tracks = results.unwrap();

    // Should return tracks (may or may not include similar_id depending on embedding similarity)
    // The key assertion is that it returns results without errors
    assert!(
        tracks.len() <= 10,
        "Should respect limit parameter"
    );

    // All results should have semantic similarity type
    for track in &tracks {
        assert_eq!(track.similarity_type, SimilarityType::Semantic);
        assert!(track.score >= 0.0 && track.score <= 1.0, "Score should be between 0 and 1");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_find_similar_by_embedding_no_embedding() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create track without embedding
    let track_id = ctx
        .add_track(
            "Track Without Embedding",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;

    let service = SimilarityService::new(pool);
    let result = service.find_similar_by_embedding(track_id, 10).await;

    assert!(result.is_err(), "Should return error when track has no embedding");

    if let Err(ApiError::NotFound { resource_type, .. }) = result {
        assert_eq!(resource_type, "track embedding");
    } else {
        panic!("Expected NotFound error for track embedding");
    }

    ctx.cleanup().await;
}

// ========== Audio Features Similarity Tests ==========

#[tokio::test]
async fn test_find_similar_by_features() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create source track with specific audio features
    let source_features = json!({
        "bpm": 120.0,
        "loudness": -8.0,
        "energy": 0.7,
        "danceability": 0.6,
        "valence": 0.5
    });
    let source_id = ctx
        .add_track(
            "Source Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            source_features,
        )
        .await;

    // Create similar track (similar audio features)
    let similar_features = json!({
        "bpm": 122.0,
        "loudness": -9.0,
        "energy": 0.72,
        "danceability": 0.58,
        "valence": 0.52
    });
    let _similar_id = ctx
        .add_track(
            "Similar Features Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            similar_features,
        )
        .await;

    // Create different track (very different audio features)
    let different_features = json!({
        "bpm": 60.0,
        "loudness": -20.0,
        "energy": 0.2,
        "danceability": 0.1,
        "valence": 0.9
    });
    let _different_id = ctx
        .add_track(
            "Different Features Track",
            &["classical"],
            &["calm"],
            &["piano"],
            different_features,
        )
        .await;

    let service = SimilarityService::new(pool);
    let results = service.find_similar_by_features(source_id, 10).await;

    assert!(results.is_ok(), "Should find similar tracks by features");
    let tracks = results.unwrap();

    // Should return tracks
    assert!(!tracks.is_empty(), "Should find at least one similar track");

    // All results should have acoustic similarity type
    for track in &tracks {
        assert_eq!(track.similarity_type, SimilarityType::Acoustic);
        assert!(track.score >= 0.0 && track.score <= 1.0, "Score should be between 0 and 1");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_find_similar_by_features_no_features() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create track with empty audio features
    let empty_features = json!({
        "bpm": null,
        "loudness": null,
        "energy": null
    });
    let track_id = ctx
        .add_track(
            "Track Without Features",
            &["rock"],
            &["energetic"],
            &["guitar"],
            empty_features,
        )
        .await;

    let service = SimilarityService::new(pool);
    let result = service.find_similar_by_features(track_id, 10).await;

    assert!(result.is_err(), "Should return error when track has no audio features");

    if let Err(ApiError::NotFound { resource_type, .. }) = result {
        assert_eq!(resource_type, "track audio features");
    } else {
        panic!("Expected NotFound error for track audio features");
    }

    ctx.cleanup().await;
}

// ========== Tag/Genre Similarity Tests ==========

#[tokio::test]
async fn test_find_similar_by_tags() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create source track with specific tags
    let source_id = ctx
        .add_track(
            "Source Track",
            &["rock", "indie", "alternative"],
            &["energetic", "upbeat"],
            &["guitar", "drums"],
            standard_audio_features(),
        )
        .await;

    // Create similar track (overlapping tags)
    let _similar_id = ctx
        .add_track(
            "Similar Tags Track",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;

    // Create different track (no overlapping tags)
    let _different_id = ctx
        .add_track(
            "Different Tags Track",
            &["classical", "orchestral"],
            &["calm", "peaceful"],
            &["violin", "piano"],
            standard_audio_features(),
        )
        .await;

    let service = SimilarityService::new(pool);
    let results = service.find_similar_by_tags(source_id, 10).await;

    assert!(results.is_ok(), "Should find similar tracks by tags");
    let tracks = results.unwrap();

    // Should return tracks with overlapping tags
    assert!(!tracks.is_empty(), "Should find tracks with overlapping tags");

    // All results should have categorical similarity type
    for track in &tracks {
        assert_eq!(track.similarity_type, SimilarityType::Categorical);
        assert!(track.score >= 0.0 && track.score <= 1.0, "Score should be between 0 and 1");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_find_similar_by_tags_no_matches() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create source track with unique tags
    let source_id = ctx
        .add_track(
            "Unique Track",
            &["uniquegenre1234"],
            &["uniquemood5678"],
            &["uniquetag9012"],
            standard_audio_features(),
        )
        .await;

    let service = SimilarityService::new(pool);
    let results = service.find_similar_by_tags(source_id, 10).await;

    assert!(results.is_ok(), "Should return Ok even with no matches");
    let tracks = results.unwrap();

    // May have no matches if unique tags don't overlap with anything
    assert!(tracks.len() <= 10, "Should respect limit");

    ctx.cleanup().await;
}

// ========== Combined Similarity Tests ==========

#[tokio::test]
async fn test_find_similar_combined() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create source track with all features
    let source_id = ctx
        .add_track(
            "Source Track",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    ctx.add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create track similar in all dimensions
    let similar_id = ctx
        .add_track(
            "Similar All Dimensions",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            json!({
                "bpm": 118.0,
                "loudness": -9.0,
                "energy": 0.68,
                "danceability": 0.58,
                "valence": 0.48
            }),
        )
        .await;
    ctx.add_embedding(similar_id, &generate_test_embedding(2))
        .await;

    // Create track similar only in tags
    let tags_only_id = ctx
        .add_track(
            "Similar Tags Only",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            json!({
                "bpm": 180.0,
                "loudness": -3.0,
                "energy": 0.2,
                "danceability": 0.1,
                "valence": 0.9
            }),
        )
        .await;
    ctx.add_embedding(tags_only_id, &generate_test_embedding(200))
        .await;

    let service = SimilarityService::new(pool);
    let results = service.find_similar_combined(source_id, 10).await;

    assert!(results.is_ok(), "Should find similar tracks using combined method");
    let tracks = results.unwrap();

    // Should return tracks
    assert!(!tracks.is_empty(), "Should find similar tracks");

    // All results should have combined similarity type
    for track in &tracks {
        assert_eq!(track.similarity_type, SimilarityType::Combined);
        assert!(track.score >= 0.0, "Score should be non-negative");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_find_similar_combined_partial_data() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create source track with only tags (no embedding, minimal features)
    let source_id = ctx
        .add_track(
            "Source Track Partial",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            json!({
                "bpm": null,
                "loudness": null,
                "energy": null
            }),
        )
        .await;

    // Create another track with tags
    let _other_id = ctx
        .add_track(
            "Other Track",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;

    let service = SimilarityService::new(pool);
    let results = service.find_similar_combined(source_id, 10).await;

    // Combined should still work even with partial data
    assert!(
        results.is_ok(),
        "Combined similarity should handle partial data gracefully"
    );

    ctx.cleanup().await;
}

// ========== Error Cases ==========

#[tokio::test]
async fn test_similarity_track_not_found() {
    require_db!(pool);

    let service = SimilarityService::new(pool);
    let nonexistent_id = Uuid::new_v4();

    // Test each similarity method with non-existent track
    let embedding_result = service.find_similar_by_embedding(nonexistent_id, 10).await;
    assert!(
        embedding_result.is_err(),
        "Should return error for non-existent track (embedding)"
    );

    let features_result = service.find_similar_by_features(nonexistent_id, 10).await;
    assert!(
        features_result.is_err(),
        "Should return error for non-existent track (features)"
    );

    // Tags query may return empty rather than error
    let tags_result = service.find_similar_by_tags(nonexistent_id, 10).await;
    assert!(
        tags_result.is_ok(),
        "Tags query should not error for non-existent track (returns empty)"
    );
    assert!(
        tags_result.unwrap().is_empty(),
        "Should return empty for non-existent track"
    );
}

#[tokio::test]
async fn test_similarity_empty_results() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create a single isolated track
    let track_id = ctx
        .add_track(
            "Isolated Track",
            &["veryuniquegene123"],
            &["veryuniquemood456"],
            &["veryuniquetag789"],
            json!({
                "bpm": null,
                "energy": null
            }),
        )
        .await;

    let service = SimilarityService::new(pool);

    // Combined should return empty or error gracefully
    let results = service.find_similar_combined(track_id, 10).await;
    assert!(
        results.is_ok(),
        "Combined should handle case with no similar tracks"
    );
    // Result may be empty if no similar tracks found

    ctx.cleanup().await;
}

// ========== Limit Parameter Tests ==========

#[tokio::test]
async fn test_similarity_limit_clamping() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create source track
    let source_id = ctx
        .add_track(
            "Source Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    ctx.add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create multiple similar tracks
    for i in 0..5 {
        let track_id = ctx
            .add_track(
                &format!("Similar Track {}", i),
                &["rock"],
                &["energetic"],
                &["guitar"],
                standard_audio_features(),
            )
            .await;
        ctx.add_embedding(track_id, &generate_test_embedding((i + 2) as u8))
            .await;
    }

    let service = SimilarityService::new(pool);

    // Test with limit of 2
    let results = service.find_similar_by_embedding(source_id, 2).await;
    assert!(results.is_ok());
    assert!(
        results.unwrap().len() <= 2,
        "Should respect small limit"
    );

    // Test with excessive limit (should be clamped to MAX_SIMILARITY_RESULTS)
    let results = service.find_similar_by_embedding(source_id, 1000).await;
    assert!(results.is_ok());
    assert!(
        results.unwrap().len() <= 100,
        "Should clamp excessive limit to max"
    );

    // Test with zero limit (should be clamped to 1)
    let results = service.find_similar_by_embedding(source_id, 0).await;
    assert!(results.is_ok());
    // Clamped to 1, so should return at most 1 result
    assert!(
        results.unwrap().len() <= 1,
        "Zero limit should be clamped to 1"
    );

    ctx.cleanup().await;
}

// ========== Score Validation Tests ==========

#[tokio::test]
async fn test_similarity_scores_are_valid() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create source track
    let source_id = ctx
        .add_track(
            "Source Track",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    ctx.add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create several tracks for similarity matching
    for i in 0..3 {
        let track_id = ctx
            .add_track(
                &format!("Track {}", i),
                &["rock"],
                &["energetic"],
                &["guitar"],
                standard_audio_features(),
            )
            .await;
        ctx.add_embedding(track_id, &generate_test_embedding((i + 5) as u8))
            .await;
    }

    let service = SimilarityService::new(pool);

    // Check embedding similarity scores
    if let Ok(tracks) = service.find_similar_by_embedding(source_id, 10).await {
        for track in tracks {
            assert!(
                track.score >= 0.0 && track.score <= 1.0,
                "Embedding score {} should be in [0, 1] range",
                track.score
            );
        }
    }

    // Check feature similarity scores
    if let Ok(tracks) = service.find_similar_by_features(source_id, 10).await {
        for track in tracks {
            assert!(
                track.score >= 0.0 && track.score <= 1.0,
                "Feature score {} should be in [0, 1] range",
                track.score
            );
        }
    }

    // Check tag similarity scores
    if let Ok(tracks) = service.find_similar_by_tags(source_id, 10).await {
        for track in tracks {
            assert!(
                track.score >= 0.0 && track.score <= 1.0,
                "Tag score {} should be in [0, 1] range",
                track.score
            );
        }
    }

    ctx.cleanup().await;
}

// ========== Response Structure Tests ==========

#[tokio::test]
async fn test_similar_track_response_structure() {
    require_db!(pool);

    let mut ctx = TestContext::new(pool.clone()).await;

    // Create source track
    let source_id = ctx
        .add_track(
            "Source Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    ctx.add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create a similar track
    let similar_id = ctx
        .add_track(
            "Similar Track With Title",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    ctx.add_embedding(similar_id, &generate_test_embedding(2))
        .await;

    let service = SimilarityService::new(pool);
    let results = service.find_similar_by_embedding(source_id, 10).await;

    assert!(results.is_ok());
    let tracks = results.unwrap();

    if !tracks.is_empty() {
        let track = &tracks[0];

        // Verify response structure
        assert!(!track.track_id.is_nil(), "Track ID should be valid UUID");
        assert!(!track.title.is_empty(), "Title should not be empty");
        // artist_name and album_title are optional
        assert!(track.score >= 0.0, "Score should be non-negative");
        assert_eq!(
            track.similarity_type,
            SimilarityType::Semantic,
            "Should have correct similarity type"
        );
    }

    ctx.cleanup().await;
}

// ==========================================================================
// GraphQL Integration Tests
// ==========================================================================
//
// These tests exercise the GraphQL API layer for similarity queries,
// testing the `similarTracks` and `similarTracksByMethod` queries.

use async_graphql::{EmptyMutation, EmptySubscription, Schema};

/// GraphQL test context that wraps the TestContext and provides schema execution
struct GraphQLTestContext {
    ctx: TestContext,
    schema: Schema<resonance_api::graphql::query::Query, EmptyMutation, EmptySubscription>,
}

impl GraphQLTestContext {
    /// Create a new GraphQL test context
    async fn new(pool: PgPool) -> Self {
        let ctx = TestContext::new(pool.clone()).await;

        // Create a minimal schema with only the SimilarityService for testing
        let similarity_service = resonance_api::services::similarity::SimilarityService::new(pool.clone());
        let track_repo = resonance_api::repositories::TrackRepository::new(pool.clone());

        let schema = Schema::build(
            resonance_api::graphql::query::Query::default(),
            EmptyMutation,
            EmptySubscription,
        )
        .data(pool)
        .data(similarity_service)
        .data(track_repo)
        .finish();

        Self { ctx, schema }
    }

    /// Execute a GraphQL query and return the result
    async fn execute(&self, query: &str) -> async_graphql::Response {
        self.schema.execute(query).await
    }

    /// Execute a GraphQL query with variables
    async fn execute_with_variables(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> async_graphql::Response {
        let request = async_graphql::Request::new(query).variables(
            async_graphql::Variables::from_json(variables),
        );
        self.schema.execute(request).await
    }

    /// Cleanup test data
    async fn cleanup(&self) {
        self.ctx.cleanup().await;
    }
}

// ========== similarTracks Query Tests ==========

#[tokio::test]
async fn test_graphql_similar_tracks_query() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create source track with all features
    let source_id = gql_ctx
        .ctx
        .add_track(
            "GraphQL Source Track",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create similar tracks
    for i in 0..3 {
        let track_id = gql_ctx
            .ctx
            .add_track(
                &format!("Similar Track {}", i),
                &["rock", "indie"],
                &["energetic"],
                &["guitar"],
                json!({
                    "bpm": 118.0 + i as f64,
                    "loudness": -9.0,
                    "energy": 0.68,
                    "danceability": 0.58,
                    "valence": 0.48
                }),
            )
            .await;
        gql_ctx
            .ctx
            .add_embedding(track_id, &generate_test_embedding((i + 2) as u8))
            .await;
    }

    // Execute GraphQL query
    let query = format!(
        r#"
        query {{
            similarTracks(trackId: "{}", limit: 10) {{
                trackId
                title
                score
            }}
        }}
        "#,
        source_id
    );

    let response = gql_ctx.execute(&query).await;

    // Check for no errors
    assert!(
        response.errors.is_empty(),
        "GraphQL query should not have errors: {:?}",
        response.errors
    );

    // Parse response data
    let data = response.data.into_json().unwrap();
    let similar_tracks = data["similarTracks"].as_array().unwrap();

    // Should return similar tracks
    assert!(
        !similar_tracks.is_empty(),
        "Should find similar tracks via GraphQL"
    );

    // Verify response structure
    for track in similar_tracks {
        assert!(track["trackId"].is_string(), "trackId should be a string");
        assert!(track["title"].is_string(), "title should be a string");
        assert!(track["score"].is_number(), "score should be a number");

        let score = track["score"].as_f64().unwrap();
        assert!(score >= 0.0 && score <= 1.0, "Score should be in [0, 1]");
    }

    gql_ctx.cleanup().await;
}

#[tokio::test]
async fn test_graphql_similar_tracks_with_limit() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create source track
    let source_id = gql_ctx
        .ctx
        .add_track(
            "Source Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create 5 similar tracks
    for i in 0..5 {
        let track_id = gql_ctx
            .ctx
            .add_track(
                &format!("Similar Track {}", i),
                &["rock"],
                &["energetic"],
                &["guitar"],
                standard_audio_features(),
            )
            .await;
        gql_ctx
            .ctx
            .add_embedding(track_id, &generate_test_embedding((i + 2) as u8))
            .await;
    }

    // Query with limit of 2
    let query = format!(
        r#"
        query {{
            similarTracks(trackId: "{}", limit: 2) {{
                trackId
            }}
        }}
        "#,
        source_id
    );

    let response = gql_ctx.execute(&query).await;
    assert!(response.errors.is_empty(), "Should not have errors");

    let data = response.data.into_json().unwrap();
    let similar_tracks = data["similarTracks"].as_array().unwrap();

    assert!(
        similar_tracks.len() <= 2,
        "Should respect limit parameter, got {}",
        similar_tracks.len()
    );

    gql_ctx.cleanup().await;
}

// ========== similarTracksByMethod Query Tests ==========

#[tokio::test]
async fn test_graphql_similar_tracks_by_method_semantic() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create source track with embedding
    let source_id = gql_ctx
        .ctx
        .add_track(
            "Semantic Source",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create similar track with similar embedding
    let similar_id = gql_ctx
        .ctx
        .add_track(
            "Semantic Similar",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(similar_id, &generate_test_embedding(2))
        .await;

    let query = format!(
        r#"
        query {{
            similarTracksByMethod(trackId: "{}", method: SEMANTIC, limit: 10) {{
                trackId
                title
                score
                similarityType
            }}
        }}
        "#,
        source_id
    );

    let response = gql_ctx.execute(&query).await;
    assert!(
        response.errors.is_empty(),
        "SEMANTIC query should not have errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let tracks = data["similarTracksByMethod"].as_array().unwrap();

    // Should return tracks
    assert!(!tracks.is_empty(), "Should find semantically similar tracks");

    // All tracks should have SEMANTIC similarity type
    for track in tracks {
        assert_eq!(
            track["similarityType"].as_str().unwrap(),
            "SEMANTIC",
            "All tracks should have SEMANTIC similarity type"
        );
    }

    gql_ctx.cleanup().await;
}

#[tokio::test]
async fn test_graphql_similar_tracks_by_method_acoustic() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create source track with specific audio features
    let source_features = json!({
        "bpm": 120.0,
        "loudness": -8.0,
        "energy": 0.7,
        "danceability": 0.6,
        "valence": 0.5
    });
    let source_id = gql_ctx
        .ctx
        .add_track(
            "Acoustic Source",
            &["rock"],
            &["energetic"],
            &["guitar"],
            source_features,
        )
        .await;

    // Create track with similar features
    let similar_features = json!({
        "bpm": 122.0,
        "loudness": -9.0,
        "energy": 0.72,
        "danceability": 0.58,
        "valence": 0.52
    });
    let _similar_id = gql_ctx
        .ctx
        .add_track(
            "Acoustic Similar",
            &["rock"],
            &["energetic"],
            &["guitar"],
            similar_features,
        )
        .await;

    let query = format!(
        r#"
        query {{
            similarTracksByMethod(trackId: "{}", method: ACOUSTIC, limit: 10) {{
                trackId
                title
                score
                similarityType
            }}
        }}
        "#,
        source_id
    );

    let response = gql_ctx.execute(&query).await;
    assert!(
        response.errors.is_empty(),
        "ACOUSTIC query should not have errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let tracks = data["similarTracksByMethod"].as_array().unwrap();

    // Should return tracks
    assert!(
        !tracks.is_empty(),
        "Should find acoustically similar tracks"
    );

    // All tracks should have ACOUSTIC similarity type
    for track in tracks {
        assert_eq!(
            track["similarityType"].as_str().unwrap(),
            "ACOUSTIC",
            "All tracks should have ACOUSTIC similarity type"
        );
        let score = track["score"].as_f64().unwrap();
        assert!(score >= 0.0 && score <= 1.0, "Score should be in [0, 1]");
    }

    gql_ctx.cleanup().await;
}

#[tokio::test]
async fn test_graphql_similar_tracks_by_method_categorical() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create source track with specific tags
    let source_id = gql_ctx
        .ctx
        .add_track(
            "Categorical Source",
            &["rock", "indie", "alternative"],
            &["energetic", "upbeat"],
            &["guitar", "drums"],
            standard_audio_features(),
        )
        .await;

    // Create track with overlapping tags
    let _similar_id = gql_ctx
        .ctx
        .add_track(
            "Categorical Similar",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;

    let query = format!(
        r#"
        query {{
            similarTracksByMethod(trackId: "{}", method: CATEGORICAL, limit: 10) {{
                trackId
                title
                score
                similarityType
            }}
        }}
        "#,
        source_id
    );

    let response = gql_ctx.execute(&query).await;
    assert!(
        response.errors.is_empty(),
        "CATEGORICAL query should not have errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let tracks = data["similarTracksByMethod"].as_array().unwrap();

    // Should return tracks
    assert!(
        !tracks.is_empty(),
        "Should find categorically similar tracks"
    );

    // All tracks should have CATEGORICAL similarity type
    for track in tracks {
        assert_eq!(
            track["similarityType"].as_str().unwrap(),
            "CATEGORICAL",
            "All tracks should have CATEGORICAL similarity type"
        );
    }

    gql_ctx.cleanup().await;
}

#[tokio::test]
async fn test_graphql_similar_tracks_by_method_combined() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create source track with all features
    let source_id = gql_ctx
        .ctx
        .add_track(
            "Combined Source",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create track similar in all dimensions
    let similar_id = gql_ctx
        .ctx
        .add_track(
            "Combined Similar",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            json!({
                "bpm": 118.0,
                "loudness": -9.0,
                "energy": 0.68,
                "danceability": 0.58,
                "valence": 0.48
            }),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(similar_id, &generate_test_embedding(2))
        .await;

    let query = format!(
        r#"
        query {{
            similarTracksByMethod(trackId: "{}", method: COMBINED, limit: 10) {{
                trackId
                title
                score
                similarityType
            }}
        }}
        "#,
        source_id
    );

    let response = gql_ctx.execute(&query).await;
    assert!(
        response.errors.is_empty(),
        "COMBINED query should not have errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let tracks = data["similarTracksByMethod"].as_array().unwrap();

    // Should return tracks
    assert!(!tracks.is_empty(), "Should find combined similar tracks");

    // All tracks should have COMBINED similarity type
    for track in tracks {
        assert_eq!(
            track["similarityType"].as_str().unwrap(),
            "COMBINED",
            "All tracks should have COMBINED similarity type"
        );
    }

    gql_ctx.cleanup().await;
}

// ========== GraphQL Error Cases ==========

#[tokio::test]
async fn test_graphql_similar_tracks_invalid_track_id() {
    require_db!(pool);

    let gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Query with invalid UUID format
    let query = r#"
        query {
            similarTracks(trackId: "invalid-uuid", limit: 10) {
                trackId
            }
        }
    "#;

    let response = gql_ctx.execute(query).await;

    // Should have an error
    assert!(
        !response.errors.is_empty(),
        "Should have error for invalid track ID"
    );

    // Check error message contains relevant information
    let error_msg = response.errors[0].message.to_lowercase();
    assert!(
        error_msg.contains("invalid") || error_msg.contains("track"),
        "Error should mention invalid track ID: {}",
        response.errors[0].message
    );

    gql_ctx.cleanup().await;
}

#[tokio::test]
async fn test_graphql_similar_tracks_nonexistent_track() {
    require_db!(pool);

    let gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    let nonexistent_id = Uuid::new_v4();

    let query = format!(
        r#"
        query {{
            similarTracks(trackId: "{}", limit: 10) {{
                trackId
            }}
        }}
        "#,
        nonexistent_id
    );

    let response = gql_ctx.execute(&query).await;

    // Should either have an error or return empty results (both are valid behaviors)
    if response.errors.is_empty() {
        let data = response.data.into_json().unwrap();
        let tracks = data["similarTracks"].as_array().unwrap();
        assert!(
            tracks.is_empty(),
            "Should return empty for nonexistent track"
        );
    } else {
        // Error is also acceptable
        assert!(!response.errors.is_empty());
    }

    gql_ctx.cleanup().await;
}

#[tokio::test]
async fn test_graphql_similar_tracks_by_method_no_embedding() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create track WITHOUT embedding
    let track_id = gql_ctx
        .ctx
        .add_track(
            "Track Without Embedding",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;

    // Query SEMANTIC method which requires embedding
    let query = format!(
        r#"
        query {{
            similarTracksByMethod(trackId: "{}", method: SEMANTIC, limit: 10) {{
                trackId
            }}
        }}
        "#,
        track_id
    );

    let response = gql_ctx.execute(&query).await;

    // Should have an error because track has no embedding
    assert!(
        !response.errors.is_empty(),
        "Should error when track has no embedding for SEMANTIC method"
    );

    gql_ctx.cleanup().await;
}

#[tokio::test]
async fn test_graphql_similar_tracks_by_method_no_features() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create track with empty audio features
    let empty_features = json!({
        "bpm": null,
        "loudness": null,
        "energy": null
    });
    let track_id = gql_ctx
        .ctx
        .add_track(
            "Track Without Features",
            &["rock"],
            &["energetic"],
            &["guitar"],
            empty_features,
        )
        .await;

    // Query ACOUSTIC method which requires audio features
    let query = format!(
        r#"
        query {{
            similarTracksByMethod(trackId: "{}", method: ACOUSTIC, limit: 10) {{
                trackId
            }}
        }}
        "#,
        track_id
    );

    let response = gql_ctx.execute(&query).await;

    // Should have an error because track has no audio features
    assert!(
        !response.errors.is_empty(),
        "Should error when track has no audio features for ACOUSTIC method"
    );

    gql_ctx.cleanup().await;
}

// ========== GraphQL Variables Tests ==========

#[tokio::test]
async fn test_graphql_similar_tracks_with_variables() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create source track
    let source_id = gql_ctx
        .ctx
        .add_track(
            "Variable Test Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create similar track
    let similar_id = gql_ctx
        .ctx
        .add_track(
            "Similar Variable Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(similar_id, &generate_test_embedding(2))
        .await;

    let query = r#"
        query SimilarTracks($trackId: ID!, $limit: Int!) {
            similarTracks(trackId: $trackId, limit: $limit) {
                trackId
                title
                score
            }
        }
    "#;

    let variables = json!({
        "trackId": source_id.to_string(),
        "limit": 5
    });

    let response = gql_ctx.execute_with_variables(query, variables).await;

    assert!(
        response.errors.is_empty(),
        "Query with variables should not have errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let tracks = data["similarTracks"].as_array().unwrap();

    assert!(
        tracks.len() <= 5,
        "Should respect limit variable"
    );

    gql_ctx.cleanup().await;
}

#[tokio::test]
async fn test_graphql_similar_tracks_by_method_with_variables() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create source track
    let source_id = gql_ctx
        .ctx
        .add_track(
            "Method Variable Track",
            &["rock", "indie"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create similar tracks
    for i in 0..2 {
        let track_id = gql_ctx
            .ctx
            .add_track(
                &format!("Similar Method Variable {}", i),
                &["rock", "indie"],
                &["energetic"],
                &["guitar"],
                standard_audio_features(),
            )
            .await;
        gql_ctx
            .ctx
            .add_embedding(track_id, &generate_test_embedding((i + 2) as u8))
            .await;
    }

    let query = r#"
        query SimilarByMethod($trackId: ID!, $method: SimilarityMethod!, $limit: Int!) {
            similarTracksByMethod(trackId: $trackId, method: $method, limit: $limit) {
                trackId
                title
                score
                similarityType
            }
        }
    "#;

    let variables = json!({
        "trackId": source_id.to_string(),
        "method": "COMBINED",
        "limit": 3
    });

    let response = gql_ctx.execute_with_variables(query, variables).await;

    assert!(
        response.errors.is_empty(),
        "Query with method variable should not have errors: {:?}",
        response.errors
    );

    let data = response.data.into_json().unwrap();
    let tracks = data["similarTracksByMethod"].as_array().unwrap();

    for track in tracks {
        assert_eq!(
            track["similarityType"].as_str().unwrap(),
            "COMBINED",
            "Method variable should be respected"
        );
    }

    gql_ctx.cleanup().await;
}

// ========== GraphQL Default Limit Tests ==========

#[tokio::test]
async fn test_graphql_similar_tracks_default_limit() {
    require_db!(pool);

    let mut gql_ctx = GraphQLTestContext::new(pool.clone()).await;

    // Create source track
    let source_id = gql_ctx
        .ctx
        .add_track(
            "Default Limit Track",
            &["rock"],
            &["energetic"],
            &["guitar"],
            standard_audio_features(),
        )
        .await;
    gql_ctx
        .ctx
        .add_embedding(source_id, &generate_test_embedding(1))
        .await;

    // Create multiple similar tracks
    for i in 0..15 {
        let track_id = gql_ctx
            .ctx
            .add_track(
                &format!("Similar Default {}", i),
                &["rock"],
                &["energetic"],
                &["guitar"],
                standard_audio_features(),
            )
            .await;
        gql_ctx
            .ctx
            .add_embedding(track_id, &generate_test_embedding((i + 2) as u8))
            .await;
    }

    // Query WITHOUT specifying limit (should use default of 10)
    let query = format!(
        r#"
        query {{
            similarTracks(trackId: "{}") {{
                trackId
            }}
        }}
        "#,
        source_id
    );

    let response = gql_ctx.execute(&query).await;
    assert!(response.errors.is_empty(), "Should not have errors");

    let data = response.data.into_json().unwrap();
    let tracks = data["similarTracks"].as_array().unwrap();

    // Default limit is 10
    assert!(
        tracks.len() <= 10,
        "Should respect default limit of 10, got {}",
        tracks.len()
    );

    gql_ctx.cleanup().await;
}
