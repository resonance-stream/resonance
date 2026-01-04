//! Mock services for worker integration tests
//!
//! Provides mock implementations of external services (Ollama, Lidarr)
//! for testing worker jobs without network dependencies.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mock Ollama server for testing embedding and generation
pub struct MockOllamaServer {
    server: MockServer,
    embedding_call_count: Arc<AtomicUsize>,
    generate_call_count: Arc<AtomicUsize>,
    chat_call_count: Arc<AtomicUsize>,
}

impl MockOllamaServer {
    /// Start a new mock Ollama server
    pub async fn start() -> Self {
        let server = MockServer::start().await;
        let embedding_call_count = Arc::new(AtomicUsize::new(0));
        let generate_call_count = Arc::new(AtomicUsize::new(0));
        let chat_call_count = Arc::new(AtomicUsize::new(0));

        Self {
            server,
            embedding_call_count,
            generate_call_count,
            chat_call_count,
        }
    }

    /// Get the server URL
    pub fn url(&self) -> String {
        self.server.uri()
    }

    /// Mount a mock for successful embedding generation
    pub async fn mock_embeddings_success(&self) {
        // Generate a 768-dimensional embedding (nomic-embed-text dimension)
        let embedding: Vec<f32> = (0..768).map(|i| (i as f32 * 0.001) % 1.0).collect();

        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "embedding": embedding
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for embedding generation with custom embedding
    pub async fn mock_embeddings_with_value(&self, embedding: Vec<f32>) {
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "embedding": embedding
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for embedding generation failure
    pub async fn mock_embeddings_failure(&self, status_code: u16, error_message: &str) {
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(
                ResponseTemplate::new(status_code).set_body_json(json!({
                    "error": error_message
                })),
            )
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for model not found error
    pub async fn mock_embeddings_model_not_found(&self) {
        Mock::given(method("POST"))
            .and(path("/api/embeddings"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "error": "model 'nomic-embed-text' not found, try pulling it first"
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for successful text generation
    pub async fn mock_generate_success(&self, response_text: &str) {
        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "mistral",
                "response": response_text,
                "done": true
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for text generation failure
    pub async fn mock_generate_failure(&self, status_code: u16, error_message: &str) {
        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(
                ResponseTemplate::new(status_code).set_body_json(json!({
                    "error": error_message
                })),
            )
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for successful chat completion
    pub async fn mock_chat_success(&self, response_text: &str) {
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "mistral",
                "message": {
                    "role": "assistant",
                    "content": response_text
                },
                "done": true
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for mood analysis response
    pub async fn mock_mood_analysis(&self, moods: &[&str], tags: &[&str], description: &str) {
        let response = json!({
            "moods": moods,
            "tags": tags,
            "description": description,
            "energy": "medium",
            "valence": "positive"
        });

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "mistral",
                "message": {
                    "role": "assistant",
                    "content": serde_json::to_string(&response).unwrap()
                },
                "done": true
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for the /api/tags endpoint (list models)
    pub async fn mock_list_models(&self, models: &[&str]) {
        let model_list: Vec<serde_json::Value> = models
            .iter()
            .map(|name| {
                json!({
                    "name": name,
                    "modified_at": "2024-01-01T00:00:00Z",
                    "size": 4_000_000_000_i64
                })
            })
            .collect();

        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": model_list
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for connection timeout (delayed response)
    pub async fn mock_timeout(&self, delay_ms: u64) {
        Mock::given(method("POST"))
            .and(path_regex("/api/(embeddings|generate|chat)"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_millis(delay_ms))
                    .set_body_json(json!({"error": "timeout"})),
            )
            .mount(&self.server)
            .await;
    }

    /// Get embedding call count
    pub fn embedding_calls(&self) -> usize {
        self.embedding_call_count.load(Ordering::SeqCst)
    }

    /// Get generate call count
    pub fn generate_calls(&self) -> usize {
        self.generate_call_count.load(Ordering::SeqCst)
    }

    /// Get chat call count
    pub fn chat_calls(&self) -> usize {
        self.chat_call_count.load(Ordering::SeqCst)
    }

    /// Get reference to the underlying mock server for custom mock setups
    pub fn inner(&self) -> &MockServer {
        &self.server
    }

    /// Mount a mock for chat completion with custom response JSON
    pub async fn mock_chat_with_json(&self, response_json: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "mistral",
                "message": {
                    "role": "assistant",
                    "content": serde_json::to_string(&response_json).unwrap()
                },
                "done": true
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for chat completion failure
    pub async fn mock_chat_failure(&self, status_code: u16, error_message: &str) {
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(status_code).set_body_json(json!({
                    "error": error_message
                })),
            )
            .mount(&self.server)
            .await;
    }
}

/// Mock Lidarr server for testing Lidarr sync jobs
pub struct MockLidarrServer {
    server: MockServer,
    api_key: String,
}

impl MockLidarrServer {
    /// Start a new mock Lidarr server with default API key
    pub async fn start() -> Self {
        Self::start_with_api_key("test-api-key").await
    }

    /// Start a new mock Lidarr server with custom API key
    pub async fn start_with_api_key(api_key: &str) -> Self {
        let server = MockServer::start().await;
        Self {
            server,
            api_key: api_key.to_string(),
        }
    }

    /// Get the server URL
    pub fn url(&self) -> String {
        self.server.uri()
    }

    /// Get the API key
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Mount a mock for successful artist list
    pub async fn mock_artists_success(&self, artists: Vec<LidarrArtistFixture>) {
        let artists_json: Vec<serde_json::Value> = artists.into_iter().map(|a| a.to_json()).collect();

        Mock::given(method("GET"))
            .and(path("/api/v1/artist"))
            .and(header("X-Api-Key", self.api_key.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(artists_json))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for empty artist list
    pub async fn mock_artists_empty(&self) {
        Mock::given(method("GET"))
            .and(path("/api/v1/artist"))
            .and(header("X-Api-Key", self.api_key.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for successful album list
    pub async fn mock_albums_success(&self, albums: Vec<LidarrAlbumFixture>) {
        let albums_json: Vec<serde_json::Value> = albums.into_iter().map(|a| a.to_json()).collect();

        Mock::given(method("GET"))
            .and(path("/api/v1/album"))
            .and(header("X-Api-Key", self.api_key.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(albums_json))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for empty album list
    pub async fn mock_albums_empty(&self) {
        Mock::given(method("GET"))
            .and(path("/api/v1/album"))
            .and(header("X-Api-Key", self.api_key.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for authentication failure
    pub async fn mock_auth_failure(&self) {
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/(artist|album)"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "Unauthorized"
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for server error
    pub async fn mock_server_error(&self, error_message: &str) {
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/(artist|album)"))
            .and(header("X-Api-Key", self.api_key.as_str()))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "error": error_message
            })))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for rate limiting
    pub async fn mock_rate_limit(&self) {
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/(artist|album)"))
            .and(header("X-Api-Key", self.api_key.as_str()))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "60")
                    .set_body_json(json!({
                        "error": "Rate limit exceeded"
                    })),
            )
            .mount(&self.server)
            .await;
    }
}

/// Fixture for creating Lidarr artist responses
#[derive(Debug, Clone)]
pub struct LidarrArtistFixture {
    pub id: i64,
    pub artist_name: String,
    pub sort_name: Option<String>,
    pub overview: Option<String>,
    pub monitored: bool,
    pub path: Option<String>,
    pub foreign_artist_id: Option<String>,
    pub genres: Vec<String>,
    pub images: Vec<LidarrImageFixture>,
}

impl LidarrArtistFixture {
    /// Create a new monitored artist fixture
    pub fn monitored(id: i64, name: &str) -> Self {
        Self {
            id,
            artist_name: name.to_string(),
            sort_name: Some(name.to_string()),
            overview: Some(format!("Biography of {}", name)),
            monitored: true,
            path: Some(format!("/music/{}", name.to_lowercase().replace(' ', "_"))),
            foreign_artist_id: Some(uuid::Uuid::new_v4().to_string()),
            genres: vec!["Rock".to_string()],
            images: vec![LidarrImageFixture::poster()],
        }
    }

    /// Create an unmonitored artist fixture
    pub fn unmonitored(id: i64, name: &str) -> Self {
        let mut artist = Self::monitored(id, name);
        artist.monitored = false;
        artist
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "artistName": self.artist_name,
            "sortName": self.sort_name,
            "overview": self.overview,
            "monitored": self.monitored,
            "path": self.path,
            "foreignArtistId": self.foreign_artist_id,
            "genres": self.genres,
            "images": self.images.iter().map(|i| i.to_json()).collect::<Vec<_>>()
        })
    }
}

/// Fixture for creating Lidarr album responses
#[derive(Debug, Clone)]
pub struct LidarrAlbumFixture {
    pub id: i64,
    pub title: String,
    pub artist_id: i64,
    pub monitored: bool,
    pub foreign_album_id: Option<String>,
    pub release_date: Option<String>,
    pub genres: Vec<String>,
    pub album_type: Option<String>,
    pub statistics: LidarrAlbumStatisticsFixture,
}

impl LidarrAlbumFixture {
    /// Create a new album fixture with tracks on disk
    pub fn with_tracks(id: i64, title: &str, artist_id: i64, track_count: i32) -> Self {
        Self {
            id,
            title: title.to_string(),
            artist_id,
            monitored: true,
            foreign_album_id: Some(uuid::Uuid::new_v4().to_string()),
            release_date: Some("2024-01-15".to_string()),
            genres: vec!["Rock".to_string()],
            album_type: Some("album".to_string()),
            statistics: LidarrAlbumStatisticsFixture {
                total_track_count: track_count,
                track_file_count: track_count,
                size_on_disk: (track_count as i64) * 50_000_000,
                percent_of_tracks: 100.0,
            },
        }
    }

    /// Create an album fixture without tracks on disk
    pub fn without_tracks(id: i64, title: &str, artist_id: i64) -> Self {
        let mut album = Self::with_tracks(id, title, artist_id, 10);
        album.statistics.track_file_count = 0;
        album.statistics.size_on_disk = 0;
        album.statistics.percent_of_tracks = 0.0;
        album
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "title": self.title,
            "artistId": self.artist_id,
            "monitored": self.monitored,
            "foreignAlbumId": self.foreign_album_id,
            "releaseDate": self.release_date,
            "genres": self.genres,
            "albumType": self.album_type,
            "statistics": {
                "totalTrackCount": self.statistics.total_track_count,
                "trackFileCount": self.statistics.track_file_count,
                "sizeOnDisk": self.statistics.size_on_disk,
                "percentOfTracks": self.statistics.percent_of_tracks
            }
        })
    }
}

/// Fixture for Lidarr album statistics
#[derive(Debug, Clone)]
pub struct LidarrAlbumStatisticsFixture {
    pub total_track_count: i32,
    pub track_file_count: i32,
    pub size_on_disk: i64,
    pub percent_of_tracks: f64,
}

/// Fixture for Lidarr image responses
#[derive(Debug, Clone)]
pub struct LidarrImageFixture {
    pub cover_type: String,
    pub url: String,
}

impl LidarrImageFixture {
    /// Create a poster image fixture
    pub fn poster() -> Self {
        Self {
            cover_type: "poster".to_string(),
            url: "https://example.com/poster.jpg".to_string(),
        }
    }

    /// Create a fanart image fixture
    pub fn fanart() -> Self {
        Self {
            cover_type: "fanart".to_string(),
            url: "https://example.com/fanart.jpg".to_string(),
        }
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "coverType": self.cover_type,
            "url": self.url
        })
    }
}

/// Mock Redis client for testing prefetch cache operations
///
/// This struct provides an in-memory key-value store that mimics Redis
/// behavior for testing without requiring a real Redis server.
pub struct MockRedisStore {
    store: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, MockRedisEntry>>>,
}

/// Entry in the mock Redis store with expiration tracking
struct MockRedisEntry {
    value: String,
    expires_at: Option<std::time::Instant>,
}

impl MockRedisStore {
    /// Create a new mock Redis store
    pub fn new() -> Self {
        Self {
            store: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Set a key with expiration (SETEX equivalent)
    pub fn setex(&self, key: &str, seconds: i64, value: String) {
        let expires_at = if seconds > 0 {
            Some(std::time::Instant::now() + std::time::Duration::from_secs(seconds as u64))
        } else {
            None
        };

        let mut store = self.store.write().unwrap();
        store.insert(
            key.to_string(),
            MockRedisEntry { value, expires_at },
        );
    }

    /// Get a key value (GET equivalent)
    pub fn get(&self, key: &str) -> Option<String> {
        let store = self.store.read().unwrap();
        store.get(key).and_then(|entry| {
            if let Some(expires_at) = entry.expires_at {
                if std::time::Instant::now() > expires_at {
                    return None;
                }
            }
            Some(entry.value.clone())
        })
    }

    /// Delete a key (DEL equivalent)
    pub fn del(&self, key: &str) -> bool {
        let mut store = self.store.write().unwrap();
        store.remove(key).is_some()
    }

    /// Check if a key exists (EXISTS equivalent)
    pub fn exists(&self, key: &str) -> bool {
        let store = self.store.read().unwrap();
        if let Some(entry) = store.get(key) {
            if let Some(expires_at) = entry.expires_at {
                return std::time::Instant::now() <= expires_at;
            }
            return true;
        }
        false
    }

    /// Get all keys matching a pattern (KEYS equivalent, simplified)
    pub fn keys(&self, pattern: &str) -> Vec<String> {
        let store = self.store.read().unwrap();
        let pattern = pattern.replace("*", "");
        store
            .keys()
            .filter(|k| k.contains(&pattern))
            .cloned()
            .collect()
    }

    /// Clear all keys (FLUSHALL equivalent)
    pub fn flush_all(&self) {
        let mut store = self.store.write().unwrap();
        store.clear();
    }

    /// Get the number of keys in the store
    pub fn len(&self) -> usize {
        let store = self.store.read().unwrap();
        store.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for MockRedisStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MockRedisStore {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_ollama_server_starts() {
        let server = MockOllamaServer::start().await;
        assert!(!server.url().is_empty());
        assert!(server.url().starts_with("http://"));
    }

    #[tokio::test]
    async fn test_mock_ollama_embeddings() {
        let server = MockOllamaServer::start().await;
        server.mock_embeddings_success().await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/embeddings", server.url()))
            .json(&serde_json::json!({"model": "nomic-embed-text", "prompt": "test"}))
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());

        let body: serde_json::Value = response.json().await.unwrap();
        let embedding = body["embedding"].as_array().unwrap();
        assert_eq!(embedding.len(), 768);
    }

    #[tokio::test]
    async fn test_mock_lidarr_server_starts() {
        let server = MockLidarrServer::start().await;
        assert!(!server.url().is_empty());
        assert_eq!(server.api_key(), "test-api-key");
    }

    #[tokio::test]
    async fn test_mock_lidarr_artists() {
        let server = MockLidarrServer::start().await;
        let artists = vec![
            LidarrArtistFixture::monitored(1, "Queen"),
            LidarrArtistFixture::monitored(2, "The Beatles"),
        ];
        server.mock_artists_success(artists).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/api/v1/artist", server.url()))
            .header("X-Api-Key", server.api_key())
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());

        let body: Vec<serde_json::Value> = response.json().await.unwrap();
        assert_eq!(body.len(), 2);
        assert_eq!(body[0]["artistName"], "Queen");
    }

    #[tokio::test]
    async fn test_mock_lidarr_auth_failure() {
        let server = MockLidarrServer::start().await;
        server.mock_auth_failure().await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/api/v1/artist", server.url()))
            .header("X-Api-Key", "wrong-key")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), 401);
    }

    #[test]
    fn test_lidarr_artist_fixture_monitored() {
        let artist = LidarrArtistFixture::monitored(42, "Queen");
        assert_eq!(artist.id, 42);
        assert_eq!(artist.artist_name, "Queen");
        assert!(artist.monitored);
        assert!(artist.path.is_some());
    }

    #[test]
    fn test_lidarr_artist_fixture_unmonitored() {
        let artist = LidarrArtistFixture::unmonitored(42, "Queen");
        assert!(!artist.monitored);
    }

    #[test]
    fn test_lidarr_album_fixture_with_tracks() {
        let album = LidarrAlbumFixture::with_tracks(1, "A Night at the Opera", 42, 12);
        assert_eq!(album.statistics.track_file_count, 12);
        assert_eq!(album.statistics.percent_of_tracks, 100.0);
    }

    #[test]
    fn test_lidarr_album_fixture_without_tracks() {
        let album = LidarrAlbumFixture::without_tracks(1, "A Night at the Opera", 42);
        assert_eq!(album.statistics.track_file_count, 0);
        assert_eq!(album.statistics.percent_of_tracks, 0.0);
    }

    #[test]
    fn test_lidarr_artist_to_json() {
        let artist = LidarrArtistFixture::monitored(42, "Queen");
        let json = artist.to_json();

        assert_eq!(json["id"], 42);
        assert_eq!(json["artistName"], "Queen");
        assert_eq!(json["monitored"], true);
        assert!(json["images"].is_array());
    }

    #[test]
    fn test_lidarr_album_to_json() {
        let album = LidarrAlbumFixture::with_tracks(1, "A Night at the Opera", 42, 12);
        let json = album.to_json();

        assert_eq!(json["id"], 1);
        assert_eq!(json["title"], "A Night at the Opera");
        assert_eq!(json["artistId"], 42);
        assert_eq!(json["statistics"]["trackFileCount"], 12);
    }

    #[test]
    fn test_mock_redis_store_new() {
        let store = MockRedisStore::new();
        assert!(store.is_empty());
    }

    #[test]
    fn test_mock_redis_store_setex_and_get() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());

        assert!(store.exists("key1"));
        assert_eq!(store.get("key1"), Some("value1".to_string()));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_mock_redis_store_del() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());
        assert!(store.exists("key1"));

        assert!(store.del("key1"));
        assert!(!store.exists("key1"));
        assert_eq!(store.get("key1"), None);
    }

    #[test]
    fn test_mock_redis_store_keys() {
        let store = MockRedisStore::new();
        store.setex("prefetch:user1:track1", 3600, "data1".to_string());
        store.setex("prefetch:user1:track2", 3600, "data2".to_string());
        store.setex("other:key", 3600, "data3".to_string());

        let keys = store.keys("prefetch:user1");
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_mock_redis_store_flush_all() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());
        store.setex("key2", 3600, "value2".to_string());
        assert_eq!(store.len(), 2);

        store.flush_all();
        assert!(store.is_empty());
    }

    #[test]
    fn test_mock_redis_store_clone() {
        let store = MockRedisStore::new();
        store.setex("key1", 3600, "value1".to_string());

        let store2 = store.clone();
        assert_eq!(store2.get("key1"), Some("value1".to_string()));

        // Changes in one should reflect in the other (shared Arc)
        store2.setex("key2", 3600, "value2".to_string());
        assert!(store.exists("key2"));
    }
}
