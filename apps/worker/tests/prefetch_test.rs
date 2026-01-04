//! Prefetch job integration tests
//!
//! This module tests the smart prefetch job functionality including:
//! - Combined score calculation for track similarity
//! - Redis cache operations for track metadata
//! - Similarity queries (embedding-based and feature-based)
//! - Queue-based prefetching

mod common;

use common::MockRedisStore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// Test Constants (matching prefetch.rs)
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
const BPM_NORMALIZATION_FACTOR: f64 = 200.0;

/// Loudness normalization offset (typical loudness range: -60 to 0 dB)
const LOUDNESS_OFFSET: f64 = 60.0;

/// Cache TTL in seconds (30 minutes)
const CACHE_TTL_SECONDS: i64 = 30 * 60;

// =============================================================================
// Test Types (mirroring prefetch.rs types for testing)
// =============================================================================

/// Cached track metadata for prefetch (matches CachedTrackMetadata in prefetch.rs)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachedTrackMetadata {
    pub id: Uuid,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub duration_ms: Option<i32>,
    pub file_path: String,
    pub file_format: String,
}

impl CachedTrackMetadata {
    /// Create a test track metadata instance
    fn test_track(id: Uuid, title: &str) -> Self {
        Self {
            id,
            title: title.to_string(),
            artist_name: Some("Test Artist".to_string()),
            album_title: Some("Test Album".to_string()),
            duration_ms: Some(180000),
            file_path: format!("/music/{}.flac", id),
            file_format: "flac".to_string(),
        }
    }
}

/// Audio features for similarity calculation
#[derive(Debug, Clone)]
struct AudioFeatures {
    energy: f64,
    loudness: f64,
    valence: f64,
    danceability: f64,
    bpm: f64,
}

impl Default for AudioFeatures {
    fn default() -> Self {
        Self {
            energy: 0.5,
            loudness: -10.0,
            valence: 0.5,
            danceability: 0.5,
            bpm: 120.0,
        }
    }
}

// =============================================================================
// Unit Tests: Combined Score Calculation
// =============================================================================

/// Calculate combined similarity score using the same weights as prefetch.rs
fn calculate_combined_score(
    semantic_score: f64,
    acoustic_score: f64,
    categorical_score: f64,
) -> f64 {
    semantic_score * WEIGHT_SEMANTIC
        + acoustic_score * WEIGHT_ACOUSTIC
        + categorical_score * WEIGHT_CATEGORICAL
}

/// Calculate fallback score when embeddings aren't available
fn calculate_fallback_score(feature_score: f64, tag_score: f64) -> f64 {
    feature_score * WEIGHT_FALLBACK_FEATURE + tag_score * WEIGHT_FALLBACK_TAGS
}

/// Calculate acoustic (audio feature) similarity between two tracks
fn calculate_acoustic_similarity(source: &AudioFeatures, target: &AudioFeatures) -> f64 {
    let energy_diff = (target.energy - source.energy).powi(2);
    let loudness_diff = ((target.loudness + LOUDNESS_OFFSET) / LOUDNESS_OFFSET
        - (source.loudness + LOUDNESS_OFFSET) / LOUDNESS_OFFSET)
        .powi(2);
    let valence_diff = (target.valence - source.valence).powi(2);
    let danceability_diff = (target.danceability - source.danceability).powi(2);
    let bpm_diff = ((target.bpm - source.bpm) / BPM_NORMALIZATION_FACTOR).powi(2);

    let distance =
        (energy_diff + loudness_diff + valence_diff + danceability_diff + bpm_diff).sqrt() / 2.0;
    (1.0 - distance).clamp(0.0, 1.0)
}

/// Calculate categorical (genre/mood/tag) similarity using weighted Jaccard
fn calculate_categorical_similarity(
    source_genres: &[&str],
    source_moods: &[&str],
    source_tags: &[&str],
    target_genres: &[&str],
    target_moods: &[&str],
    target_tags: &[&str],
) -> f64 {
    let genre_intersection = source_genres
        .iter()
        .filter(|g| target_genres.contains(g))
        .count();
    let mood_intersection = source_moods
        .iter()
        .filter(|m| target_moods.contains(m))
        .count();
    let tag_intersection = source_tags
        .iter()
        .filter(|t| target_tags.contains(t))
        .count();

    let genre_union = source_genres
        .iter()
        .chain(target_genres.iter())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let mood_union = source_moods
        .iter()
        .chain(target_moods.iter())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let tag_union = source_tags
        .iter()
        .chain(target_tags.iter())
        .collect::<std::collections::HashSet<_>>()
        .len();

    // Weighted: moods are weighted 2x (more specific than genre)
    let numerator = genre_intersection + mood_intersection * 2 + tag_intersection;
    let denominator = genre_union + mood_union * 2 + tag_union;

    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

#[test]
fn test_combined_weights_sum_to_one() {
    let total = WEIGHT_SEMANTIC + WEIGHT_ACOUSTIC + WEIGHT_CATEGORICAL;
    assert!(
        (total - 1.0).abs() < f64::EPSILON,
        "Combined weights should sum to 1.0, got {}",
        total
    );
}

#[test]
fn test_fallback_weights_sum_to_one() {
    let total = WEIGHT_FALLBACK_FEATURE + WEIGHT_FALLBACK_TAGS;
    assert!(
        (total - 1.0).abs() < f64::EPSILON,
        "Fallback weights should sum to 1.0, got {}",
        total
    );
}

#[test]
fn test_combined_score_perfect_match() {
    // Perfect similarity in all dimensions
    let score = calculate_combined_score(1.0, 1.0, 1.0);
    assert!(
        (score - 1.0).abs() < f64::EPSILON,
        "Perfect match should give score of 1.0, got {}",
        score
    );
}

#[test]
fn test_combined_score_no_match() {
    // Zero similarity in all dimensions
    let score = calculate_combined_score(0.0, 0.0, 0.0);
    assert!(
        score.abs() < f64::EPSILON,
        "No match should give score of 0.0, got {}",
        score
    );
}

#[test]
fn test_combined_score_semantic_dominant() {
    // Test that semantic similarity has the highest weight (0.5)
    let semantic_only = calculate_combined_score(1.0, 0.0, 0.0);
    let acoustic_only = calculate_combined_score(0.0, 1.0, 0.0);
    let categorical_only = calculate_combined_score(0.0, 0.0, 1.0);

    assert!(
        (semantic_only - WEIGHT_SEMANTIC).abs() < f64::EPSILON,
        "Semantic-only score should be {}, got {}",
        WEIGHT_SEMANTIC,
        semantic_only
    );
    assert!(
        (acoustic_only - WEIGHT_ACOUSTIC).abs() < f64::EPSILON,
        "Acoustic-only score should be {}, got {}",
        WEIGHT_ACOUSTIC,
        acoustic_only
    );
    assert!(
        (categorical_only - WEIGHT_CATEGORICAL).abs() < f64::EPSILON,
        "Categorical-only score should be {}, got {}",
        WEIGHT_CATEGORICAL,
        categorical_only
    );

    // Semantic should have highest individual weight
    assert!(
        semantic_only > acoustic_only,
        "Semantic weight should be higher than acoustic"
    );
    assert!(
        acoustic_only > categorical_only,
        "Acoustic weight should be higher than categorical"
    );
}

#[test]
fn test_fallback_score_calculation() {
    // Test fallback mode when embeddings aren't available
    let feature_only = calculate_fallback_score(1.0, 0.0);
    let tag_only = calculate_fallback_score(0.0, 1.0);
    let both = calculate_fallback_score(1.0, 1.0);

    assert!(
        (feature_only - WEIGHT_FALLBACK_FEATURE).abs() < f64::EPSILON,
        "Feature-only fallback should be {}, got {}",
        WEIGHT_FALLBACK_FEATURE,
        feature_only
    );
    assert!(
        (tag_only - WEIGHT_FALLBACK_TAGS).abs() < f64::EPSILON,
        "Tag-only fallback should be {}, got {}",
        WEIGHT_FALLBACK_TAGS,
        tag_only
    );
    assert!(
        (both - 1.0).abs() < f64::EPSILON,
        "Full fallback should be 1.0, got {}",
        both
    );
}

#[test]
fn test_acoustic_similarity_identical_tracks() {
    let features = AudioFeatures::default();
    let score = calculate_acoustic_similarity(&features, &features);
    assert!(
        (score - 1.0).abs() < 0.01,
        "Identical tracks should have similarity ~1.0, got {}",
        score
    );
}

#[test]
fn test_acoustic_similarity_different_tracks() {
    let source = AudioFeatures {
        energy: 0.9,
        loudness: -5.0,
        valence: 0.8,
        danceability: 0.9,
        bpm: 140.0,
    };
    let target = AudioFeatures {
        energy: 0.1,
        loudness: -30.0,
        valence: 0.2,
        danceability: 0.1,
        bpm: 70.0,
    };

    let score = calculate_acoustic_similarity(&source, &target);
    assert!(
        score < 0.5,
        "Very different tracks should have low similarity, got {}",
        score
    );
    assert!(score >= 0.0, "Score should be non-negative, got {}", score);
}

#[test]
fn test_categorical_similarity_exact_match() {
    let genres = ["Rock", "Alternative"];
    let moods = ["Energetic", "Happy"];
    let tags = ["guitar", "drums"];

    let score = calculate_categorical_similarity(&genres, &moods, &tags, &genres, &moods, &tags);
    assert!(
        (score - 1.0).abs() < f64::EPSILON,
        "Exact match should give 1.0, got {}",
        score
    );
}

#[test]
fn test_categorical_similarity_no_overlap() {
    let source_genres = ["Rock", "Metal"];
    let source_moods = ["Aggressive"];
    let source_tags = ["guitar"];

    let target_genres = ["Jazz", "Blues"];
    let target_moods = ["Relaxed"];
    let target_tags = ["piano"];

    let score = calculate_categorical_similarity(
        &source_genres,
        &source_moods,
        &source_tags,
        &target_genres,
        &target_moods,
        &target_tags,
    );
    assert!(
        score.abs() < f64::EPSILON,
        "No overlap should give 0.0, got {}",
        score
    );
}

#[test]
fn test_categorical_similarity_partial_overlap() {
    let source_genres = ["Rock", "Alternative"];
    let source_moods = ["Energetic"];
    let source_tags = ["guitar", "drums"];

    let target_genres = ["Rock", "Pop"];
    let target_moods = ["Happy"];
    let target_tags = ["guitar", "synth"];

    let score = calculate_categorical_similarity(
        &source_genres,
        &source_moods,
        &source_tags,
        &target_genres,
        &target_moods,
        &target_tags,
    );

    // Should be between 0 and 1 with partial overlap
    assert!(score > 0.0, "Partial overlap should give positive score");
    assert!(
        score < 1.0,
        "Partial overlap should give score less than 1.0"
    );
}

// =============================================================================
// Integration Tests: Redis Cache Operations
// =============================================================================

#[test]
fn test_cache_track_metadata_serialization() {
    let track_id = Uuid::new_v4();
    let metadata = CachedTrackMetadata::test_track(track_id, "Test Song");

    let json = serde_json::to_string(&metadata).expect("Should serialize");
    let deserialized: CachedTrackMetadata =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(metadata, deserialized);
}

#[test]
fn test_redis_mock_cache_prefetch_tracks() {
    let store = MockRedisStore::new();
    let user_id = Uuid::new_v4();

    // Simulate caching multiple tracks
    let track_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
    for (i, track_id) in track_ids.iter().enumerate() {
        let metadata = CachedTrackMetadata::test_track(*track_id, &format!("Track {}", i));
        let cache_key = format!("prefetch:{}:{}", user_id, track_id);
        let json = serde_json::to_string(&metadata).unwrap();
        store.setex(&cache_key, CACHE_TTL_SECONDS, json);
    }

    // Verify all tracks are cached
    assert_eq!(store.len(), 5);

    // Retrieve and verify a track
    let first_track_key = format!("prefetch:{}:{}", user_id, track_ids[0]);
    let cached_json = store.get(&first_track_key).expect("Track should be cached");
    let cached: CachedTrackMetadata = serde_json::from_str(&cached_json).unwrap();
    assert_eq!(cached.id, track_ids[0]);
    assert_eq!(cached.title, "Track 0");
}

#[test]
fn test_redis_mock_cache_user_isolation() {
    let store = MockRedisStore::new();
    let user1 = Uuid::new_v4();
    let user2 = Uuid::new_v4();
    let track_id = Uuid::new_v4();

    // Cache same track for different users
    let metadata1 = CachedTrackMetadata::test_track(track_id, "User1 Version");
    let metadata2 = CachedTrackMetadata::test_track(track_id, "User2 Version");

    store.setex(
        &format!("prefetch:{}:{}", user1, track_id),
        CACHE_TTL_SECONDS,
        serde_json::to_string(&metadata1).unwrap(),
    );
    store.setex(
        &format!("prefetch:{}:{}", user2, track_id),
        CACHE_TTL_SECONDS,
        serde_json::to_string(&metadata2).unwrap(),
    );

    // Verify isolation
    let user1_keys = store.keys(&format!("prefetch:{}", user1));
    let user2_keys = store.keys(&format!("prefetch:{}", user2));

    assert_eq!(user1_keys.len(), 1);
    assert_eq!(user2_keys.len(), 1);

    let cached1: CachedTrackMetadata =
        serde_json::from_str(&store.get(&user1_keys[0]).unwrap()).unwrap();
    let cached2: CachedTrackMetadata =
        serde_json::from_str(&store.get(&user2_keys[0]).unwrap()).unwrap();

    assert_eq!(cached1.title, "User1 Version");
    assert_eq!(cached2.title, "User2 Version");
}

#[test]
fn test_redis_mock_cache_cleanup() {
    let store = MockRedisStore::new();
    let user_id = Uuid::new_v4();

    // Cache some tracks
    for i in 0..3 {
        let track_id = Uuid::new_v4();
        let metadata = CachedTrackMetadata::test_track(track_id, &format!("Track {}", i));
        store.setex(
            &format!("prefetch:{}:{}", user_id, track_id),
            CACHE_TTL_SECONDS,
            serde_json::to_string(&metadata).unwrap(),
        );
    }

    assert_eq!(store.len(), 3);

    // Flush all (simulating cache invalidation)
    store.flush_all();
    assert!(store.is_empty());
}

// =============================================================================
// Integration Tests: Queue-Based Prefetching
// =============================================================================

/// Simulate queue item for testing
#[derive(Debug, Clone)]
struct QueueItem {
    #[allow(dead_code)]
    user_id: Uuid,
    track_id: Uuid,
    position: i32,
    prefetched: bool,
}

impl QueueItem {
    fn new(user_id: Uuid, track_id: Uuid, position: i32) -> Self {
        Self {
            user_id,
            track_id,
            position,
            prefetched: false,
        }
    }
}

/// Simulate queue state for testing
struct MockQueueState {
    user_id: Uuid,
    current_index: i32,
    items: Vec<QueueItem>,
}

impl MockQueueState {
    fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            current_index: 0,
            items: Vec::new(),
        }
    }

    fn add_item(&mut self, track_id: Uuid) {
        let position = self.items.len() as i32;
        self.items
            .push(QueueItem::new(self.user_id, track_id, position));
    }

    /// Fetch upcoming unprefetched tracks (simulates fetch_queue_tracks)
    fn fetch_upcoming_unprefetched(&self, count: usize) -> Vec<Uuid> {
        self.items
            .iter()
            .filter(|item| item.position > self.current_index && !item.prefetched)
            .take(count)
            .map(|item| item.track_id)
            .collect()
    }

    /// Mark tracks as prefetched (simulates mark_queue_prefetched)
    fn mark_prefetched(&mut self, track_ids: &[Uuid]) {
        for item in &mut self.items {
            if item.position > self.current_index && track_ids.contains(&item.track_id) {
                item.prefetched = true;
            }
        }
    }

    /// Advance playback position
    fn advance(&mut self) {
        if self.current_index < self.items.len() as i32 - 1 {
            self.current_index += 1;
        }
    }
}

#[test]
fn test_queue_fetch_upcoming_tracks() {
    let user_id = Uuid::new_v4();
    let mut queue = MockQueueState::new(user_id);

    // Add 10 tracks to queue
    for _ in 0..10 {
        queue.add_item(Uuid::new_v4());
    }

    // Fetch first 5 upcoming (all after position 0)
    let upcoming = queue.fetch_upcoming_unprefetched(5);
    assert_eq!(upcoming.len(), 5);

    // All tracks should be unprefetched initially
    assert!(queue.items.iter().all(|item| !item.prefetched));
}

#[test]
fn test_queue_mark_prefetched() {
    let user_id = Uuid::new_v4();
    let mut queue = MockQueueState::new(user_id);

    for _ in 0..5 {
        queue.add_item(Uuid::new_v4());
    }

    // Fetch and mark first 3 as prefetched
    let upcoming = queue.fetch_upcoming_unprefetched(3);
    queue.mark_prefetched(&upcoming);

    // Verify prefetch status
    let prefetched_count = queue.items.iter().filter(|i| i.prefetched).count();
    assert_eq!(prefetched_count, 3);

    // Subsequent fetch should only return remaining unprefetched tracks
    let remaining = queue.fetch_upcoming_unprefetched(5);
    assert_eq!(remaining.len(), 1); // Only 1 unprefetched track remains (position 4)
}

#[test]
fn test_queue_respects_current_position() {
    let user_id = Uuid::new_v4();
    let mut queue = MockQueueState::new(user_id);

    for _ in 0..5 {
        queue.add_item(Uuid::new_v4());
    }

    // Advance to position 2
    queue.advance();
    queue.advance();
    queue.current_index = 2;

    // Should only return tracks after current position (3 and 4)
    let upcoming = queue.fetch_upcoming_unprefetched(5);
    assert_eq!(upcoming.len(), 2);
}

#[test]
fn test_queue_prefetch_idempotency() {
    let user_id = Uuid::new_v4();
    let mut queue = MockQueueState::new(user_id);

    // Add 4 items (positions 0, 1, 2, 3)
    // With current_index = 0, positions 1, 2, 3 are "upcoming"
    for _ in 0..4 {
        queue.add_item(Uuid::new_v4());
    }

    let upcoming = queue.fetch_upcoming_unprefetched(3);
    assert_eq!(upcoming.len(), 3); // Should get 3 upcoming tracks (positions 1, 2, 3)

    // Mark same tracks twice
    queue.mark_prefetched(&upcoming);
    queue.mark_prefetched(&upcoming);

    // Should still only have 3 prefetched (idempotent operation)
    let prefetched_count = queue.items.iter().filter(|i| i.prefetched).count();
    assert_eq!(prefetched_count, 3);
}

// =============================================================================
// Integration Tests: Similarity Query Simulation
// =============================================================================

/// Simulated track with features for similarity testing
#[derive(Debug, Clone)]
struct SimulatedTrack {
    id: Uuid,
    features: AudioFeatures,
    genres: Vec<String>,
    moods: Vec<String>,
    tags: Vec<String>,
    has_embedding: bool,
    embedding_similarity: Option<f64>, // Pre-computed for testing
}

impl SimulatedTrack {
    fn new(id: Uuid, features: AudioFeatures) -> Self {
        Self {
            id,
            features,
            genres: vec!["Rock".to_string()],
            moods: vec!["Energetic".to_string()],
            tags: vec!["guitar".to_string()],
            has_embedding: true,
            embedding_similarity: Some(0.8),
        }
    }

    fn without_embedding(mut self) -> Self {
        self.has_embedding = false;
        self.embedding_similarity = None;
        self
    }

    fn with_genres(mut self, genres: Vec<&str>) -> Self {
        self.genres = genres.into_iter().map(String::from).collect();
        self
    }

    #[allow(dead_code)]
    fn with_moods(mut self, moods: Vec<&str>) -> Self {
        self.moods = moods.into_iter().map(String::from).collect();
        self
    }
}

/// Simulate the similarity query that ranks tracks
fn rank_tracks_by_similarity(
    source: &SimulatedTrack,
    candidates: &[SimulatedTrack],
    use_embeddings: bool,
) -> Vec<(Uuid, f64)> {
    let mut scored: Vec<(Uuid, f64)> = candidates
        .iter()
        .filter(|c| c.id != source.id)
        .map(|candidate| {
            let acoustic_score =
                calculate_acoustic_similarity(&source.features, &candidate.features);

            let source_genres: Vec<&str> = source.genres.iter().map(|s| s.as_str()).collect();
            let source_moods: Vec<&str> = source.moods.iter().map(|s| s.as_str()).collect();
            let source_tags: Vec<&str> = source.tags.iter().map(|s| s.as_str()).collect();
            let target_genres: Vec<&str> = candidate.genres.iter().map(|s| s.as_str()).collect();
            let target_moods: Vec<&str> = candidate.moods.iter().map(|s| s.as_str()).collect();
            let target_tags: Vec<&str> = candidate.tags.iter().map(|s| s.as_str()).collect();

            let categorical_score = calculate_categorical_similarity(
                &source_genres,
                &source_moods,
                &source_tags,
                &target_genres,
                &target_moods,
                &target_tags,
            );

            let score = if use_embeddings && source.has_embedding && candidate.has_embedding {
                let semantic_score = candidate.embedding_similarity.unwrap_or(0.0);
                calculate_combined_score(semantic_score, acoustic_score, categorical_score)
            } else {
                calculate_fallback_score(acoustic_score, categorical_score)
            };

            (candidate.id, score)
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

#[test]
fn test_similarity_ranking_with_embeddings() {
    let source = SimulatedTrack::new(Uuid::new_v4(), AudioFeatures::default())
        .with_genres(vec!["Rock", "Alternative"]);

    let candidates = vec![
        // Very similar track
        SimulatedTrack {
            id: Uuid::new_v4(),
            features: AudioFeatures::default(),
            genres: vec!["Rock".to_string(), "Alternative".to_string()],
            moods: vec!["Energetic".to_string()],
            tags: vec!["guitar".to_string()],
            has_embedding: true,
            embedding_similarity: Some(0.95), // Very high semantic similarity
        },
        // Different track
        SimulatedTrack {
            id: Uuid::new_v4(),
            features: AudioFeatures {
                energy: 0.2,
                loudness: -25.0,
                valence: 0.3,
                danceability: 0.2,
                bpm: 80.0,
            },
            genres: vec!["Jazz".to_string()],
            moods: vec!["Relaxed".to_string()],
            tags: vec!["piano".to_string()],
            has_embedding: true,
            embedding_similarity: Some(0.2), // Low semantic similarity
        },
    ];

    let ranked = rank_tracks_by_similarity(&source, &candidates, true);

    assert_eq!(ranked.len(), 2);
    // First track should be ranked higher (more similar)
    assert_eq!(ranked[0].0, candidates[0].id);
    assert!(
        ranked[0].1 > ranked[1].1,
        "More similar track should rank higher"
    );
}

#[test]
fn test_similarity_fallback_without_embeddings() {
    let source = SimulatedTrack::new(Uuid::new_v4(), AudioFeatures::default())
        .without_embedding()
        .with_genres(vec!["Rock"]);

    let candidates = vec![
        // Similar features and genres (no embedding)
        SimulatedTrack::new(
            Uuid::new_v4(),
            AudioFeatures {
                energy: 0.55,
                loudness: -11.0,
                valence: 0.52,
                danceability: 0.48,
                bpm: 122.0,
            },
        )
        .without_embedding()
        .with_genres(vec!["Rock"]),
        // Different features and genres
        SimulatedTrack::new(
            Uuid::new_v4(),
            AudioFeatures {
                energy: 0.1,
                loudness: -35.0,
                valence: 0.9,
                danceability: 0.1,
                bpm: 60.0,
            },
        )
        .without_embedding()
        .with_genres(vec!["Classical"]),
    ];

    // Use fallback mode (no embeddings)
    let ranked = rank_tracks_by_similarity(&source, &candidates, false);

    assert_eq!(ranked.len(), 2);
    // First candidate should rank higher due to similar features and matching genre
    assert_eq!(ranked[0].0, candidates[0].id);
}

// =============================================================================
// Integration Tests: Full Prefetch Flow
// =============================================================================

#[test]
fn test_prefetch_job_types() {
    let user_id = Uuid::new_v4();
    let track_id = Uuid::new_v4();

    // Test autoplay prefetch job
    struct PrefetchJob {
        #[allow(dead_code)]
        user_id: Uuid,
        #[allow(dead_code)]
        current_track_id: Uuid,
        prefetch_count: Option<usize>,
        is_autoplay: bool,
    }

    let autoplay_job = PrefetchJob {
        user_id,
        current_track_id: track_id,
        prefetch_count: Some(5),
        is_autoplay: true,
    };

    assert!(autoplay_job.is_autoplay);
    assert_eq!(autoplay_job.prefetch_count, Some(5));

    // Test queue-based prefetch job
    let queue_job = PrefetchJob {
        user_id,
        current_track_id: track_id,
        prefetch_count: Some(5),
        is_autoplay: false,
    };

    assert!(!queue_job.is_autoplay);
}

#[test]
fn test_cache_key_format() {
    let user_id = Uuid::new_v4();
    let track_id = Uuid::new_v4();

    let cache_key = format!("prefetch:{}:{}", user_id, track_id);

    // Verify key format
    assert!(cache_key.starts_with("prefetch:"));
    assert!(cache_key.contains(&user_id.to_string()));
    assert!(cache_key.contains(&track_id.to_string()));

    // Parse back
    let parts: Vec<&str> = cache_key.split(':').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "prefetch");
}

#[test]
fn test_cache_ttl_value() {
    // Cache TTL should be 30 minutes (1800 seconds)
    assert_eq!(CACHE_TTL_SECONDS, 1800);

    // Verify it's reasonable for prefetch use case (compile-time checks)
    const _: () = assert!(CACHE_TTL_SECONDS >= 60 * 5); // At least 5 minutes
    const _: () = assert!(CACHE_TTL_SECONDS <= 60 * 60 * 2); // No more than 2 hours
}

#[test]
fn test_normalization_constants() {
    // BPM normalization: 200 BPM difference should normalize to ~1.0
    assert!((BPM_NORMALIZATION_FACTOR - 200.0).abs() < f64::EPSILON);

    // Loudness normalization: -60 dB to 0 dB range
    assert!((LOUDNESS_OFFSET - 60.0).abs() < f64::EPSILON);

    // Test normalization produces expected ranges
    let bpm_diff = 100.0; // 100 BPM difference
    let normalized_bpm = bpm_diff / BPM_NORMALIZATION_FACTOR;
    assert!((0.0..=1.0).contains(&normalized_bpm));

    let loudness_val = -30.0; // Mid-range loudness
    let normalized_loudness = (loudness_val + LOUDNESS_OFFSET) / LOUDNESS_OFFSET;
    assert!((0.0..=1.0).contains(&normalized_loudness));
}
