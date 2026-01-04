//! Mock Lidarr server for testing library sync jobs
//!
//! Provides a [`MockLidarrServer`] that simulates Lidarr API endpoints
//! for testing music library synchronization without a real Lidarr instance.

use serde_json::json;
use wiremock::matchers::{header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mock Lidarr server for testing Lidarr sync jobs
///
/// This struct wraps a [`wiremock::MockServer`] and provides convenience methods
/// for setting up common Lidarr API responses including artists, albums, and error scenarios.
///
/// # Example
///
/// ```rust,ignore
/// use resonance_test_utils::{MockLidarrServer, LidarrArtistFixture};
///
/// #[tokio::test]
/// async fn test_lidarr_sync() {
///     let server = MockLidarrServer::start().await;
///     let artists = vec![LidarrArtistFixture::monitored(1, "Queen")];
///     server.mock_artists_success(artists).await;
///
///     // Configure your Lidarr client with server.url() and server.api_key()
/// }
/// ```
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
        let artists_json: Vec<serde_json::Value> =
            artists.into_iter().map(|a| a.to_json()).collect();

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

    /// Mount a mock for authentication failure with a specific bad API key
    ///
    /// This mock only matches requests that use the specified invalid API key,
    /// preventing interference with other mocks that use the valid API key.
    ///
    /// # Arguments
    ///
    /// * `bad_api_key` - The specific invalid API key to match against
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let server = MockLidarrServer::start().await;
    /// server.mock_auth_failure("wrong-api-key").await;
    ///
    /// // This request will get 401 Unauthorized
    /// client.get("/api/v1/artist").header("X-Api-Key", "wrong-api-key").send().await;
    ///
    /// // This request would NOT match the auth failure mock
    /// client.get("/api/v1/artist").header("X-Api-Key", server.api_key()).send().await;
    /// ```
    pub async fn mock_auth_failure(&self, bad_api_key: &str) {
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/(artist|album)"))
            .and(header("X-Api-Key", bad_api_key))
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

#[cfg(test)]
mod tests {
    use super::*;

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
        server.mock_auth_failure("wrong-key").await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/api/v1/artist", server.url()))
            .header("X-Api-Key", "wrong-key")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), 401);
    }

    #[tokio::test]
    async fn test_mock_lidarr_auth_failure_does_not_interfere_with_valid_key() {
        let server = MockLidarrServer::start().await;
        let artists = vec![LidarrArtistFixture::monitored(1, "Queen")];

        // Set up both auth failure for bad key AND success for valid key
        server.mock_auth_failure("wrong-key").await;
        server.mock_artists_success(artists).await;

        let client = reqwest::Client::new();

        // Request with valid key should succeed
        let valid_response = client
            .get(format!("{}/api/v1/artist", server.url()))
            .header("X-Api-Key", server.api_key())
            .send()
            .await
            .unwrap();
        assert!(valid_response.status().is_success());

        // Request with invalid key should get 401
        let invalid_response = client
            .get(format!("{}/api/v1/artist", server.url()))
            .header("X-Api-Key", "wrong-key")
            .send()
            .await
            .unwrap();
        assert_eq!(invalid_response.status().as_u16(), 401);
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
}
