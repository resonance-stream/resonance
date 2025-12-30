//! Comprehensive error handling for the Resonance Worker
//!
//! This module provides a unified error type hierarchy using thiserror
//! for background job processing, with specific variants for each job type.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Main worker error type with comprehensive error variants
#[derive(Error, Debug)]
pub enum WorkerError {
    // ========== Job Processing Errors ==========
    /// Job data could not be parsed
    #[error("invalid job data: {0}")]
    InvalidJobData(String),

    /// Invalid job payload (missing or malformed fields)
    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    /// Resource not found
    #[error("not found: {0}")]
    NotFound(String),

    /// Job execution was cancelled (e.g., due to shutdown)
    #[error("job cancelled: {0}")]
    Cancelled(String),

    /// Job timed out during execution
    #[error("job timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Job failed after maximum retry attempts
    #[error("job failed after {attempts} attempts: {reason}")]
    MaxRetriesExceeded { attempts: u32, reason: String },

    // ========== Database Errors ==========
    /// Database query failed
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Database connection pool exhausted
    #[error("database connection unavailable")]
    DatabaseUnavailable,

    /// Database transaction failed
    #[error("database transaction failed: {0}")]
    Transaction(String),

    // ========== Redis/Queue Errors ==========
    /// Redis operation failed
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Queue operation failed
    #[error("queue error: {0}")]
    Queue(String),

    /// Failed to deserialize job from queue
    #[error("job deserialization failed: {0}")]
    JobDeserialization(#[from] serde_json::Error),

    // ========== Library Scan Errors ==========
    /// File system access error
    #[error("filesystem error: {0}")]
    Filesystem(#[from] std::io::Error),

    /// Music library path not found or inaccessible
    #[error("music library path not found: {0}")]
    LibraryNotFound(String),

    /// Failed to read audio file metadata
    #[error("metadata extraction failed for '{path}': {reason}")]
    MetadataExtraction { path: String, reason: String },

    /// Unsupported audio format
    #[error("unsupported audio format: {0}")]
    UnsupportedFormat(String),

    // ========== Audio Feature Extraction Errors ==========
    /// Generic audio processing error
    #[error("audio processing error: {0}")]
    AudioProcessing(String),

    /// Audio decoding failed
    #[error("audio decoding failed for '{path}': {reason}")]
    AudioDecoding { path: String, reason: String },

    /// Audio analysis failed
    #[error("audio analysis failed: {0}")]
    AudioAnalysis(String),

    /// Invalid audio data
    #[error("invalid audio data: {0}")]
    InvalidAudioData(String),

    // ========== Embedding Generation Errors ==========
    /// Ollama service unavailable
    #[error("Ollama service unavailable: {0}")]
    OllamaUnavailable(String),

    /// Ollama model not found
    #[error("Ollama model not found: {0}")]
    OllamaModelNotFound(String),

    /// Embedding generation failed
    #[error("embedding generation failed: {0}")]
    EmbeddingGeneration(String),

    /// Invalid embedding dimensions
    #[error("invalid embedding dimensions: expected {expected}, got {actual}")]
    InvalidEmbeddingDimensions { expected: usize, actual: usize },

    // ========== Mood Detection Errors ==========
    /// Mood detection failed
    #[error("mood detection failed: {0}")]
    MoodDetectionFailed(String),

    // ========== Lidarr Integration Errors ==========
    /// Lidarr not configured
    #[error("Lidarr integration not configured")]
    LidarrNotConfigured,

    /// Lidarr API error
    #[error("Lidarr API error: {status_code} - {message}")]
    LidarrApi { status_code: u16, message: String },

    /// Lidarr sync conflict
    #[error("Lidarr sync conflict: {0}")]
    LidarrSyncConflict(String),

    // ========== HTTP/External Service Errors ==========
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// External service timeout
    #[error("external service timeout: {service}")]
    ServiceTimeout { service: String },

    /// External service returned error
    #[error("external service error from {service}: {message}")]
    ServiceError { service: String, message: String },

    // ========== Playlist Generation Errors ==========
    /// Not enough tracks for playlist generation
    #[error("not enough tracks for playlist generation: need {required}, have {available}")]
    InsufficientTracks { required: usize, available: usize },

    /// User not found for personalized playlist
    #[error("user not found: {0}")]
    UserNotFound(String),

    /// No listening history for recommendations
    #[error("no listening history found for user: {0}")]
    NoListeningHistory(String),

    // ========== Prefetch Errors ==========
    /// Track not found for prefetch
    #[error("track not found for prefetch: {0}")]
    TrackNotFound(i64),

    /// Prefetch queue full
    #[error("prefetch queue full, max capacity: {0}")]
    PrefetchQueueFull(usize),

    // ========== Configuration Errors ==========
    /// Configuration error
    #[error("configuration error: {0}")]
    Configuration(String),

    /// Missing required configuration
    #[error("missing required configuration: {0}")]
    MissingConfiguration(&'static str),

    // ========== Internal Errors ==========
    /// Internal worker error (catch-all for unexpected errors)
    #[error("internal worker error: {0}")]
    Internal(String),
}

impl WorkerError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Database(_)
                | Self::DatabaseUnavailable
                | Self::Redis(_)
                | Self::Queue(_)
                | Self::OllamaUnavailable(_)
                | Self::Http(_)
                | Self::ServiceTimeout { .. }
                | Self::Timeout { .. }
        )
    }

    /// Get a severity level for logging
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            // Critical errors that should alert operators
            Self::Configuration(_)
            | Self::MissingConfiguration(_)
            | Self::DatabaseUnavailable
            | Self::MaxRetriesExceeded { .. } => ErrorSeverity::Critical,

            // Errors that indicate service issues
            Self::Database(_)
            | Self::Redis(_)
            | Self::OllamaUnavailable(_)
            | Self::LidarrApi { .. }
            | Self::Internal(_) => ErrorSeverity::Error,

            // Warnings for expected failures
            Self::Timeout { .. }
            | Self::ServiceTimeout { .. }
            | Self::Http(_)
            | Self::Cancelled(_) => ErrorSeverity::Warning,

            // Info level for normal processing issues
            _ => ErrorSeverity::Info,
        }
    }

    /// Get the job type this error is related to, if applicable
    pub fn job_context(&self) -> Option<&'static str> {
        match self {
            Self::LibraryNotFound(_)
            | Self::MetadataExtraction { .. }
            | Self::AudioProcessing(_) => Some("library_scan"),
            Self::AudioDecoding { .. }
            | Self::AudioAnalysis(_)
            | Self::InvalidAudioData(_)
            | Self::UnsupportedFormat(_) => Some("feature_extraction"),
            Self::OllamaUnavailable(_)
            | Self::OllamaModelNotFound(_)
            | Self::EmbeddingGeneration(_)
            | Self::InvalidEmbeddingDimensions { .. } => Some("embedding_generation"),
            Self::MoodDetectionFailed(_) => Some("mood_detection"),
            Self::LidarrNotConfigured | Self::LidarrApi { .. } | Self::LidarrSyncConflict(_) => {
                Some("lidarr_sync")
            }
            Self::InsufficientTracks { .. }
            | Self::UserNotFound(_)
            | Self::NoListeningHistory(_) => Some("weekly_playlist"),
            Self::TrackNotFound(_) | Self::PrefetchQueueFull(_) => Some("prefetch"),
            _ => None,
        }
    }

    /// Log the error with appropriate severity
    pub fn log(&self) {
        let context = self.job_context().unwrap_or("general");
        match self.severity() {
            ErrorSeverity::Critical => {
                tracing::error!(
                    error = %self,
                    context = context,
                    retryable = self.is_retryable(),
                    "Critical worker error"
                );
            }
            ErrorSeverity::Error => {
                tracing::error!(
                    error = %self,
                    context = context,
                    retryable = self.is_retryable(),
                    "Worker error"
                );
            }
            ErrorSeverity::Warning => {
                tracing::warn!(
                    error = %self,
                    context = context,
                    retryable = self.is_retryable(),
                    "Worker warning"
                );
            }
            ErrorSeverity::Info => {
                tracing::info!(
                    error = %self,
                    context = context,
                    retryable = self.is_retryable(),
                    "Worker info"
                );
            }
        }
    }

    /// Create a metadata extraction error
    pub fn metadata_extraction(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::MetadataExtraction {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create an audio decoding error
    pub fn audio_decoding(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::AudioDecoding {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a Lidarr API error
    pub fn lidarr_api(status_code: u16, message: impl Into<String>) -> Self {
        Self::LidarrApi {
            status_code,
            message: message.into(),
        }
    }

    /// Create a service error
    pub fn service_error(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ServiceError {
            service: service.into(),
            message: message.into(),
        }
    }
}

/// Error severity levels for logging and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Critical errors that should trigger alerts
    Critical,
    /// Standard errors
    Error,
    /// Warnings for expected failures
    Warning,
    /// Informational messages
    Info,
}

/// Result type alias for worker operations
pub type WorkerResult<T> = Result<T, WorkerError>;

/// Job execution result with metadata for retry handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Whether the job succeeded
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Whether the job can be retried
    pub retryable: bool,
    /// Number of attempts made
    pub attempts: u32,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl JobResult {
    /// Create a successful job result
    pub fn success(duration_ms: u64) -> Self {
        Self {
            success: true,
            error_message: None,
            retryable: false,
            attempts: 1,
            duration_ms,
        }
    }

    /// Create a failed job result from an error
    pub fn from_error(err: &WorkerError, attempts: u32, duration_ms: u64) -> Self {
        Self {
            success: false,
            error_message: Some(err.to_string()),
            retryable: err.is_retryable(),
            attempts,
            duration_ms,
        }
    }
}

// ========== Conversion Implementations ==========

impl From<anyhow::Error> for WorkerError {
    fn from(err: anyhow::Error) -> Self {
        // Try to downcast to WorkerError first
        match err.downcast::<WorkerError>() {
            Ok(worker_err) => worker_err,
            Err(err) => Self::Internal(err.to_string()),
        }
    }
}

impl From<std::env::VarError> for WorkerError {
    fn from(err: std::env::VarError) -> Self {
        Self::Configuration(err.to_string())
    }
}

impl From<url::ParseError> for WorkerError {
    fn from(err: url::ParseError) -> Self {
        Self::Configuration(format!("invalid URL: {}", err))
    }
}

impl From<resonance_ollama_client::OllamaError> for WorkerError {
    fn from(err: resonance_ollama_client::OllamaError) -> Self {
        match &err {
            resonance_ollama_client::OllamaError::ConnectionRefused(url) => {
                Self::OllamaUnavailable(format!("connection refused to {}", url))
            }
            resonance_ollama_client::OllamaError::ModelNotFound(model) => {
                Self::OllamaModelNotFound(model.clone())
            }
            resonance_ollama_client::OllamaError::DimensionMismatch { expected, actual } => {
                Self::InvalidEmbeddingDimensions {
                    expected: *expected,
                    actual: *actual,
                }
            }
            resonance_ollama_client::OllamaError::Timeout(secs) => Self::ServiceTimeout {
                service: format!("Ollama ({}s)", secs),
            },
            resonance_ollama_client::OllamaError::RetriesExhausted {
                attempts,
                last_error,
            } => Self::MaxRetriesExceeded {
                attempts: *attempts,
                reason: last_error.clone(),
            },
            _ => Self::EmbeddingGeneration(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retryable_errors() {
        assert!(WorkerError::DatabaseUnavailable.is_retryable());
        assert!(WorkerError::Timeout { seconds: 30 }.is_retryable());
        assert!(WorkerError::OllamaUnavailable("test".to_string()).is_retryable());

        assert!(!WorkerError::InvalidJobData("test".to_string()).is_retryable());
        assert!(!WorkerError::LidarrNotConfigured.is_retryable());
        assert!(!WorkerError::UnsupportedFormat("mp4".to_string()).is_retryable());
    }

    #[test]
    fn test_severity_levels() {
        assert_eq!(
            WorkerError::Configuration("test".to_string()).severity(),
            ErrorSeverity::Critical
        );
        assert_eq!(
            WorkerError::DatabaseUnavailable.severity(),
            ErrorSeverity::Critical
        );
        assert_eq!(
            WorkerError::Database(sqlx::Error::PoolClosed).severity(),
            ErrorSeverity::Error
        );
        assert_eq!(
            WorkerError::Timeout { seconds: 30 }.severity(),
            ErrorSeverity::Warning
        );
    }

    #[test]
    fn test_job_context() {
        assert_eq!(
            WorkerError::LibraryNotFound("/music".to_string()).job_context(),
            Some("library_scan")
        );
        assert_eq!(
            WorkerError::OllamaUnavailable("test".to_string()).job_context(),
            Some("embedding_generation")
        );
        assert_eq!(
            WorkerError::LidarrNotConfigured.job_context(),
            Some("lidarr_sync")
        );
        assert_eq!(
            WorkerError::TrackNotFound(123).job_context(),
            Some("prefetch")
        );
    }

    #[test]
    fn test_error_display() {
        let err = WorkerError::metadata_extraction("/path/to/file.mp3", "failed to read tags");
        assert_eq!(
            err.to_string(),
            "metadata extraction failed for '/path/to/file.mp3': failed to read tags"
        );

        let err = WorkerError::lidarr_api(404, "Artist not found");
        assert_eq!(err.to_string(), "Lidarr API error: 404 - Artist not found");
    }

    #[test]
    fn test_job_result() {
        let result = JobResult::success(1500);
        assert!(result.success);
        assert!(result.error_message.is_none());

        let err = WorkerError::DatabaseUnavailable;
        let result = JobResult::from_error(&err, 3, 5000);
        assert!(!result.success);
        assert!(result.retryable);
        assert_eq!(result.attempts, 3);
    }
}
