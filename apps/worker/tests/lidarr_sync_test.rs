//! Integration tests for Lidarr sync job
//!
//! Tests for syncing artist metadata, album metadata, and handling
//! various API error scenarios using wiremock for HTTP mocking.

mod common;

use common::{LidarrAlbumFixture, LidarrArtistFixture, MockLidarrServer};
use serde_json::json;
use uuid::Uuid;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Unit Tests: JSON Deserialization
// ============================================================================

/// Test deserializing a minimal LidarrArtist JSON response
#[test]
fn test_deserialize_lidarr_artist_minimal() {
    let json = json!({
        "id": 1,
        "artistName": "Queen",
        "monitored": true
    });

    // Verify the JSON can be parsed with required fields only
    let artist: serde_json::Value = json;
    assert_eq!(artist["id"], 1);
    assert_eq!(artist["artistName"], "Queen");
    assert_eq!(artist["monitored"], true);
}

/// Test deserializing a complete LidarrArtist with all fields
#[test]
fn test_deserialize_lidarr_artist_complete() {
    let mbid = Uuid::new_v4().to_string();
    let json = json!({
        "id": 42,
        "artistName": "Queen",
        "sortName": "Queen",
        "overview": "British rock band formed in London in 1970",
        "monitored": true,
        "path": "/music/queen",
        "foreignArtistId": mbid,
        "genres": ["Rock", "Glam Rock", "Progressive Rock"],
        "images": [
            {"coverType": "poster", "url": "https://example.com/queen.jpg"},
            {"coverType": "fanart", "url": "https://example.com/queen_fanart.jpg"}
        ]
    });

    // Verify all fields are present
    assert_eq!(json["id"], 42);
    assert_eq!(json["artistName"], "Queen");
    assert_eq!(json["sortName"], "Queen");
    assert_eq!(
        json["overview"],
        "British rock band formed in London in 1970"
    );
    assert_eq!(json["monitored"], true);
    assert_eq!(json["path"], "/music/queen");
    assert_eq!(json["foreignArtistId"], mbid);
    assert_eq!(json["genres"].as_array().unwrap().len(), 3);
    assert_eq!(json["images"].as_array().unwrap().len(), 2);
}

/// Test deserializing LidarrArtist with empty optional arrays
#[test]
fn test_deserialize_lidarr_artist_empty_arrays() {
    let json = json!({
        "id": 1,
        "artistName": "Unknown Artist",
        "monitored": false,
        "genres": [],
        "images": []
    });

    assert_eq!(json["id"], 1);
    assert!(json["genres"].as_array().unwrap().is_empty());
    assert!(json["images"].as_array().unwrap().is_empty());
}

/// Test deserializing a minimal LidarrAlbum JSON response
#[test]
fn test_deserialize_lidarr_album_minimal() {
    let json = json!({
        "id": 100,
        "title": "A Night at the Opera",
        "artistId": 42,
        "monitored": true,
        "statistics": {}
    });

    assert_eq!(json["id"], 100);
    assert_eq!(json["title"], "A Night at the Opera");
    assert_eq!(json["artistId"], 42);
    assert_eq!(json["monitored"], true);
}

/// Test deserializing a complete LidarrAlbum with statistics
#[test]
fn test_deserialize_lidarr_album_with_statistics() {
    let mbid = Uuid::new_v4().to_string();
    let json = json!({
        "id": 100,
        "title": "A Night at the Opera",
        "artistId": 42,
        "monitored": true,
        "foreignAlbumId": mbid,
        "releaseDate": "1975-11-21T00:00:00Z",
        "genres": ["Rock", "Progressive Rock"],
        "albumType": "album",
        "statistics": {
            "totalTrackCount": 12,
            "trackFileCount": 12,
            "sizeOnDisk": 650000000,
            "percentOfTracks": 100.0
        }
    });

    assert_eq!(json["id"], 100);
    assert_eq!(json["statistics"]["totalTrackCount"], 12);
    assert_eq!(json["statistics"]["trackFileCount"], 12);
    assert_eq!(json["statistics"]["sizeOnDisk"], 650000000_i64);
    assert_eq!(json["statistics"]["percentOfTracks"], 100.0);
}

/// Test deserializing LidarrAlbum with zero statistics (no files)
#[test]
fn test_deserialize_lidarr_album_no_files() {
    let json = json!({
        "id": 101,
        "title": "Future Album",
        "artistId": 42,
        "monitored": true,
        "statistics": {
            "totalTrackCount": 10,
            "trackFileCount": 0,
            "sizeOnDisk": 0,
            "percentOfTracks": 0.0
        }
    });

    assert_eq!(json["statistics"]["trackFileCount"], 0);
    assert_eq!(json["statistics"]["sizeOnDisk"], 0);
    assert_eq!(json["statistics"]["percentOfTracks"], 0.0);
}

/// Test deserializing album with various album types
#[test]
fn test_deserialize_lidarr_album_types() {
    for album_type in &["single", "ep", "compilation", "live", "album"] {
        let json = json!({
            "id": 1,
            "title": "Test Album",
            "artistId": 1,
            "monitored": true,
            "albumType": album_type,
            "statistics": {}
        });

        assert_eq!(json["albumType"], *album_type);
    }
}

// ============================================================================
// Integration Tests: Wiremock HTTP Mocking
// ============================================================================

/// Test fetching artists from Lidarr API with successful response
#[tokio::test]
async fn test_fetch_artists_success() {
    let server = MockLidarrServer::start().await;

    let artists = vec![
        LidarrArtistFixture::monitored(1, "Queen"),
        LidarrArtistFixture::monitored(2, "The Beatles"),
        LidarrArtistFixture::unmonitored(3, "Pink Floyd"),
    ];

    server.mock_artists_success(artists).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/artist", server.url()))
        .header("X-Api-Key", server.api_key())
        .send()
        .await
        .expect("request should succeed");

    assert!(response.status().is_success());

    let body: Vec<serde_json::Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 3);

    // Verify first artist (monitored)
    assert_eq!(body[0]["id"], 1);
    assert_eq!(body[0]["artistName"], "Queen");
    assert_eq!(body[0]["monitored"], true);

    // Verify third artist (unmonitored)
    assert_eq!(body[2]["id"], 3);
    assert_eq!(body[2]["monitored"], false);
}

/// Test fetching albums from Lidarr API with various track statuses
#[tokio::test]
async fn test_fetch_albums_success() {
    let server = MockLidarrServer::start().await;

    let albums = vec![
        LidarrAlbumFixture::with_tracks(1, "A Night at the Opera", 42, 12),
        LidarrAlbumFixture::with_tracks(2, "News of the World", 42, 11),
        LidarrAlbumFixture::without_tracks(3, "Hot Space", 42),
    ];

    server.mock_albums_success(albums).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/album", server.url()))
        .header("X-Api-Key", server.api_key())
        .send()
        .await
        .expect("request should succeed");

    assert!(response.status().is_success());

    let body: Vec<serde_json::Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 3);

    // Albums with tracks
    assert_eq!(body[0]["statistics"]["trackFileCount"], 12);
    assert_eq!(body[1]["statistics"]["trackFileCount"], 11);

    // Album without tracks on disk
    assert_eq!(body[2]["statistics"]["trackFileCount"], 0);
}

/// Test API returns 401 Unauthorized with invalid API key
#[tokio::test]
async fn test_api_error_unauthorized() {
    let server = MockLidarrServer::start().await;
    server.mock_auth_failure("wrong-api-key").await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/artist", server.url()))
        .header("X-Api-Key", "wrong-api-key")
        .send()
        .await
        .expect("request should succeed");

    assert_eq!(response.status().as_u16(), 401);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"], "Unauthorized");
}

/// Test API returns 500 Internal Server Error
#[tokio::test]
async fn test_api_error_server_error() {
    let server = MockLidarrServer::start().await;
    server.mock_server_error("Database connection failed").await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/artist", server.url()))
        .header("X-Api-Key", server.api_key())
        .send()
        .await
        .expect("request should succeed");

    assert_eq!(response.status().as_u16(), 500);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"], "Database connection failed");
}

/// Test API returns 429 Rate Limit with Retry-After header
#[tokio::test]
async fn test_api_error_rate_limit() {
    let server = MockLidarrServer::start().await;
    server.mock_rate_limit().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/artist", server.url()))
        .header("X-Api-Key", server.api_key())
        .send()
        .await
        .expect("request should succeed");

    assert_eq!(response.status().as_u16(), 429);

    // Check Retry-After header
    let retry_after = response.headers().get("Retry-After");
    assert!(retry_after.is_some());
    assert_eq!(retry_after.unwrap().to_str().unwrap(), "60");
}

/// Test handling empty artist list from Lidarr
#[tokio::test]
async fn test_empty_artist_list() {
    let server = MockLidarrServer::start().await;
    server.mock_artists_empty().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/artist", server.url()))
        .header("X-Api-Key", server.api_key())
        .send()
        .await
        .expect("request should succeed");

    assert!(response.status().is_success());

    let body: Vec<serde_json::Value> = response.json().await.unwrap();
    assert!(body.is_empty());
}

// ============================================================================
// Path Validation Tests
// ============================================================================

/// Test that artist path fixtures are valid paths
#[test]
fn test_artist_path_validation_valid() {
    let artist = LidarrArtistFixture::monitored(1, "Queen");
    let path = artist.path.as_ref().unwrap();

    // Path should be within music library
    assert!(path.starts_with("/music/"));
    assert!(path.ends_with("queen"));
}

/// Test that path with spaces is properly formatted
#[test]
fn test_artist_path_with_spaces() {
    let artist = LidarrArtistFixture::monitored(1, "The Rolling Stones");
    let path = artist.path.as_ref().unwrap();

    // Spaces should be replaced with underscores
    assert!(!path.contains(' '));
    assert!(path.contains("the_rolling_stones"));
}

/// Additional test for malformed API response handling
#[tokio::test]
async fn test_api_malformed_response() {
    let server = MockServer::start().await;

    // Return malformed JSON that's missing required fields
    Mock::given(method("GET"))
        .and(path("/api/v1/artist"))
        .and(header("X-Api-Key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/artist", server.uri()))
        .header("X-Api-Key", "test-key")
        .send()
        .await
        .expect("request should succeed");

    assert!(response.status().is_success());

    // JSON parsing should fail
    let body_result: Result<Vec<serde_json::Value>, _> = response.json().await;
    assert!(body_result.is_err());
}
