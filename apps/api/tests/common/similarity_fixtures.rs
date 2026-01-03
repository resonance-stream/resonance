//! Test fixtures for similarity service integration tests
//!
//! Provides comprehensive test data for testing track similarity methods:
//! - Embedding-based (semantic) similarity
//! - Audio feature-based (acoustic) similarity
//! - Tag-based (categorical) similarity
//! - Combined similarity (weighted blend)
//!
//! # Usage
//!
//! ```rust
//! use common::similarity_fixtures::*;
//!
//! let mut ctx = TestContext::new(pool).await;
//! let fixtures = SimilarityTestFixtures::new();
//!
//! // Create a complete track with all similarity features
//! let track_id = fixtures.rock_energetic.create(&mut ctx).await;
//!
//! // Or create tracks with specific attributes
//! let custom = TrackFixtureBuilder::new("Custom Track")
//!     .genres(&["electronic", "ambient"])
//!     .moods(&["calm", "ethereal"])
//!     .bpm(90.0)
//!     .energy(0.3)
//!     .with_embedding_seed(42)
//!     .build();
//! let track_id = custom.create(&mut ctx).await;
//! ```

#![allow(dead_code)]

use serde_json::json;

// ============================================================================
// Embedding Generation
// ============================================================================

/// Embedding dimension used by the system (768 for typical text embeddings)
pub const EMBEDDING_DIMENSION: usize = 768;

/// Generate a deterministic test embedding vector
///
/// Creates a 768-dimensional embedding where similar seeds produce similar embeddings.
/// This allows testing that embedding similarity works correctly.
///
/// # Arguments
/// * `seed` - A seed value where close values produce similar embeddings
///
/// # Returns
/// A 768-dimensional f32 array suitable for pgvector
pub fn generate_test_embedding(seed: u8) -> [f32; EMBEDDING_DIMENSION] {
    let mut embedding = [0.0f32; EMBEDDING_DIMENSION];
    let seed_f = seed as f32;

    for (i, val) in embedding.iter_mut().enumerate() {
        // Create a pattern based on seed so similar seeds = similar embeddings
        // Using sine waves with seed-based phase shifts for smoother similarity
        let base = (i as f32 * 0.01 + seed_f * 0.1).sin();
        let harmonic = (i as f32 * 0.02 + seed_f * 0.05).cos();
        *val = (base * 0.6 + harmonic * 0.4) * 0.1;
    }

    // Normalize the embedding for consistent cosine similarity
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for val in embedding.iter_mut() {
            *val /= magnitude;
        }
    }

    embedding
}

/// Generate a "dissimilar" embedding by using a very different pattern
///
/// Creates an embedding that should have low similarity to embeddings from
/// `generate_test_embedding` with typical seed values (0-50).
pub fn generate_dissimilar_embedding(seed: u8) -> [f32; EMBEDDING_DIMENSION] {
    let mut embedding = [0.0f32; EMBEDDING_DIMENSION];
    let seed_f = seed as f32;

    for (i, val) in embedding.iter_mut().enumerate() {
        // Use different pattern that produces low similarity
        let base = (i as f32 * 0.05 + seed_f * 0.3 + 100.0).cos() * 0.7;
        let harmonic = (i as f32 * 0.03 - seed_f * 0.2 + 50.0).sin() * 0.3;
        *val = (base + harmonic) * 0.1;
    }

    // Normalize
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for val in embedding.iter_mut() {
            *val /= magnitude;
        }
    }

    embedding
}

// ============================================================================
// Audio Features
// ============================================================================

/// Audio features for testing acoustic similarity
#[derive(Debug, Clone)]
pub struct AudioFeaturesFixture {
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub mode: Option<String>,
    pub loudness: Option<f64>,
    pub energy: Option<f64>,
    pub danceability: Option<f64>,
    pub valence: Option<f64>,
    pub acousticness: Option<f64>,
    pub instrumentalness: Option<f64>,
    pub speechiness: Option<f64>,
}

impl Default for AudioFeaturesFixture {
    fn default() -> Self {
        Self {
            bpm: Some(120.0),
            key: Some("C".to_string()),
            mode: Some("major".to_string()),
            loudness: Some(-8.0),
            energy: Some(0.7),
            danceability: Some(0.6),
            valence: Some(0.5),
            acousticness: Some(0.3),
            instrumentalness: Some(0.1),
            speechiness: Some(0.05),
        }
    }
}

impl AudioFeaturesFixture {
    /// Create empty/null audio features
    pub fn empty() -> Self {
        Self {
            bpm: None,
            key: None,
            mode: None,
            loudness: None,
            energy: None,
            danceability: None,
            valence: None,
            acousticness: None,
            instrumentalness: None,
            speechiness: None,
        }
    }

    /// Create high-energy features (typical for rock/electronic)
    pub fn high_energy() -> Self {
        Self {
            bpm: Some(140.0),
            key: Some("E".to_string()),
            mode: Some("minor".to_string()),
            loudness: Some(-5.0),
            energy: Some(0.9),
            danceability: Some(0.7),
            valence: Some(0.6),
            acousticness: Some(0.1),
            instrumentalness: Some(0.2),
            speechiness: Some(0.03),
        }
    }

    /// Create calm/acoustic features (typical for folk/ambient)
    pub fn calm() -> Self {
        Self {
            bpm: Some(80.0),
            key: Some("G".to_string()),
            mode: Some("major".to_string()),
            loudness: Some(-15.0),
            energy: Some(0.25),
            danceability: Some(0.3),
            valence: Some(0.4),
            acousticness: Some(0.85),
            instrumentalness: Some(0.3),
            speechiness: Some(0.04),
        }
    }

    /// Create dance music features
    pub fn dance() -> Self {
        Self {
            bpm: Some(128.0),
            key: Some("A".to_string()),
            mode: Some("minor".to_string()),
            loudness: Some(-6.0),
            energy: Some(0.85),
            danceability: Some(0.92),
            valence: Some(0.75),
            acousticness: Some(0.05),
            instrumentalness: Some(0.8),
            speechiness: Some(0.05),
        }
    }

    /// Create jazz features
    pub fn jazz() -> Self {
        Self {
            bpm: Some(95.0),
            key: Some("Bb".to_string()),
            mode: Some("major".to_string()),
            loudness: Some(-12.0),
            energy: Some(0.45),
            danceability: Some(0.55),
            valence: Some(0.6),
            acousticness: Some(0.7),
            instrumentalness: Some(0.6),
            speechiness: Some(0.08),
        }
    }

    /// Create classical features
    pub fn classical() -> Self {
        Self {
            bpm: Some(72.0),
            key: Some("D".to_string()),
            mode: Some("major".to_string()),
            loudness: Some(-18.0),
            energy: Some(0.35),
            danceability: Some(0.2),
            valence: Some(0.55),
            acousticness: Some(0.95),
            instrumentalness: Some(0.98),
            speechiness: Some(0.02),
        }
    }

    /// Create hip-hop features
    pub fn hip_hop() -> Self {
        Self {
            bpm: Some(90.0),
            key: Some("F".to_string()),
            mode: Some("minor".to_string()),
            loudness: Some(-7.0),
            energy: Some(0.65),
            danceability: Some(0.82),
            valence: Some(0.4),
            acousticness: Some(0.15),
            instrumentalness: Some(0.02),
            speechiness: Some(0.35),
        }
    }

    /// Convert to serde_json::Value
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "bpm": self.bpm,
            "key": self.key,
            "mode": self.mode,
            "loudness": self.loudness,
            "energy": self.energy,
            "danceability": self.danceability,
            "valence": self.valence,
            "acousticness": self.acousticness,
            "instrumentalness": self.instrumentalness,
            "speechiness": self.speechiness
        })
    }

    /// Create features slightly similar to these (small variations)
    pub fn similar(&self) -> Self {
        Self {
            bpm: self.bpm.map(|v| v + 5.0),
            key: self.key.clone(),
            mode: self.mode.clone(),
            loudness: self.loudness.map(|v| v - 1.0),
            energy: self.energy.map(|v| (v + 0.05).min(1.0)),
            danceability: self.danceability.map(|v| (v - 0.05).max(0.0)),
            valence: self.valence.map(|v| (v + 0.03).min(1.0)),
            acousticness: self.acousticness.map(|v| (v - 0.02).max(0.0)),
            instrumentalness: self.instrumentalness.map(|v| (v + 0.01).min(1.0)),
            speechiness: self.speechiness.map(|v| (v - 0.01).max(0.0)),
        }
    }
}

// ============================================================================
// Track Fixture Builder
// ============================================================================

/// Builder for creating complete track fixtures
#[derive(Debug, Clone)]
pub struct TrackFixtureBuilder {
    title: String,
    genres: Vec<String>,
    moods: Vec<String>,
    tags: Vec<String>,
    audio_features: AudioFeaturesFixture,
    embedding_seed: Option<u8>,
    use_dissimilar_embedding: bool,
}

impl TrackFixtureBuilder {
    /// Create a new track fixture builder
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            genres: vec!["rock".to_string()],
            moods: vec!["energetic".to_string()],
            tags: vec!["guitar".to_string()],
            audio_features: AudioFeaturesFixture::default(),
            embedding_seed: None,
            use_dissimilar_embedding: false,
        }
    }

    /// Set genres for the track
    pub fn genres(mut self, genres: &[&str]) -> Self {
        self.genres = genres.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set moods for the track
    pub fn moods(mut self, moods: &[&str]) -> Self {
        self.moods = moods.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set tags for the track
    pub fn tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set audio features
    pub fn audio_features(mut self, features: AudioFeaturesFixture) -> Self {
        self.audio_features = features;
        self
    }

    /// Set BPM directly
    pub fn bpm(mut self, bpm: f64) -> Self {
        self.audio_features.bpm = Some(bpm);
        self
    }

    /// Set energy directly
    pub fn energy(mut self, energy: f64) -> Self {
        self.audio_features.energy = Some(energy);
        self
    }

    /// Set loudness directly
    pub fn loudness(mut self, loudness: f64) -> Self {
        self.audio_features.loudness = Some(loudness);
        self
    }

    /// Set valence directly
    pub fn valence(mut self, valence: f64) -> Self {
        self.audio_features.valence = Some(valence);
        self
    }

    /// Set danceability directly
    pub fn danceability(mut self, danceability: f64) -> Self {
        self.audio_features.danceability = Some(danceability);
        self
    }

    /// Include an embedding with the given seed
    pub fn with_embedding_seed(mut self, seed: u8) -> Self {
        self.embedding_seed = Some(seed);
        self.use_dissimilar_embedding = false;
        self
    }

    /// Include a dissimilar embedding (for testing negative cases)
    pub fn with_dissimilar_embedding(mut self, seed: u8) -> Self {
        self.embedding_seed = Some(seed);
        self.use_dissimilar_embedding = true;
        self
    }

    /// Build the fixture data (returns raw data for manual insertion)
    pub fn build(self) -> TrackFixture {
        TrackFixture {
            title: self.title,
            genres: self.genres,
            moods: self.moods,
            tags: self.tags,
            audio_features: self.audio_features,
            embedding_seed: self.embedding_seed,
            use_dissimilar_embedding: self.use_dissimilar_embedding,
        }
    }
}

/// Complete track fixture data
#[derive(Debug, Clone)]
pub struct TrackFixture {
    pub title: String,
    pub genres: Vec<String>,
    pub moods: Vec<String>,
    pub tags: Vec<String>,
    pub audio_features: AudioFeaturesFixture,
    pub embedding_seed: Option<u8>,
    pub use_dissimilar_embedding: bool,
}

impl TrackFixture {
    /// Get genres as slice of &str (for SQL binding)
    pub fn genres_as_refs(&self) -> Vec<&str> {
        self.genres.iter().map(|s| s.as_str()).collect()
    }

    /// Get moods as slice of &str (for SQL binding)
    pub fn moods_as_refs(&self) -> Vec<&str> {
        self.moods.iter().map(|s| s.as_str()).collect()
    }

    /// Get tags as slice of &str (for SQL binding)
    pub fn tags_as_refs(&self) -> Vec<&str> {
        self.tags.iter().map(|s| s.as_str()).collect()
    }

    /// Get the embedding if configured
    pub fn get_embedding(&self) -> Option<[f32; EMBEDDING_DIMENSION]> {
        self.embedding_seed.map(|seed| {
            if self.use_dissimilar_embedding {
                generate_dissimilar_embedding(seed)
            } else {
                generate_test_embedding(seed)
            }
        })
    }

    /// Check if this fixture has an embedding
    pub fn has_embedding(&self) -> bool {
        self.embedding_seed.is_some()
    }

    /// Check if this fixture has complete audio features
    pub fn has_audio_features(&self) -> bool {
        self.audio_features.energy.is_some() && self.audio_features.loudness.is_some()
    }
}

// ============================================================================
// Predefined Fixtures Collection
// ============================================================================

/// Collection of predefined track fixtures for testing various similarity scenarios
pub struct SimilarityTestFixtures {
    // Rock/Indie cluster (should be similar to each other)
    pub rock_energetic: TrackFixture,
    pub rock_indie: TrackFixture,
    pub rock_alternative: TrackFixture,

    // Electronic/Dance cluster
    pub electronic_dance: TrackFixture,
    pub electronic_ambient: TrackFixture,
    pub electronic_house: TrackFixture,

    // Classical/Orchestral cluster
    pub classical_symphony: TrackFixture,
    pub classical_piano: TrackFixture,
    pub classical_chamber: TrackFixture,

    // Jazz cluster
    pub jazz_bebop: TrackFixture,
    pub jazz_smooth: TrackFixture,

    // Outliers (for testing dissimilarity)
    pub unique_experimental: TrackFixture,
    pub unique_noise: TrackFixture,
}

impl SimilarityTestFixtures {
    /// Create a new set of predefined fixtures
    pub fn new() -> Self {
        Self {
            // Rock cluster - embedding seeds 1-3 for similarity
            rock_energetic: TrackFixtureBuilder::new("Rock Energetic Track")
                .genres(&["rock", "hard rock"])
                .moods(&["energetic", "powerful", "driving"])
                .tags(&["guitar", "drums", "distortion"])
                .audio_features(AudioFeaturesFixture::high_energy())
                .with_embedding_seed(1)
                .build(),

            rock_indie: TrackFixtureBuilder::new("Indie Rock Track")
                .genres(&["rock", "indie", "alternative"])
                .moods(&["energetic", "nostalgic"])
                .tags(&["guitar", "drums", "reverb"])
                .audio_features(AudioFeaturesFixture::high_energy().similar())
                .with_embedding_seed(2)
                .build(),

            rock_alternative: TrackFixtureBuilder::new("Alternative Rock Track")
                .genres(&["rock", "alternative", "grunge"])
                .moods(&["energetic", "aggressive", "raw"])
                .tags(&["guitar", "bass", "distortion"])
                .audio_features(AudioFeaturesFixture::high_energy().similar())
                .with_embedding_seed(3)
                .build(),

            // Electronic cluster - embedding seeds 10-12
            electronic_dance: TrackFixtureBuilder::new("Electronic Dance Track")
                .genres(&["electronic", "dance", "edm"])
                .moods(&["euphoric", "energetic", "uplifting"])
                .tags(&["synth", "bass", "beat"])
                .audio_features(AudioFeaturesFixture::dance())
                .with_embedding_seed(10)
                .build(),

            electronic_ambient: TrackFixtureBuilder::new("Ambient Electronic Track")
                .genres(&["electronic", "ambient", "chillout"])
                .moods(&["calm", "ethereal", "dreamy"])
                .tags(&["synth", "pads", "atmospheric"])
                .audio_features(AudioFeaturesFixture::calm())
                .with_embedding_seed(11)
                .build(),

            electronic_house: TrackFixtureBuilder::new("House Music Track")
                .genres(&["electronic", "house", "dance"])
                .moods(&["euphoric", "groovy", "upbeat"])
                .tags(&["synth", "bass", "four-on-the-floor"])
                .audio_features(AudioFeaturesFixture::dance().similar())
                .with_embedding_seed(12)
                .build(),

            // Classical cluster - embedding seeds 20-22
            classical_symphony: TrackFixtureBuilder::new("Symphony Orchestra Track")
                .genres(&["classical", "orchestral", "symphony"])
                .moods(&["majestic", "dramatic", "emotional"])
                .tags(&["strings", "brass", "orchestra"])
                .audio_features(AudioFeaturesFixture::classical())
                .with_embedding_seed(20)
                .build(),

            classical_piano: TrackFixtureBuilder::new("Classical Piano Track")
                .genres(&["classical", "piano", "solo"])
                .moods(&["contemplative", "elegant", "refined"])
                .tags(&["piano", "solo", "acoustic"])
                .audio_features(AudioFeaturesFixture::classical().similar())
                .with_embedding_seed(21)
                .build(),

            classical_chamber: TrackFixtureBuilder::new("Chamber Music Track")
                .genres(&["classical", "chamber", "ensemble"])
                .moods(&["intimate", "refined", "delicate"])
                .tags(&["strings", "quartet", "acoustic"])
                .audio_features(AudioFeaturesFixture::classical().similar())
                .with_embedding_seed(22)
                .build(),

            // Jazz cluster - embedding seeds 30-31
            jazz_bebop: TrackFixtureBuilder::new("Bebop Jazz Track")
                .genres(&["jazz", "bebop", "swing"])
                .moods(&["lively", "sophisticated", "improvisational"])
                .tags(&["saxophone", "piano", "drums"])
                .audio_features(AudioFeaturesFixture::jazz())
                .with_embedding_seed(30)
                .build(),

            jazz_smooth: TrackFixtureBuilder::new("Smooth Jazz Track")
                .genres(&["jazz", "smooth jazz", "fusion"])
                .moods(&["relaxed", "sophisticated", "mellow"])
                .tags(&["saxophone", "keyboard", "bass"])
                .audio_features(AudioFeaturesFixture::jazz().similar())
                .with_embedding_seed(31)
                .build(),

            // Unique/Outlier tracks - use dissimilar embeddings
            unique_experimental: TrackFixtureBuilder::new("Experimental Noise Track")
                .genres(&["experimental", "noise", "avant-garde"])
                .moods(&["unsettling", "chaotic", "abstract"])
                .tags(&["noise", "atonal", "experimental"])
                .audio_features(AudioFeaturesFixture {
                    bpm: Some(0.0),
                    key: None,
                    mode: None,
                    loudness: Some(-3.0),
                    energy: Some(0.95),
                    danceability: Some(0.1),
                    valence: Some(0.2),
                    acousticness: Some(0.05),
                    instrumentalness: Some(0.99),
                    speechiness: Some(0.01),
                })
                .with_dissimilar_embedding(100)
                .build(),

            unique_noise: TrackFixtureBuilder::new("Industrial Noise Track")
                .genres(&["industrial", "noise", "harsh"])
                .moods(&["aggressive", "intense", "abrasive"])
                .tags(&["noise", "industrial", "machine"])
                .audio_features(AudioFeaturesFixture {
                    bpm: Some(150.0),
                    key: None,
                    mode: None,
                    loudness: Some(-2.0),
                    energy: Some(0.99),
                    danceability: Some(0.15),
                    valence: Some(0.1),
                    acousticness: Some(0.01),
                    instrumentalness: Some(0.95),
                    speechiness: Some(0.02),
                })
                .with_dissimilar_embedding(150)
                .build(),
        }
    }

    /// Get all rock cluster fixtures
    pub fn rock_cluster(&self) -> Vec<&TrackFixture> {
        vec![&self.rock_energetic, &self.rock_indie, &self.rock_alternative]
    }

    /// Get all electronic cluster fixtures
    pub fn electronic_cluster(&self) -> Vec<&TrackFixture> {
        vec![&self.electronic_dance, &self.electronic_ambient, &self.electronic_house]
    }

    /// Get all classical cluster fixtures
    pub fn classical_cluster(&self) -> Vec<&TrackFixture> {
        vec![&self.classical_symphony, &self.classical_piano, &self.classical_chamber]
    }

    /// Get all jazz cluster fixtures
    pub fn jazz_cluster(&self) -> Vec<&TrackFixture> {
        vec![&self.jazz_bebop, &self.jazz_smooth]
    }

    /// Get outlier/unique fixtures
    pub fn outliers(&self) -> Vec<&TrackFixture> {
        vec![&self.unique_experimental, &self.unique_noise]
    }

    /// Get all fixtures
    pub fn all(&self) -> Vec<&TrackFixture> {
        let mut all = self.rock_cluster();
        all.extend(self.electronic_cluster());
        all.extend(self.classical_cluster());
        all.extend(self.jazz_cluster());
        all.extend(self.outliers());
        all
    }
}

impl Default for SimilarityTestFixtures {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests for the fixtures themselves
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_generation_produces_valid_vectors() {
        let embedding = generate_test_embedding(1);

        // Check dimension
        assert_eq!(embedding.len(), EMBEDDING_DIMENSION);

        // Check normalization (magnitude should be ~1.0)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001, "Embedding should be normalized");
    }

    #[test]
    fn test_similar_seeds_produce_similar_embeddings() {
        let emb1 = generate_test_embedding(1);
        let emb2 = generate_test_embedding(2);
        let emb_far = generate_test_embedding(100);

        // Calculate cosine similarity
        let sim_close: f32 = emb1.iter().zip(emb2.iter()).map(|(a, b)| a * b).sum();
        let sim_far: f32 = emb1.iter().zip(emb_far.iter()).map(|(a, b)| a * b).sum();

        // Close seeds should have higher similarity than far seeds
        assert!(
            sim_close > sim_far,
            "Close seeds should be more similar: {} vs {}",
            sim_close,
            sim_far
        );
    }

    #[test]
    fn test_dissimilar_embedding_is_different() {
        let normal = generate_test_embedding(1);
        let dissimilar = generate_dissimilar_embedding(1);

        // Calculate cosine similarity
        let similarity: f32 = normal.iter().zip(dissimilar.iter()).map(|(a, b)| a * b).sum();

        // Should have low similarity (could be positive or negative, but low magnitude)
        assert!(
            similarity.abs() < 0.5,
            "Dissimilar embedding should have low similarity: {}",
            similarity
        );
    }

    #[test]
    fn test_audio_features_default_values() {
        let features = AudioFeaturesFixture::default();

        assert_eq!(features.bpm, Some(120.0));
        assert_eq!(features.energy, Some(0.7));
        assert!(features.loudness.is_some());
    }

    #[test]
    fn test_audio_features_empty() {
        let features = AudioFeaturesFixture::empty();

        assert!(features.bpm.is_none());
        assert!(features.energy.is_none());
        assert!(features.loudness.is_none());
    }

    #[test]
    fn test_audio_features_to_json() {
        let features = AudioFeaturesFixture::default();
        let json = features.to_json();

        assert_eq!(json["bpm"], 120.0);
        assert_eq!(json["energy"], 0.7);
    }

    #[test]
    fn test_track_fixture_builder() {
        let fixture = TrackFixtureBuilder::new("Test Track")
            .genres(&["rock", "metal"])
            .moods(&["aggressive", "powerful"])
            .tags(&["guitar", "drums"])
            .bpm(180.0)
            .energy(0.95)
            .with_embedding_seed(42)
            .build();

        assert_eq!(fixture.title, "Test Track");
        assert_eq!(fixture.genres, vec!["rock", "metal"]);
        assert_eq!(fixture.moods, vec!["aggressive", "powerful"]);
        assert_eq!(fixture.audio_features.bpm, Some(180.0));
        assert_eq!(fixture.audio_features.energy, Some(0.95));
        assert_eq!(fixture.embedding_seed, Some(42));
        assert!(!fixture.use_dissimilar_embedding);
    }

    #[test]
    fn test_similarity_fixtures_clusters() {
        let fixtures = SimilarityTestFixtures::new();

        // Check cluster sizes
        assert_eq!(fixtures.rock_cluster().len(), 3);
        assert_eq!(fixtures.electronic_cluster().len(), 3);
        assert_eq!(fixtures.classical_cluster().len(), 3);
        assert_eq!(fixtures.jazz_cluster().len(), 2);
        assert_eq!(fixtures.outliers().len(), 2);

        // Check total count
        assert_eq!(fixtures.all().len(), 13);
    }

    #[test]
    fn test_similar_audio_features() {
        let base = AudioFeaturesFixture::high_energy();
        let similar = base.similar();

        // Similar features should have small differences
        let bpm_diff = (base.bpm.unwrap() - similar.bpm.unwrap()).abs();
        assert!(bpm_diff <= 10.0, "BPM should be similar");

        let energy_diff = (base.energy.unwrap() - similar.energy.unwrap()).abs();
        assert!(energy_diff <= 0.1, "Energy should be similar");
    }
}
