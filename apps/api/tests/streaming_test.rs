//! Integration tests for audio streaming routes
//!
//! Tests the complete streaming lifecycle:
//! - Authentication (valid token, invalid token, missing token)
//! - Range request handling (full file, partial range, suffix range)
//! - ETag/caching (If-None-Match, If-Modified-Since)
//! - Transcoding options (format, bitrate validation)
//! - Path traversal security (../, absolute paths outside library)
//!
//! # Note
//!
//! These tests use mocks/test fixtures for the track repository and file system
//! to enable testing streaming behavior without a real database. For full
//! end-to-end integration tests that require database setup, see the API
//! integration test suite.
//!
//! # Running the tests
//! ```bash
//! cargo test --test streaming_test -p resonance-api
//! ```

mod common;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, Method, Request, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use resonance_api::error::{ApiError, ApiResult};
use resonance_api::models::{AudioFormat, Track};

// ========== Test Configuration ==========

/// Test track ID for an existing track
fn test_track_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
}

/// Test track ID for a non-existent track
fn nonexistent_track_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000999").unwrap()
}

// ========== Mock Track Repository ==========

/// Mock track repository for testing without database
#[derive(Clone)]
struct MockTrackRepository {
    tracks: Arc<RwLock<Vec<Track>>>,
}

impl MockTrackRepository {
    fn new() -> Self {
        Self {
            tracks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn add_track(&self, track: Track) {
        self.tracks.write().await.push(track);
    }

    async fn find_by_id(&self, track_id: Uuid) -> Result<Option<Track>, ApiError> {
        let tracks = self.tracks.read().await;
        Ok(tracks.iter().find(|t| t.id == track_id).cloned())
    }
}

/// Create a test track with the specified file path
fn create_test_track(id: Uuid, file_path: &str) -> Track {
    use resonance_api::models::track::AudioFeatures;

    Track {
        id,
        title: "Test Track".to_string(),
        album_id: Some(Uuid::new_v4()),
        artist_id: Uuid::new_v4(),
        mbid: None,
        file_path: file_path.to_string(),
        file_size: 1000,
        file_format: AudioFormat::Flac,
        file_hash: None,
        duration_ms: 180000,
        bit_rate: Some(1411),
        sample_rate: Some(44100),
        channels: Some(2),
        bit_depth: Some(16),
        track_number: Some(1),
        disc_number: Some(1),
        genres: vec!["Rock".to_string()],
        explicit: false,
        lyrics: None,
        synced_lyrics: None,
        audio_features: AudioFeatures::default(),
        ai_mood: vec![],
        ai_tags: vec![],
        ai_description: None,
        play_count: 0,
        skip_count: 0,
        last_played_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ========== Test App State ==========

#[derive(Clone)]
struct TestAppState {
    track_repo: MockTrackRepository,
    music_library_path: PathBuf,
}

// ========== Streaming Handler (simplified for testing) ==========

#[derive(Debug, Deserialize, Default)]
struct TranscodeQuery {
    format: Option<String>,
    bitrate: Option<u32>,
}

/// Check Authorization header and return error if invalid
fn validate_auth(headers: &axum::http::HeaderMap) -> ApiResult<()> {
    let auth_header = headers.get(header::AUTHORIZATION);

    match auth_header {
        Some(value) => {
            let header_str = value.to_str().map_err(|_| ApiError::Unauthorized)?;

            if !header_str.starts_with("Bearer ") {
                return Err(ApiError::Unauthorized);
            }

            let token = &header_str[7..];

            if token == "valid_token" {
                Ok(())
            } else if token == "expired_token" {
                Err(ApiError::InvalidToken("token has expired".to_string()))
            } else {
                Err(ApiError::InvalidToken("invalid token".to_string()))
            }
        }
        None => Err(ApiError::Unauthorized),
    }
}

/// Simplified streaming handler for testing
async fn stream_track(
    State(state): State<TestAppState>,
    Path(track_id): Path<Uuid>,
    Query(transcode_query): Query<TranscodeQuery>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Response> {
    // 1. Validate authentication
    validate_auth(&headers)?;

    // 2. Look up track
    let track = state
        .track_repo
        .find_by_id(track_id)
        .await?
        .ok_or_else(|| ApiError::not_found("track", track_id.to_string()))?;

    // 3. Validate and resolve file path
    let file_path = validate_file_path(&track.file_path, &state.music_library_path)?;

    // 4. Validate transcoding parameters
    if transcode_query.bitrate.is_some() && transcode_query.format.is_none() {
        return Err(ApiError::ValidationError(
            "`bitrate` requires `format` parameter".to_string(),
        ));
    }

    // 5. Handle transcoding request
    if let Some(format_str) = &transcode_query.format {
        // Reject Range requests for transcoding
        if headers.get(header::RANGE).is_some() {
            return Err(ApiError::InvalidRange(
                "Range requests not supported for transcoding".to_string(),
            ));
        }

        // Validate format
        let valid_formats = ["mp3", "aac", "opus", "flac"];
        if !valid_formats.contains(&format_str.as_str()) {
            return Err(ApiError::ValidationError(format!(
                "Unsupported format: {}",
                format_str
            )));
        }

        // Validate bitrate if provided
        if let Some(bitrate) = transcode_query.bitrate {
            let valid_bitrates = [64, 96, 128, 192, 256, 320];
            if !valid_bitrates.contains(&bitrate) {
                return Err(ApiError::ValidationError(format!(
                    "Unsupported bitrate: {}",
                    bitrate
                )));
            }
        }

        // Return mock transcoded response
        let content_type = match format_str.as_str() {
            "mp3" => "audio/mpeg",
            "aac" => "audio/aac",
            "opus" => "audio/opus",
            "flac" => "audio/flac",
            _ => "application/octet-stream",
        };

        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::ACCEPT_RANGES, "none")
            .header(header::CACHE_CONTROL, "private, no-store")
            .body(Body::from("transcoded_audio_data"))
            .unwrap());
    }

    // 6. Open file and get metadata
    let file_content = tokio::fs::read(&file_path).await.map_err(|e| {
        tracing::error!(error = %e, path = %file_path.display(), "Failed to read audio file");
        ApiError::AudioFileNotFound(track.file_path.clone())
    })?;

    let file_size = file_content.len() as u64;
    let content_type = "audio/flac";

    // 7. Generate caching headers
    let metadata = tokio::fs::metadata(&file_path)
        .await
        .map_err(|e| ApiError::AudioProcessing(format!("Failed to read file metadata: {}", e)))?;
    let modified = metadata
        .modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let etag = generate_etag(file_size, modified);
    let last_modified = httpdate::fmt_http_date(modified);

    // 8. Check for conditional request (304 Not Modified)
    if is_cache_valid(&headers, &etag, modified) {
        return Ok(Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, &etag)
            .header(header::LAST_MODIFIED, &last_modified)
            .header(
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable",
            )
            .body(Body::empty())
            .unwrap());
    }

    // 9. Handle range request
    let range_header = headers.get(header::RANGE).and_then(|h| h.to_str().ok());

    match range_header {
        Some(range) => {
            let (start, end) = parse_range_header(range, file_size)?;
            let content_length = end - start + 1;

            // Safely convert u64 range bounds to usize with checked conversions
            let start_usize = usize::try_from(start).map_err(|_| {
                ApiError::InvalidRange("Range start exceeds platform limits".to_string())
            })?;
            let end_usize = usize::try_from(end).map_err(|_| {
                ApiError::InvalidRange("Range end exceeds platform limits".to_string())
            })?;

            // Bounds check against actual file content length
            if end_usize >= file_content.len() {
                return Err(ApiError::RangeNotSatisfiable { file_size });
            }

            let range_content = file_content[start_usize..=end_usize].to_vec();

            Ok(Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CONTENT_LENGTH, content_length)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {}-{}/{}", start, end, file_size),
                )
                .header(header::ETAG, &etag)
                .header(header::LAST_MODIFIED, &last_modified)
                .header(
                    header::CACHE_CONTROL,
                    "private, max-age=31536000, immutable",
                )
                .body(Body::from(range_content))
                .unwrap())
        }
        None => Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, file_size)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::ETAG, &etag)
            .header(header::LAST_MODIFIED, &last_modified)
            .header(
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable",
            )
            .body(Body::from(file_content))
            .unwrap()),
    }
}

/// HEAD request handler
async fn head_track(
    State(state): State<TestAppState>,
    Path(track_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Response> {
    // 1. Validate authentication
    validate_auth(&headers)?;

    // 2. Look up track
    let track = state
        .track_repo
        .find_by_id(track_id)
        .await?
        .ok_or_else(|| ApiError::not_found("track", track_id.to_string()))?;

    // 3. Validate and resolve file path
    let file_path = validate_file_path(&track.file_path, &state.music_library_path)?;

    // 4. Get file metadata
    let metadata = tokio::fs::metadata(&file_path).await.map_err(|e| {
        tracing::error!(error = %e, path = %file_path.display(), "Failed to read audio file metadata");
        ApiError::AudioFileNotFound(track.file_path.clone())
    })?;

    let file_size = metadata.len();
    let content_type = "audio/flac";
    let modified = metadata
        .modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let etag = generate_etag(file_size, modified);
    let last_modified = httpdate::fmt_http_date(modified);

    // 5. Check for conditional request
    if is_cache_valid(&headers, &etag, modified) {
        return Ok(Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, &etag)
            .header(header::LAST_MODIFIED, &last_modified)
            .header(
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable",
            )
            .body(Body::empty())
            .unwrap());
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, file_size)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ETAG, &etag)
        .header(header::LAST_MODIFIED, &last_modified)
        .header(
            header::CACHE_CONTROL,
            "private, max-age=31536000, immutable",
        )
        .body(Body::empty())
        .unwrap())
}

// ========== Helper Functions ==========

fn validate_file_path(file_path: &str, music_library_path: &std::path::Path) -> ApiResult<PathBuf> {
    let input_path = std::path::Path::new(file_path);

    // Reject any parent-dir components in relative paths
    if !input_path.is_absolute()
        && input_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ApiError::Forbidden("Access denied".to_string()));
    }

    let full_path = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        music_library_path.join(input_path)
    };

    // Canonicalize to resolve symlinks and check boundaries
    let canonical = full_path
        .canonicalize()
        .map_err(|_| ApiError::AudioFileNotFound(file_path.to_string()))?;

    let canonical_library = music_library_path
        .canonicalize()
        .map_err(|e| ApiError::AudioProcessing(format!("Invalid music library path: {}", e)))?;

    if !canonical.starts_with(&canonical_library) {
        return Err(ApiError::Forbidden("Access denied".to_string()));
    }

    Ok(canonical)
}

fn generate_etag(file_size: u64, modified: std::time::SystemTime) -> String {
    let mtime_secs = modified
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("\"{}-{}\"", file_size, mtime_secs)
}

/// Check if the client's cached version is still valid
///
/// Per RFC 7232 Section 6:
/// - If-None-Match takes precedence over If-Modified-Since
/// - If If-None-Match is present (even if malformed), If-Modified-Since is ignored
fn is_cache_valid(
    headers: &axum::http::HeaderMap,
    etag: &str,
    modified: std::time::SystemTime,
) -> bool {
    // Check If-None-Match (takes precedence over If-Modified-Since per RFC 7232)
    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
        // RFC 7232: If If-None-Match is present, it takes precedence and
        // If-Modified-Since must be ignored. A malformed If-None-Match header
        // means the condition cannot be evaluated, so return false (cache invalid).
        let Ok(value) = if_none_match.to_str() else {
            return false;
        };
        // Handle both single value and comma-separated list
        return value.split(',').any(|v| {
            let v = v.trim();
            // RFC 7232: Weak comparison - strip "W/" prefix if present
            let v_trimmed = v.strip_prefix("W/").unwrap_or(v);
            let etag_trimmed = etag.strip_prefix("W/").unwrap_or(etag);
            v_trimmed == etag_trimmed || v == "*"
        });
    }

    // Check If-Modified-Since (only if If-None-Match is not present)
    if let Some(if_modified_since) = headers.get(header::IF_MODIFIED_SINCE) {
        if let Ok(value) = if_modified_since.to_str() {
            if let Ok(if_modified_since_time) = httpdate::parse_http_date(value) {
                let now = std::time::SystemTime::now();
                if if_modified_since_time > now {
                    return false;
                }
                if let Ok(modified_secs) =
                    modified.duration_since(std::time::SystemTime::UNIX_EPOCH)
                {
                    let modified_truncated = std::time::SystemTime::UNIX_EPOCH
                        + std::time::Duration::from_secs(modified_secs.as_secs());
                    return modified_truncated <= if_modified_since_time;
                }
            }
        }
    }

    false
}

fn parse_range_header(range_header: &str, file_size: u64) -> Result<(u64, u64), ApiError> {
    let range_header = range_header.trim();

    if !range_header.starts_with("bytes=") {
        return Err(ApiError::InvalidRange("Invalid range unit".to_string()));
    }

    let range_spec = &range_header[6..];

    if range_spec.contains(',') {
        return Err(ApiError::InvalidRange(
            "Multiple ranges not supported".to_string(),
        ));
    }

    let parts: Vec<&str> = range_spec.split('-').collect();
    if parts.len() != 2 {
        return Err(ApiError::InvalidRange("Invalid range format".to_string()));
    }

    let (start, end) = match (parts[0].is_empty(), parts[1].is_empty()) {
        (false, false) => {
            let start: u64 = parts[0]
                .parse()
                .map_err(|_| ApiError::InvalidRange("Invalid start position".to_string()))?;
            let end: u64 = parts[1]
                .parse()
                .map_err(|_| ApiError::InvalidRange("Invalid end position".to_string()))?;
            (start, end)
        }
        (false, true) => {
            let start: u64 = parts[0]
                .parse()
                .map_err(|_| ApiError::InvalidRange("Invalid start position".to_string()))?;
            (start, file_size.saturating_sub(1))
        }
        (true, false) => {
            let suffix_length: u64 = parts[1]
                .parse()
                .map_err(|_| ApiError::InvalidRange("Invalid suffix length".to_string()))?;
            if suffix_length >= file_size {
                (0, file_size.saturating_sub(1))
            } else {
                (file_size - suffix_length, file_size.saturating_sub(1))
            }
        }
        (true, true) => {
            return Err(ApiError::InvalidRange("Empty range".to_string()));
        }
    };

    if start >= file_size {
        return Err(ApiError::RangeNotSatisfiable { file_size });
    }

    let end = end.min(file_size.saturating_sub(1));

    if start > end {
        return Err(ApiError::InvalidRange(
            "Start position greater than end".to_string(),
        ));
    }

    Ok((start, end))
}

// ========== Test Fixtures ==========

/// Create a test app with the streaming routes
fn create_test_app(state: TestAppState) -> Router {
    Router::new()
        .route("/:track_id", get(stream_track).head(head_track))
        .with_state(state)
}

/// Create test state with temp directory and track repo
async fn create_test_state() -> (TestAppState, TempDir, MockTrackRepository) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let track_repo = MockTrackRepository::new();

    let state = TestAppState {
        track_repo: track_repo.clone(),
        music_library_path: temp_dir.path().to_path_buf(),
    };

    (state, temp_dir, track_repo)
}

/// Create a test audio file in the temp directory
fn create_test_audio_file(temp_dir: &TempDir, filename: &str, content: &[u8]) -> PathBuf {
    let file_path = temp_dir.path().join(filename);
    let mut file = std::fs::File::create(&file_path).expect("Failed to create test file");
    file.write_all(content).expect("Failed to write test file");
    file_path
}

/// Parse response body as bytes
async fn get_body_bytes(response: Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
}

/// Parse response body as JSON
async fn parse_body<T: for<'de> Deserialize<'de>>(response: Response) -> T {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

// ========== Authentication Tests ==========

#[tokio::test]
async fn test_stream_requires_authentication() {
    let (state, _temp_dir, _track_repo) = create_test_state().await;
    let app = create_test_app(state);

    // Request without Authorization header
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_stream_rejects_invalid_token() {
    let (state, _temp_dir, _track_repo) = create_test_state().await;
    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer invalid_token_here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: Value = parse_body(response).await;
    assert_eq!(body["code"], "INVALID_TOKEN");
}

#[tokio::test]
async fn test_stream_rejects_expired_token() {
    let (state, _temp_dir, _track_repo) = create_test_state().await;
    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer expired_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: Value = parse_body(response).await;
    assert_eq!(body["code"], "INVALID_TOKEN");
    assert!(body["message"].as_str().unwrap().contains("expired"));
}

// ========== Range Request Tests ==========

#[tokio::test]
async fn test_stream_full_file_without_range() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    // Create test file
    let audio_content = b"FAKE_FLAC_AUDIO_DATA_FOR_TESTING_1234567890";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    // Add track to mock repo
    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "audio/flac"
    );
    assert_eq!(
        response.headers().get(header::ACCEPT_RANGES).unwrap(),
        "bytes"
    );
    assert!(response.headers().get(header::CONTENT_LENGTH).is_some());
    assert!(response.headers().get(header::ETAG).is_some());

    let body = get_body_bytes(response).await;
    assert_eq!(body, audio_content);
}

#[tokio::test]
async fn test_stream_partial_range_request() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    // Create test file with known content
    let audio_content = b"0123456789ABCDEFGHIJ";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .header(header::RANGE, "bytes=5-14")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        response.headers().get(header::CONTENT_RANGE).unwrap(),
        "bytes 5-14/20"
    );
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap(),
        "10"
    );

    let body = get_body_bytes(response).await;
    assert_eq!(body, b"56789ABCDE");
}

#[tokio::test]
async fn test_stream_suffix_range_request() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"0123456789ABCDEFGHIJ";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    // Request last 5 bytes
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .header(header::RANGE, "bytes=-5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);

    let body = get_body_bytes(response).await;
    assert_eq!(body, b"FGHIJ");
}

#[tokio::test]
async fn test_stream_open_end_range_request() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"0123456789ABCDEFGHIJ";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    // Request from byte 15 to end
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .header(header::RANGE, "bytes=15-")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);

    let body = get_body_bytes(response).await;
    assert_eq!(body, b"FGHIJ");
}

#[tokio::test]
async fn test_stream_range_beyond_file_size() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"0123456789";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    // Request range starting beyond file size
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .header(header::RANGE, "bytes=100-200")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);

    // Check Content-Range header per RFC 7233
    let content_range = response
        .headers()
        .get(header::CONTENT_RANGE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_range.contains("bytes */10"));
}

// ========== ETag/Caching Tests ==========

#[tokio::test]
async fn test_stream_returns_etag_and_last_modified() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"test_audio_content";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(header::ETAG).is_some());
    assert!(response.headers().get(header::LAST_MODIFIED).is_some());
    assert!(response.headers().get(header::CACHE_CONTROL).is_some());

    let cache_control = response
        .headers()
        .get(header::CACHE_CONTROL)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cache_control.contains("private"));
    assert!(cache_control.contains("immutable"));
}

#[tokio::test]
async fn test_stream_if_none_match_returns_304() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"test_audio_content";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state.clone());

    // First request to get the ETag
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let etag = response
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Second request with If-None-Match
    let app2 = create_test_app(state);
    let response = app2
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);

    // Body should be empty for 304
    let body = get_body_bytes(response).await;
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_head_request_returns_metadata_without_body() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"test_audio_content_for_head_request";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::HEAD)
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(header::CONTENT_TYPE).is_some());
    assert!(response.headers().get(header::CONTENT_LENGTH).is_some());
    assert!(response.headers().get(header::ETAG).is_some());
    assert!(response.headers().get(header::ACCEPT_RANGES).is_some());

    // Body should be empty for HEAD
    let body = get_body_bytes(response).await;
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_head_if_none_match_returns_304() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"test_audio_content";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state.clone());

    // First HEAD request to get ETag
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::HEAD)
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let etag = response
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Second HEAD request with If-None-Match
    let app2 = create_test_app(state);
    let response = app2
        .oneshot(
            Request::builder()
                .method(Method::HEAD)
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
}

// ========== Transcoding Tests ==========

#[tokio::test]
async fn test_transcode_requires_format_for_bitrate() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"test_audio";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    // Request with bitrate but no format
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}?bitrate=192", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = parse_body(response).await;
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_transcode_rejects_invalid_format() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"test_audio";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}?format=invalid_format", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = parse_body(response).await;
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["message"].as_str().unwrap().contains("format"));
}

#[tokio::test]
async fn test_transcode_rejects_range_requests() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    let audio_content = b"test_audio";
    create_test_audio_file(&temp_dir, "test.flac", audio_content);

    let track = create_test_track(test_track_id(), "test.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    // Request transcoding with Range header
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}?format=mp3", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .header(header::RANGE, "bytes=0-1000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = parse_body(response).await;
    assert_eq!(body["code"], "INVALID_RANGE");
    assert!(body["message"].as_str().unwrap().contains("not supported"));
}

// ========== Path Traversal Security Tests ==========

#[tokio::test]
async fn test_path_traversal_blocked_relative() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    // Create a file outside the music library (in parent dir)
    let parent_dir = temp_dir.path().parent().unwrap();
    let secret_file = parent_dir.join("secret.txt");
    std::fs::write(&secret_file, b"SECRET_DATA").ok();

    // Try to access file via path traversal
    let track = create_test_track(test_track_id(), "../secret.txt");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body: Value = parse_body(response).await;
    assert_eq!(body["code"], "FORBIDDEN");

    // Cleanup
    std::fs::remove_file(&secret_file).ok();
}

#[tokio::test]
async fn test_path_traversal_blocked_nested() {
    let (state, temp_dir, track_repo) = create_test_state().await;

    // Create a subdir and try nested traversal
    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();

    // Try nested path traversal
    let track = create_test_track(test_track_id(), "subdir/../../etc/passwd");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body: Value = parse_body(response).await;
    assert_eq!(body["code"], "FORBIDDEN");
}

#[tokio::test]
async fn test_absolute_path_outside_library_blocked() {
    let (state, _temp_dir, track_repo) = create_test_state().await;

    // Create a file outside the library
    let outside_file = std::env::temp_dir().join("outside_library_test.txt");
    std::fs::write(&outside_file, b"SECRET").ok();

    // Try to access via absolute path
    let track = create_test_track(test_track_id(), outside_file.to_str().unwrap());
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should be forbidden OR not found (depending on whether it's inside library)
    // The key is it should NOT return the file content
    assert!(
        response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::NOT_FOUND
    );

    // Cleanup
    std::fs::remove_file(&outside_file).ok();
}

// ========== Track Not Found Tests ==========

#[tokio::test]
async fn test_stream_nonexistent_track_returns_404() {
    let (state, _temp_dir, _track_repo) = create_test_state().await;
    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", nonexistent_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body: Value = parse_body(response).await;
    assert_eq!(body["code"], "NOT_FOUND");
}

#[tokio::test]
async fn test_stream_missing_audio_file_returns_404() {
    let (state, _temp_dir, track_repo) = create_test_state().await;

    // Add track but don't create the file
    let track = create_test_track(test_track_id(), "nonexistent.flac");
    track_repo.add_track(track).await;

    let app = create_test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/{}", test_track_id()))
                .header(header::AUTHORIZATION, "Bearer valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body: Value = parse_body(response).await;
    assert_eq!(body["code"], "AUDIO_NOT_FOUND");
}
