//! Test fixtures for worker integration tests
//!
//! Provides reusable test data, configuration builders, and database fixtures.

use std::collections::HashMap;
use std::path::PathBuf;

use fake::faker::internet::en::SafeEmail;
use fake::faker::lorem::en::{Sentence, Words};
use fake::faker::name::en::Name;
use fake::Fake;
use uuid::Uuid;

/// Test environment variables builder
///
/// Builds a HashMap of environment variables for testing configuration loading.
#[derive(Debug, Default)]
pub struct TestEnvBuilder {
    vars: HashMap<String, String>,
}

impl TestEnvBuilder {
    /// Create a new test environment builder with minimal required variables
    pub fn new() -> Self {
        let mut builder = Self::default();
        builder
            .set("ENVIRONMENT", "development")
            .set(
                "DATABASE_URL",
                "postgres://test:test@localhost:5432/resonance_test",
            )
            .set("REDIS_URL", "redis://localhost:6379")
            .set("MUSIC_LIBRARY_PATH", "/tmp/test-music");
        builder
    }

    /// Create a production-like environment
    pub fn production() -> Self {
        let mut builder = Self::default();
        builder
            .set("ENVIRONMENT", "production")
            .set(
                "DATABASE_URL",
                "postgres://prod:secret@prod-host:5432/resonance",
            )
            .set("REDIS_URL", "redis://prod-redis:6379")
            .set("MUSIC_LIBRARY_PATH", "/music")
            .set("OLLAMA_URL", "http://ollama:11434")
            .set("OLLAMA_MODEL", "mistral");
        builder
    }

    /// Create environment with Lidarr configured
    pub fn with_lidarr() -> Self {
        let mut builder = Self::new();
        builder
            .set("LIDARR_URL", "http://localhost:8686")
            .set("LIDARR_API_KEY", "test-api-key");
        builder
    }

    /// Set an environment variable
    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }

    /// Remove an environment variable
    pub fn remove(&mut self, key: &str) -> &mut Self {
        self.vars.remove(key);
        self
    }

    /// Get the environment variables as a HashMap
    pub fn build(&self) -> HashMap<String, String> {
        self.vars.clone()
    }

    /// Get the environment variables as tuples for temp_env
    pub fn as_tuples(&self) -> Vec<(String, String)> {
        self.vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

/// Test configuration for worker jobs
#[derive(Debug, Clone)]
pub struct TestWorkerConfig {
    pub poll_interval_secs: u64,
    pub max_concurrent_jobs: usize,
    pub max_retries: u32,
    pub retry_delay_secs: u64,
}

impl Default for TestWorkerConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 1,
            max_concurrent_jobs: 2,
            max_retries: 1,
            retry_delay_secs: 1,
        }
    }
}

/// Test track metadata for embedding/mood tests
#[derive(Debug, Clone)]
pub struct TestTrack {
    pub id: Uuid,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub genres: Vec<String>,
    pub file_path: PathBuf,
    pub duration_ms: i32,
}

impl TestTrack {
    /// Create a new test track with random data
    pub fn random() -> Self {
        let words: Vec<String> = Words(2..4).fake();
        Self {
            id: Uuid::new_v4(),
            title: words.join(" "),
            artist_name: Some(Name().fake()),
            album_title: Some(Words(2..4).fake::<Vec<String>>().join(" ")),
            genres: vec!["Rock".to_string(), "Alternative".to_string()],
            file_path: PathBuf::from(format!("/music/track_{}.flac", Uuid::new_v4())),
            duration_ms: (180_000..300_000).fake(),
        }
    }

    /// Create a test track with known data
    pub fn known() -> Self {
        Self {
            id: Uuid::new_v4(),
            title: "Bohemian Rhapsody".to_string(),
            artist_name: Some("Queen".to_string()),
            album_title: Some("A Night at the Opera".to_string()),
            genres: vec!["Rock".to_string(), "Progressive Rock".to_string()],
            file_path: PathBuf::from("/music/queen/bohemian_rhapsody.flac"),
            duration_ms: 355_000,
        }
    }

    /// Create a minimal test track (no optional fields)
    pub fn minimal() -> Self {
        Self {
            id: Uuid::new_v4(),
            title: "Unknown Track".to_string(),
            artist_name: None,
            album_title: None,
            genres: vec![],
            file_path: PathBuf::from("/music/unknown.mp3"),
            duration_ms: 180_000,
        }
    }
}

/// Test artist for Lidarr sync tests
#[derive(Debug, Clone)]
pub struct TestArtist {
    pub id: Uuid,
    pub lidarr_id: i64,
    pub name: String,
    pub sort_name: Option<String>,
    pub biography: Option<String>,
    pub image_url: Option<String>,
    pub genres: Vec<String>,
    pub mbid: Option<Uuid>,
}

impl TestArtist {
    /// Create a random test artist
    pub fn random() -> Self {
        Self {
            id: Uuid::new_v4(),
            lidarr_id: (1..10000).fake(),
            name: Name().fake(),
            sort_name: None,
            biography: Some(Sentence(5..10).fake()),
            image_url: Some(format!("https://example.com/artist/{}.jpg", Uuid::new_v4())),
            genres: vec!["Rock".to_string()],
            mbid: Some(Uuid::new_v4()),
        }
    }

    /// Create a known test artist
    pub fn known() -> Self {
        Self {
            id: Uuid::new_v4(),
            lidarr_id: 42,
            name: "Queen".to_string(),
            sort_name: Some("Queen".to_string()),
            biography: Some("British rock band formed in London in 1970".to_string()),
            image_url: Some("https://example.com/queen.jpg".to_string()),
            genres: vec!["Rock".to_string(), "Glam Rock".to_string()],
            mbid: Some(Uuid::parse_str("0383dadf-2a4e-4d10-a46a-e9e041da8eb3").unwrap()),
        }
    }
}

/// Test album for Lidarr sync tests
#[derive(Debug, Clone)]
pub struct TestAlbum {
    pub id: Uuid,
    pub lidarr_id: i64,
    pub title: String,
    pub artist_id: Uuid,
    pub genres: Vec<String>,
    pub release_date: Option<String>,
    pub album_type: String,
    pub mbid: Option<Uuid>,
}

impl TestAlbum {
    /// Create a random test album
    pub fn random(artist_id: Uuid) -> Self {
        let words: Vec<String> = Words(2..5).fake();
        Self {
            id: Uuid::new_v4(),
            lidarr_id: (1..10000).fake(),
            title: words.join(" "),
            artist_id,
            genres: vec!["Rock".to_string()],
            release_date: Some("2024-01-15".to_string()),
            album_type: "album".to_string(),
            mbid: Some(Uuid::new_v4()),
        }
    }

    /// Create a known test album
    pub fn known(artist_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            lidarr_id: 123,
            title: "A Night at the Opera".to_string(),
            artist_id,
            genres: vec!["Rock".to_string(), "Progressive Rock".to_string()],
            release_date: Some("1975-11-21".to_string()),
            album_type: "album".to_string(),
            mbid: Some(Uuid::parse_str("6e8bae4e-4c83-44c3-b5b9-2d7bec5b8cd8").unwrap()),
        }
    }
}

/// Helper to create a temporary music library directory
pub fn create_temp_music_library() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Helper to create a mock audio file (just creates an empty file with the right extension)
pub fn create_mock_audio_file(dir: &std::path::Path, name: &str) -> PathBuf {
    let file_path = dir.join(name);
    std::fs::write(&file_path, b"mock audio data").expect("Failed to create mock audio file");
    file_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_builder_new_has_defaults() {
        let builder = TestEnvBuilder::new();
        let vars = builder.build();

        assert!(vars.contains_key("ENVIRONMENT"));
        assert!(vars.contains_key("DATABASE_URL"));
        assert!(vars.contains_key("REDIS_URL"));
        assert_eq!(vars.get("ENVIRONMENT").unwrap(), "development");
    }

    #[test]
    fn test_env_builder_production() {
        let builder = TestEnvBuilder::production();
        let vars = builder.build();

        assert_eq!(vars.get("ENVIRONMENT").unwrap(), "production");
        assert!(vars.contains_key("OLLAMA_URL"));
    }

    #[test]
    fn test_env_builder_with_lidarr() {
        let builder = TestEnvBuilder::with_lidarr();
        let vars = builder.build();

        assert!(vars.contains_key("LIDARR_URL"));
        assert!(vars.contains_key("LIDARR_API_KEY"));
    }

    #[test]
    fn test_env_builder_set_and_remove() {
        let mut builder = TestEnvBuilder::new();
        builder.set("CUSTOM_VAR", "custom_value");
        assert_eq!(builder.build().get("CUSTOM_VAR").unwrap(), "custom_value");

        builder.remove("CUSTOM_VAR");
        assert!(!builder.build().contains_key("CUSTOM_VAR"));
    }

    #[test]
    fn test_default_worker_config() {
        let config = TestWorkerConfig::default();
        assert_eq!(config.poll_interval_secs, 1);
        assert_eq!(config.max_concurrent_jobs, 2);
        assert_eq!(config.max_retries, 1);
    }

    #[test]
    fn test_test_track_random() {
        let track = TestTrack::random();
        assert!(!track.title.is_empty());
        assert!(track.artist_name.is_some());
    }

    #[test]
    fn test_test_track_known() {
        let track = TestTrack::known();
        assert_eq!(track.title, "Bohemian Rhapsody");
        assert_eq!(track.artist_name, Some("Queen".to_string()));
    }

    #[test]
    fn test_test_track_minimal() {
        let track = TestTrack::minimal();
        assert!(track.artist_name.is_none());
        assert!(track.album_title.is_none());
        assert!(track.genres.is_empty());
    }

    #[test]
    fn test_test_artist_random() {
        let artist = TestArtist::random();
        assert!(!artist.name.is_empty());
        assert!(artist.lidarr_id > 0);
    }

    #[test]
    fn test_test_artist_known() {
        let artist = TestArtist::known();
        assert_eq!(artist.name, "Queen");
        assert_eq!(artist.lidarr_id, 42);
    }

    #[test]
    fn test_test_album_random() {
        let artist_id = Uuid::new_v4();
        let album = TestAlbum::random(artist_id);
        assert_eq!(album.artist_id, artist_id);
        assert!(!album.title.is_empty());
    }

    #[test]
    fn test_test_album_known() {
        let artist_id = Uuid::new_v4();
        let album = TestAlbum::known(artist_id);
        assert_eq!(album.title, "A Night at the Opera");
        assert_eq!(album.lidarr_id, 123);
    }

    #[test]
    fn test_create_temp_music_library() {
        let temp_dir = create_temp_music_library();
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_create_mock_audio_file() {
        let temp_dir = create_temp_music_library();
        let file_path = create_mock_audio_file(temp_dir.path(), "test.flac");
        assert!(file_path.exists());
        assert!(std::fs::read(&file_path).is_ok());
    }
}
