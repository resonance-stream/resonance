//! Comprehensive error handling for the Resonance API
//!
//! This module provides a unified error type hierarchy using thiserror,
//! with automatic HTTP status code mapping via Axum's IntoResponse trait.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// API error response body
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error code for client-side handling
    pub code: &'static str,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Main API error type with comprehensive error variants
#[derive(Error, Debug)]
pub enum ApiError {
    // ========== Authentication & Authorization ==========
    /// Invalid or missing authentication credentials
    #[error("authentication required")]
    Unauthorized,

    /// Invalid token (expired, malformed, etc.)
    #[error("invalid authentication token: {0}")]
    InvalidToken(String),

    /// User lacks permission for the requested operation
    #[error("insufficient permissions: {0}")]
    Forbidden(String),

    // ========== Resource Errors ==========
    /// Requested resource not found
    #[error("{resource_type} not found: {id}")]
    NotFound {
        resource_type: &'static str,
        id: String,
    },

    /// Resource already exists (conflict)
    #[error("{resource_type} already exists: {id}")]
    Conflict {
        resource_type: &'static str,
        id: String,
    },

    // ========== Validation Errors ==========
    /// Request validation failed
    #[error("validation error: {0}")]
    ValidationError(String),

    /// Invalid request body format
    #[error("invalid request body: {0}")]
    InvalidBody(String),

    /// Missing required field
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// Invalid query parameter
    #[error("invalid query parameter '{name}': {reason}")]
    InvalidQueryParam { name: &'static str, reason: String },

    // ========== Database Errors ==========
    /// Database query failed
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Database connection pool exhausted
    #[error("database connection unavailable")]
    DatabaseUnavailable,

    /// Service temporarily unavailable (at capacity)
    #[error("service temporarily unavailable: {0}")]
    ServiceBusy(String),

    // ========== External Service Errors ==========
    /// Redis operation failed
    #[error("cache error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Meilisearch operation failed
    #[error("search service error: {0}")]
    Search(String),

    /// Ollama AI service error
    #[error("AI service error: {0}")]
    AiService(String),

    /// Lidarr integration error
    #[error("Lidarr integration error: {0}")]
    Lidarr(String),

    /// Last.fm integration error
    #[error("Last.fm error: {0}")]
    Lastfm(String),

    /// ListenBrainz integration error
    #[error("ListenBrainz error: {0}")]
    ListenBrainz(String),

    /// HTTP client error (for external API calls)
    #[error("external service error: {0}")]
    HttpClient(#[from] reqwest::Error),

    // ========== Audio/Streaming Errors ==========
    /// Audio file not found or inaccessible
    #[error("audio file not found: {0}")]
    AudioFileNotFound(String),

    /// Audio transcoding/processing failed
    #[error("audio processing error: {0}")]
    AudioProcessing(String),

    /// Invalid audio format
    #[error("unsupported audio format: {0}")]
    UnsupportedFormat(String),

    /// Range request invalid (400 Bad Request)
    #[error("invalid range request: {0}")]
    InvalidRange(String),

    /// Range not satisfiable (416 Range Not Satisfiable)
    #[error("range not satisfiable")]
    RangeNotSatisfiable { file_size: u64 },

    // ========== Rate Limiting ==========
    /// Rate limit exceeded
    #[error("rate limit exceeded, retry after {retry_after} seconds")]
    RateLimited { retry_after: u64 },

    // ========== Configuration Errors ==========
    /// Configuration error
    #[error("configuration error: {0}")]
    Configuration(String),

    // ========== Internal Errors ==========
    /// Internal server error (catch-all for unexpected errors)
    #[error("internal server error: {0}")]
    Internal(String),

    /// JSON serialization/deserialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// WebSocket error
    #[error("websocket error: {0}")]
    WebSocket(String),

    /// JWT encoding/decoding error
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl ApiError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 401 Unauthorized
            Self::Unauthorized | Self::InvalidToken(_) => StatusCode::UNAUTHORIZED,

            // 403 Forbidden
            Self::Forbidden(_) => StatusCode::FORBIDDEN,

            // 404 Not Found
            Self::NotFound { .. } | Self::AudioFileNotFound(_) => StatusCode::NOT_FOUND,

            // 409 Conflict
            Self::Conflict { .. } => StatusCode::CONFLICT,

            // 400 Bad Request
            Self::ValidationError(_)
            | Self::InvalidBody(_)
            | Self::MissingField(_)
            | Self::InvalidQueryParam { .. }
            | Self::InvalidRange(_)
            | Self::UnsupportedFormat(_) => StatusCode::BAD_REQUEST,

            // 416 Range Not Satisfiable
            Self::RangeNotSatisfiable { .. } => StatusCode::RANGE_NOT_SATISFIABLE,

            // 422 Unprocessable Entity
            Self::Serialization(_) => StatusCode::UNPROCESSABLE_ENTITY,

            // 429 Too Many Requests
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,

            // 503 Service Unavailable
            Self::DatabaseUnavailable | Self::ServiceBusy(_) => StatusCode::SERVICE_UNAVAILABLE,

            // 502 Bad Gateway (external service errors)
            Self::Search(_)
            | Self::AiService(_)
            | Self::Lidarr(_)
            | Self::Lastfm(_)
            | Self::ListenBrainz(_)
            | Self::HttpClient(_) => StatusCode::BAD_GATEWAY,

            // 500 Internal Server Error
            Self::Database(_)
            | Self::Redis(_)
            | Self::AudioProcessing(_)
            | Self::Configuration(_)
            | Self::Internal(_)
            | Self::WebSocket(_)
            | Self::Jwt(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error code string for client-side handling
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Unauthorized => "UNAUTHORIZED",
            Self::InvalidToken(_) => "INVALID_TOKEN",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Conflict { .. } => "CONFLICT",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::InvalidBody(_) => "INVALID_BODY",
            Self::MissingField(_) => "MISSING_FIELD",
            Self::InvalidQueryParam { .. } => "INVALID_QUERY_PARAM",
            Self::Database(_) => "DATABASE_ERROR",
            Self::DatabaseUnavailable => "DATABASE_UNAVAILABLE",
            Self::ServiceBusy(_) => "SERVICE_BUSY",
            Self::Redis(_) => "CACHE_ERROR",
            Self::Search(_) => "SEARCH_ERROR",
            Self::AiService(_) => "AI_SERVICE_ERROR",
            Self::Lidarr(_) => "LIDARR_ERROR",
            Self::Lastfm(_) => "LASTFM_ERROR",
            Self::ListenBrainz(_) => "LISTENBRAINZ_ERROR",
            Self::HttpClient(_) => "EXTERNAL_SERVICE_ERROR",
            Self::AudioFileNotFound(_) => "AUDIO_NOT_FOUND",
            Self::AudioProcessing(_) => "AUDIO_PROCESSING_ERROR",
            Self::UnsupportedFormat(_) => "UNSUPPORTED_FORMAT",
            Self::InvalidRange(_) => "INVALID_RANGE",
            Self::RangeNotSatisfiable { .. } => "RANGE_NOT_SATISFIABLE",
            Self::RateLimited { .. } => "RATE_LIMITED",
            Self::Configuration(_) => "CONFIGURATION_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Serialization(_) => "SERIALIZATION_ERROR",
            Self::WebSocket(_) => "WEBSOCKET_ERROR",
            Self::Jwt(_) => "JWT_ERROR",
        }
    }

    /// Create a not found error for a specific resource
    pub fn not_found(resource_type: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type,
            id: id.into(),
        }
    }

    /// Create a conflict error for a specific resource
    pub fn conflict(resource_type: &'static str, id: impl Into<String>) -> Self {
        Self::Conflict {
            resource_type,
            id: id.into(),
        }
    }

    /// Log the error with appropriate severity based on status code
    pub fn log(&self) {
        let status = self.status_code();
        if status.is_server_error() {
            tracing::error!(
                error = %self,
                code = self.error_code(),
                status = status.as_u16(),
                "Server error occurred"
            );
        } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            tracing::warn!(
                error = %self,
                code = self.error_code(),
                status = status.as_u16(),
                "Authorization error"
            );
        } else {
            tracing::debug!(
                error = %self,
                code = self.error_code(),
                status = status.as_u16(),
                "Client error"
            );
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Log the error
        self.log();

        let status = self.status_code();
        let error_response = ErrorResponse {
            code: self.error_code(),
            message: self.to_string(),
            details: None,
        };

        // For rate limiting, add Retry-After header
        if let Self::RateLimited { retry_after } = &self {
            return (
                status,
                [("Retry-After", retry_after.to_string())],
                Json(error_response),
            )
                .into_response();
        }

        // For range not satisfiable, add Content-Range header per RFC 7233
        if let Self::RangeNotSatisfiable { file_size } = &self {
            return (
                status,
                [("Content-Range", format!("bytes */{}", file_size))],
                Json(error_response),
            )
                .into_response();
        }

        (status, Json(error_response)).into_response()
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;

// ========== Conversion Implementations ==========

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        // Try to downcast to ApiError first
        match err.downcast::<ApiError>() {
            Ok(api_err) => api_err,
            Err(err) => Self::Internal(err.to_string()),
        }
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::AudioFileNotFound(err.to_string()),
            std::io::ErrorKind::PermissionDenied => {
                Self::Forbidden("file access denied".to_string())
            }
            _ => Self::AudioProcessing(err.to_string()),
        }
    }
}

impl From<std::env::VarError> for ApiError {
    fn from(err: std::env::VarError) -> Self {
        Self::Configuration(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes() {
        assert_eq!(
            ApiError::Unauthorized.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ApiError::not_found("track", "123").status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::ValidationError("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::RateLimited { retry_after: 60 }.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(ApiError::Unauthorized.error_code(), "UNAUTHORIZED");
        assert_eq!(
            ApiError::not_found("track", "123").error_code(),
            "NOT_FOUND"
        );
    }

    #[test]
    fn test_error_display() {
        let err = ApiError::not_found("track", "abc123");
        assert_eq!(err.to_string(), "track not found: abc123");
    }
}
