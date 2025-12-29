//! Audio streaming HTTP route handlers
//!
//! This module provides endpoints for streaming audio files:
//! - `GET /stream/:track_id` - Stream audio file with HTTP range request support
//! - `HEAD /stream/:track_id` - Get file metadata without body
//!
//! Features:
//! - RFC 7233 compliant range request handling
//! - Path traversal prevention
//! - Async streaming without loading entire file into memory
//! - ETag and Last-Modified caching headers
//! - Conditional request support (If-None-Match, If-Modified-Since)

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use serde::Deserialize;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthUser;
use crate::models::AudioFormat;
use crate::repositories::TrackRepository;
use crate::services::transcoder::TranscodeError;
use crate::services::{TranscodeFormat, TranscodeOptions, TranscoderService};

/// Query parameters for transcoding options
#[derive(Debug, Deserialize, Default)]
pub struct TranscodeQuery {
    /// Target audio format (mp3, aac, opus, flac)
    /// If not specified, streams the original file without transcoding
    pub format: Option<String>,
    /// Target bitrate in kbps (64, 96, 128, 192, 256, 320)
    /// If not specified, uses the format's default bitrate
    pub bitrate: Option<u32>,
}

/// Shared application state for streaming handlers
#[derive(Clone)]
pub struct StreamingState {
    /// Track repository for database lookups
    pub track_repo: Arc<TrackRepository>,
    /// Base path to the music library
    pub music_library_path: PathBuf,
    /// Transcoder service for on-the-fly format conversion
    pub transcoder: TranscoderService,
}

impl StreamingState {
    /// Create a new StreamingState instance
    pub fn new(track_repo: TrackRepository, music_library_path: PathBuf) -> Self {
        Self {
            track_repo: Arc::new(track_repo),
            music_library_path,
            transcoder: TranscoderService::new(),
        }
    }
}

/// Create the streaming router
///
/// # Routes
/// - `GET /:track_id` - Stream audio file for a track
/// - `HEAD /:track_id` - Get file metadata without streaming body
pub fn streaming_router(state: StreamingState) -> Router {
    Router::new()
        .route("/{track_id}", get(stream_track).head(head_track))
        .with_state(state)
}

/// Stream audio file for a track
///
/// # Request
/// - Method: GET
/// - Path: /stream/:track_id
/// - Query Parameters:
///   - format: Target format (mp3, aac, opus, flac) - optional, for transcoding
///   - bitrate: Target bitrate in kbps (64, 96, 128, 192, 256, 320) - optional
/// - Headers:
///   - Authorization: Bearer <token> (required)
///   - Range: bytes=START-END (optional, for seeking - not supported with transcoding)
///   - If-None-Match: <etag> (optional, for caching)
///   - If-Modified-Since: <date> (optional, for caching)
///
/// # Response
/// - 200 OK: Full audio file stream (or transcoded stream)
/// - 206 Partial Content: Partial file for range requests (passthrough only)
/// - 304 Not Modified: Cache is still valid
/// - 401 Unauthorized: Missing or invalid token
/// - 404 Not Found: Track or audio file not found
/// - 416 Range Not Satisfiable: Invalid byte range
async fn stream_track(
    State(state): State<StreamingState>,
    _auth: AuthUser, // Validates authentication
    Path(track_id): Path<Uuid>,
    Query(transcode_query): Query<TranscodeQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    // 1. Look up track in database
    let track = state
        .track_repo
        .find_by_id(track_id)
        .await?
        .ok_or_else(|| ApiError::not_found("track", track_id.to_string()))?;

    // 2. Validate and resolve file path
    let file_path = validate_file_path(&track.file_path, &state.music_library_path).await?;

    // 3. Validate transcoding parameters
    if transcode_query.bitrate.is_some() && transcode_query.format.is_none() {
        return Err(ApiError::ValidationError(
            "`bitrate` requires `format` parameter".to_string(),
        ));
    }

    // 4. Check if transcoding is requested
    if let Some(format_str) = &transcode_query.format {
        // Reject Range requests for transcoding - we can't seek in a live-transcoded stream
        if headers.get(header::RANGE).is_some() {
            return Err(ApiError::InvalidRange(
                "Range requests not supported for transcoding".to_string(),
            ));
        }

        // Parse the target format
        let target_format = TranscodeFormat::parse(format_str).ok_or_else(|| {
            ApiError::ValidationError(format!("Unsupported format: {}", format_str))
        })?;

        // Build transcode options
        let options = match transcode_query.bitrate {
            Some(bitrate) => TranscodeOptions::with_bitrate(target_format, bitrate)
                .map_err(|e| ApiError::ValidationError(e.to_string()))?,
            None => TranscodeOptions::new(target_format),
        };

        // Start transcoding
        let transcode_stream = state
            .transcoder
            .transcode(&file_path, &options)
            .await
            .map_err(|e| {
                match &e {
                    TranscodeError::ResourceExhausted => {
                        // Return 503 Service Unavailable when at capacity
                        tracing::warn!(error = %e, "Transcoding at capacity");
                        ApiError::ServiceBusy("Transcoding capacity reached, try again later".to_string())
                    }
                    TranscodeError::FfmpegNotFound => {
                        tracing::error!(error = %e, "FFmpeg not available");
                        ApiError::Configuration("FFmpeg not installed".to_string())
                    }
                    _ => {
                        tracing::error!(error = %e, path = %file_path.display(), "Transcoding failed");
                        ApiError::AudioProcessing(format!("Transcoding failed: {}", e))
                    }
                }
            })?;

        let content_type = target_format.content_type();
        let body = Body::from_stream(transcode_stream);

        // Transcoded streams don't support range requests or Content-Length
        // (we don't know the final size until transcoding completes)
        // Note: Transfer-Encoding: chunked is implicit when streaming without Content-Length
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::ACCEPT_RANGES, "none") // Inform clients seeking is not supported
            .header(
                header::CACHE_CONTROL,
                "private, no-store", // Don't cache transcoded content
            )
            .body(body)
            .expect("Failed to build response"));
    }

    // 4. Passthrough: Open file and get metadata
    let file = File::open(&file_path).await.map_err(|e| {
        tracing::error!(error = %e, path = %file_path.display(), "Failed to open audio file");
        ApiError::AudioFileNotFound(track.file_path.clone())
    })?;

    let metadata = file
        .metadata()
        .await
        .map_err(|e| ApiError::AudioProcessing(format!("Failed to read file metadata: {}", e)))?;

    let file_size = metadata.len();
    let content_type = content_type_for_format(&track.file_format);

    // 5. Get modification time and generate caching headers
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let etag = generate_etag(file_size, modified);
    let last_modified = format_http_date(modified);

    // 6. Check for conditional request (304 Not Modified)
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
            .expect("Failed to build response"));
    }

    // 7. Handle range request
    let range_header = headers.get(header::RANGE).and_then(|h| h.to_str().ok());

    match range_header {
        Some(range) => {
            // Partial content
            let (start, end) = parse_range_header(range, file_size)?;
            let content_length = end - start + 1;

            // Seek to start position
            let mut file = file;
            file.seek(SeekFrom::Start(start))
                .await
                .map_err(|e| ApiError::AudioProcessing(format!("Failed to seek: {}", e)))?;

            // Take only the bytes we need
            let limited_file = file.take(content_length);
            let stream = ReaderStream::new(limited_file);
            let body = Body::from_stream(stream);

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
                .body(body)
                .expect("Failed to build response"))
        }
        None => {
            // Full content
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

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
                .body(body)
                .expect("Failed to build response"))
        }
    }
}

/// Get audio file metadata without streaming body (HEAD request)
///
/// # Request
/// - Method: HEAD
/// - Path: /stream/:track_id
/// - Headers:
///   - Authorization: Bearer <token> (required)
///   - If-None-Match: <etag> (optional, for caching)
///   - If-Modified-Since: <date> (optional, for caching)
///
/// # Response
/// - 200 OK: Headers with file metadata
/// - 304 Not Modified: Cache is still valid
/// - 401 Unauthorized: Missing or invalid token
/// - 404 Not Found: Track or audio file not found
async fn head_track(
    State(state): State<StreamingState>,
    _auth: AuthUser, // Validates authentication
    Path(track_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    // 1. Look up track in database
    let track = state
        .track_repo
        .find_by_id(track_id)
        .await?
        .ok_or_else(|| ApiError::not_found("track", track_id.to_string()))?;

    // 2. Validate and resolve file path
    let file_path = validate_file_path(&track.file_path, &state.music_library_path).await?;

    // 3. Get file metadata without opening the file for streaming
    let metadata = tokio::fs::metadata(&file_path).await.map_err(|e| {
        tracing::error!(error = %e, path = %file_path.display(), "Failed to read audio file metadata");
        ApiError::AudioFileNotFound(track.file_path.clone())
    })?;

    let file_size = metadata.len();
    let content_type = content_type_for_format(&track.file_format);

    // 4. Generate caching headers
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let etag = generate_etag(file_size, modified);
    let last_modified = format_http_date(modified);

    // 5. Check for conditional request (304 Not Modified)
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
            .expect("Failed to build response"));
    }

    // 6. Return headers only (no body for HEAD)
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
        .expect("Failed to build response"))
}

/// Parse HTTP Range header according to RFC 7233
///
/// Supports formats:
/// - `bytes=START-END` (specific range)
/// - `bytes=START-` (from start to end of file)
/// - `bytes=-SUFFIX` (last N bytes)
fn parse_range_header(range_header: &str, file_size: u64) -> Result<(u64, u64), ApiError> {
    let range_header = range_header.trim();

    if !range_header.starts_with("bytes=") {
        return Err(ApiError::InvalidRange("Invalid range unit".to_string()));
    }

    let range_spec = &range_header[6..];

    // We only support single ranges
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
        // bytes=500-999
        (false, false) => {
            let start: u64 = parts[0]
                .parse()
                .map_err(|_| ApiError::InvalidRange("Invalid start position".to_string()))?;
            let end: u64 = parts[1]
                .parse()
                .map_err(|_| ApiError::InvalidRange("Invalid end position".to_string()))?;
            (start, end)
        }
        // bytes=500- (from 500 to end)
        (false, true) => {
            let start: u64 = parts[0]
                .parse()
                .map_err(|_| ApiError::InvalidRange("Invalid start position".to_string()))?;
            (start, file_size.saturating_sub(1))
        }
        // bytes=-500 (last 500 bytes)
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
        // bytes=- (invalid)
        (true, true) => {
            return Err(ApiError::InvalidRange("Empty range".to_string()));
        }
    };

    // Validate range - check file boundaries first
    if start >= file_size {
        // RFC 7233: Return 416 Range Not Satisfiable when range is beyond file size
        return Err(ApiError::RangeNotSatisfiable { file_size });
    }

    // Clamp end to file size
    let end = end.min(file_size.saturating_sub(1));

    // After clamping, verify the range is valid
    if start > end {
        return Err(ApiError::InvalidRange(
            "Start position greater than end".to_string(),
        ));
    }

    Ok((start, end))
}

/// Get the Content-Type MIME type for an audio format
fn content_type_for_format(format: &AudioFormat) -> &'static str {
    match format {
        AudioFormat::Flac => "audio/flac",
        AudioFormat::Mp3 => "audio/mpeg",
        AudioFormat::Aac => "audio/aac",
        AudioFormat::Opus => "audio/opus",
        AudioFormat::Ogg => "audio/ogg",
        AudioFormat::Wav => "audio/wav",
        AudioFormat::Alac => "audio/mp4",
        AudioFormat::Other => "application/octet-stream",
    }
}

/// Generate an ETag from file metadata
///
/// Uses file size and modification time to create a unique identifier.
/// Format: `"{size}-{mtime_secs}"`
fn generate_etag(file_size: u64, modified: SystemTime) -> String {
    let mtime_secs = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("\"{}-{}\"", file_size, mtime_secs)
}

/// Format a SystemTime as an HTTP-date for Last-Modified header
///
/// Format: RFC 7231 (e.g., "Sun, 06 Nov 1994 08:49:37 GMT")
fn format_http_date(time: SystemTime) -> String {
    httpdate::fmt_http_date(time)
}

/// Check if the client's cached version is still valid
fn is_cache_valid(headers: &HeaderMap, etag: &str, modified: SystemTime) -> bool {
    // Check If-None-Match (takes precedence over If-Modified-Since per RFC 7232)
    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
        if let Ok(value) = if_none_match.to_str() {
            // Handle both single value and comma-separated list
            return value.split(',').any(|v| {
                let v = v.trim();
                // RFC 7232: Weak comparison - strip "W/" prefix if present
                let v_trimmed = v.strip_prefix("W/").unwrap_or(v);
                let etag_trimmed = etag.strip_prefix("W/").unwrap_or(etag);
                v_trimmed == etag_trimmed || v == "*"
            });
        }
    }

    // Check If-Modified-Since
    if let Some(if_modified_since) = headers.get(header::IF_MODIFIED_SINCE) {
        if let Ok(value) = if_modified_since.to_str() {
            // Parse HTTP date (supports RFC 1123, RFC 850, and asctime formats)
            if let Ok(if_modified_since_time) = httpdate::parse_http_date(value) {
                // Ignore dates in the future to avoid incorrect 304 responses
                let now = SystemTime::now();
                if if_modified_since_time > now {
                    return false;
                }

                // HTTP dates have second precision, so we truncate the file's modification time
                if let Ok(modified_secs) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                    let modified_truncated = SystemTime::UNIX_EPOCH
                        + std::time::Duration::from_secs(modified_secs.as_secs());
                    return modified_truncated <= if_modified_since_time;
                }
            }
        }
    }

    false
}

/// Validate that a file path is within the music library directory
///
/// This prevents path traversal attacks by:
/// 1. Canonicalizing the file path to resolve any `..` components
/// 2. Verifying the canonical path starts with the library path
///
/// Uses spawn_blocking to avoid blocking the async runtime during filesystem operations.
async fn validate_file_path(file_path: &str, music_library_path: &StdPath) -> ApiResult<PathBuf> {
    let file_path = file_path.to_string();
    let library = music_library_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        // Construct the full path - handle both absolute and relative paths
        let input_path = StdPath::new(&file_path);

        // Reject any parent-dir components in relative paths early to avoid
        // existence probing via different error messages (file-not-found vs forbidden)
        if !input_path.is_absolute()
            && input_path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            tracing::warn!(file_path = %file_path, "Path traversal attempt blocked (contains ..)");
            return Err(ApiError::Forbidden("Access denied".to_string()));
        }

        let full_path = if input_path.is_absolute() {
            input_path.to_path_buf()
        } else {
            library.join(input_path)
        };

        // Canonicalize to resolve any .., symlinks, etc.
        let canonical = full_path.canonicalize().map_err(|_| {
            tracing::warn!(file_path = %file_path, "Audio file not found or inaccessible");
            ApiError::AudioFileNotFound(file_path.to_string())
        })?;

        // Canonicalize the library path as well
        let canonical_library = library.canonicalize().map_err(|e| {
            tracing::error!(error = %e, path = %library.display(), "Invalid music library path");
            ApiError::AudioProcessing(format!("Invalid music library path: {}", e))
        })?;

        // Verify the canonical path starts with the library path
        if !canonical.starts_with(&canonical_library) {
            tracing::warn!(
                file_path = %file_path,
                canonical = %canonical.display(),
                library = %canonical_library.display(),
                "Path traversal attempt blocked"
            );
            return Err(ApiError::Forbidden("Access denied".to_string()));
        }

        Ok(canonical)
    })
    .await
    .map_err(|e| ApiError::Internal(format!("Path validation task failed: {}", e)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_mapping() {
        assert_eq!(content_type_for_format(&AudioFormat::Flac), "audio/flac");
        assert_eq!(content_type_for_format(&AudioFormat::Mp3), "audio/mpeg");
        assert_eq!(content_type_for_format(&AudioFormat::Aac), "audio/aac");
        assert_eq!(content_type_for_format(&AudioFormat::Opus), "audio/opus");
        assert_eq!(content_type_for_format(&AudioFormat::Ogg), "audio/ogg");
        assert_eq!(content_type_for_format(&AudioFormat::Wav), "audio/wav");
        assert_eq!(content_type_for_format(&AudioFormat::Alac), "audio/mp4");
        assert_eq!(
            content_type_for_format(&AudioFormat::Other),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_parse_range_header_full_range() {
        let (start, end) = parse_range_header("bytes=0-999", 5000).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 999);
    }

    #[test]
    fn test_parse_range_header_open_end() {
        let (start, end) = parse_range_header("bytes=500-", 5000).unwrap();
        assert_eq!(start, 500);
        assert_eq!(end, 4999);
    }

    #[test]
    fn test_parse_range_header_suffix() {
        let (start, end) = parse_range_header("bytes=-500", 5000).unwrap();
        assert_eq!(start, 4500);
        assert_eq!(end, 4999);
    }

    #[test]
    fn test_parse_range_header_suffix_larger_than_file() {
        let (start, end) = parse_range_header("bytes=-6000", 5000).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 4999);
    }

    #[test]
    fn test_parse_range_header_clamps_end_to_file_size() {
        let (start, end) = parse_range_header("bytes=0-10000", 5000).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 4999);
    }

    #[test]
    fn test_parse_range_header_invalid_unit() {
        let result = parse_range_header("chunks=0-100", 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_range_header_invalid_start_greater_than_end() {
        let result = parse_range_header("bytes=1000-500", 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_range_header_start_beyond_file_size() {
        let result = parse_range_header("bytes=6000-", 5000);
        match result {
            Err(ApiError::RangeNotSatisfiable { file_size }) => {
                assert_eq!(file_size, 5000);
            }
            Err(other) => panic!("Expected RangeNotSatisfiable, got: {:?}", other),
            Ok(v) => panic!("Expected error, got: {:?}", v),
        }
    }

    #[test]
    fn test_parse_range_header_empty_range() {
        let result = parse_range_header("bytes=-", 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_range_header_multiple_ranges_unsupported() {
        let result = parse_range_header("bytes=0-100, 200-300", 5000);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_range_header_with_whitespace() {
        let (start, end) = parse_range_header("  bytes=0-999  ", 5000).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 999);
    }

    // ========== ETag and Caching Tests ==========

    #[test]
    fn test_generate_etag_format() {
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1234567890);
        let etag = generate_etag(12345, modified);
        assert_eq!(etag, "\"12345-1234567890\"");
    }

    #[test]
    fn test_generate_etag_different_sizes() {
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let etag1 = generate_etag(100, modified);
        let etag2 = generate_etag(200, modified);
        assert_ne!(etag1, etag2);
    }

    #[test]
    fn test_generate_etag_different_times() {
        let modified1 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let modified2 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(2000);
        let etag1 = generate_etag(100, modified1);
        let etag2 = generate_etag(100, modified2);
        assert_ne!(etag1, etag2);
    }

    #[test]
    fn test_format_http_date() {
        // Unix epoch
        let date = format_http_date(SystemTime::UNIX_EPOCH);
        assert_eq!(date, "Thu, 01 Jan 1970 00:00:00 GMT");
    }

    #[test]
    fn test_is_cache_valid_with_matching_etag() {
        let mut headers = HeaderMap::new();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let etag = "\"12345-1000\"";
        headers.insert(header::IF_NONE_MATCH, etag.parse().unwrap());

        assert!(is_cache_valid(&headers, etag, modified));
    }

    #[test]
    fn test_is_cache_valid_with_non_matching_etag() {
        let mut headers = HeaderMap::new();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        headers.insert(header::IF_NONE_MATCH, "\"wrong-etag\"".parse().unwrap());

        assert!(!is_cache_valid(&headers, "\"12345-1000\"", modified));
    }

    #[test]
    fn test_is_cache_valid_with_wildcard_etag() {
        let mut headers = HeaderMap::new();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        headers.insert(header::IF_NONE_MATCH, "*".parse().unwrap());

        assert!(is_cache_valid(&headers, "\"any-etag\"", modified));
    }

    #[test]
    fn test_is_cache_valid_weak_etag_client() {
        // Client sends weak ETag (W/"..."), server has strong ETag
        let mut headers = HeaderMap::new();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let server_etag = "\"12345-1000\"";
        headers.insert(header::IF_NONE_MATCH, "W/\"12345-1000\"".parse().unwrap());

        // Should match after stripping W/ prefix
        assert!(is_cache_valid(&headers, server_etag, modified));
    }

    #[test]
    fn test_is_cache_valid_weak_etag_server() {
        // Server has weak ETag, client sends strong ETag
        let mut headers = HeaderMap::new();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let server_etag = "W/\"12345-1000\"";
        headers.insert(header::IF_NONE_MATCH, "\"12345-1000\"".parse().unwrap());

        // Should match after stripping W/ prefix from server
        assert!(is_cache_valid(&headers, server_etag, modified));
    }

    #[test]
    fn test_is_cache_valid_weak_etag_both() {
        // Both client and server have weak ETags
        let mut headers = HeaderMap::new();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let server_etag = "W/\"12345-1000\"";
        headers.insert(header::IF_NONE_MATCH, "W/\"12345-1000\"".parse().unwrap());

        // Should match
        assert!(is_cache_valid(&headers, server_etag, modified));
    }

    #[test]
    fn test_is_cache_valid_weak_etag_in_list() {
        // Client sends comma-separated list with weak ETag
        let mut headers = HeaderMap::new();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        let server_etag = "\"12345-1000\"";
        headers.insert(
            header::IF_NONE_MATCH,
            "\"wrong-etag\", W/\"12345-1000\", \"other\""
                .parse()
                .unwrap(),
        );

        // Should find match in the list
        assert!(is_cache_valid(&headers, server_etag, modified));
    }

    #[test]
    fn test_is_cache_valid_no_headers() {
        let headers = HeaderMap::new();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);

        assert!(!is_cache_valid(&headers, "\"12345-1000\"", modified));
    }

    #[test]
    fn test_is_cache_valid_future_date_rejected() {
        // If-Modified-Since date in the future should be rejected
        let mut headers = HeaderMap::new();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);

        // Use a date far in the future (year 2100)
        headers.insert(
            header::IF_MODIFIED_SINCE,
            "Sun, 01 Jan 2100 00:00:00 GMT".parse().unwrap(),
        );

        // Should return false because the date is in the future
        assert!(!is_cache_valid(&headers, "\"12345-1000\"", modified));
    }

    // ========== validate_file_path Tests ==========

    #[tokio::test]
    async fn test_validate_file_path_relative_path_valid() {
        // Use a real temporary directory for testing
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_audio_file.flac");

        // Create a test file
        std::fs::write(&test_file, b"test content").unwrap();

        let result = validate_file_path("test_audio_file.flac", &temp_dir).await;
        assert!(result.is_ok());

        // Cleanup
        std::fs::remove_file(&test_file).ok();
    }

    #[tokio::test]
    async fn test_validate_file_path_nonexistent_file() {
        let temp_dir = std::env::temp_dir();
        let result = validate_file_path("nonexistent_file.flac", &temp_dir).await;

        assert!(matches!(result, Err(ApiError::AudioFileNotFound(_))));
    }

    #[tokio::test]
    async fn test_validate_file_path_traversal_blocked() {
        // Create a file outside the library path and try to access it via traversal
        let temp_dir = std::env::temp_dir();
        let library_subdir = temp_dir.join("music_library_test");
        std::fs::create_dir_all(&library_subdir).unwrap();

        // Create a file in temp_dir (parent of library_subdir)
        let outside_file = temp_dir.join("outside_library.txt");
        std::fs::write(&outside_file, b"secret content").unwrap();

        // Try to access it via path traversal
        let result = validate_file_path("../outside_library.txt", &library_subdir).await;

        // Should be blocked as Forbidden
        assert!(matches!(result, Err(ApiError::Forbidden(_))));

        // Cleanup
        std::fs::remove_file(&outside_file).ok();
        std::fs::remove_dir(&library_subdir).ok();
    }

    #[tokio::test]
    async fn test_validate_file_path_absolute_path_inside_library() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("absolute_test_file.flac");
        std::fs::write(&test_file, b"test content").unwrap();

        // Use absolute path
        let result = validate_file_path(test_file.to_str().unwrap(), &temp_dir).await;
        assert!(result.is_ok());

        // Cleanup
        std::fs::remove_file(&test_file).ok();
    }

    #[tokio::test]
    async fn test_validate_file_path_absolute_path_outside_library() {
        let temp_dir = std::env::temp_dir();
        let library_subdir = temp_dir.join("music_library_test_2");
        std::fs::create_dir_all(&library_subdir).unwrap();

        // Create a file in temp_dir (parent of library_subdir)
        let outside_file = temp_dir.join("outside_file_absolute.txt");
        std::fs::write(&outside_file, b"secret content").unwrap();

        // Try to access it via absolute path
        let result = validate_file_path(outside_file.to_str().unwrap(), &library_subdir).await;

        // Should be blocked as Forbidden
        assert!(matches!(result, Err(ApiError::Forbidden(_))));

        // Cleanup
        std::fs::remove_file(&outside_file).ok();
        std::fs::remove_dir(&library_subdir).ok();
    }

    #[tokio::test]
    async fn test_validate_file_path_traversal_nonexistent_returns_forbidden() {
        // Test that non-existent files with traversal components return Forbidden
        // (not NotFound), to prevent existence probing attacks
        let temp_dir = std::env::temp_dir();
        let library_subdir = temp_dir.join("music_library_probe_test");
        std::fs::create_dir_all(&library_subdir).unwrap();

        // Try to access a non-existent file with traversal components
        // Should return Forbidden, not NotFound, to prevent existence probing
        let result = validate_file_path("../nonexistent_file.txt", &library_subdir).await;
        assert!(matches!(result, Err(ApiError::Forbidden(_))));

        // Also test nested traversal
        let result2 = validate_file_path("subdir/../../../secret.txt", &library_subdir).await;
        assert!(matches!(result2, Err(ApiError::Forbidden(_))));

        // Cleanup
        std::fs::remove_dir(&library_subdir).ok();
    }
}
