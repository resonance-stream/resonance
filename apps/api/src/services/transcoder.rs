//! Audio transcoding service using FFmpeg
//!
//! Provides on-the-fly format and bitrate conversion for audio streaming.
//! Supports converting between common audio formats (MP3, AAC, Opus, FLAC).
//!
//! # Security
//!
//! Callers MUST validate input paths before calling `transcode()`. This service
//! does NOT perform path traversal validation - it trusts the caller to provide
//! paths that are within the allowed music library directory.
//!
//! # Resource Limits
//!
//! The service enforces a configurable limit on concurrent transcoding operations
//! to prevent resource exhaustion. Requests that exceed this limit will receive
//! a `ResourceExhausted` error.

use bytes::Bytes;
use futures_core::Stream;
use std::path::Path;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::io::ReaderStream;

/// Errors that can occur during transcoding
#[derive(Error, Debug)]
pub enum TranscodeError {
    #[error("FFmpeg not found in PATH")]
    FfmpegNotFound,

    #[error("FFmpeg process failed: {0}")]
    ProcessError(String),

    #[error("Unsupported format: {0}")]
    #[allow(dead_code)]
    UnsupportedFormat(String),

    #[error("Invalid bitrate: {0}")]
    InvalidBitrate(u32),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Transcoding limit reached, try again later")]
    ResourceExhausted,
}

/// Output format for transcoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscodeFormat {
    Mp3,
    Aac,
    Opus,
    Flac,
}

impl TranscodeFormat {
    /// Parse format from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mp3" => Some(Self::Mp3),
            "aac" | "m4a" => Some(Self::Aac),
            "opus" | "ogg" => Some(Self::Opus),
            "flac" => Some(Self::Flac),
            _ => None,
        }
    }

    /// Get FFmpeg format/codec parameters
    fn ffmpeg_args(&self) -> Vec<&'static str> {
        match self {
            Self::Mp3 => vec!["-f", "mp3", "-c:a", "libmp3lame"],
            Self::Aac => vec!["-f", "adts", "-c:a", "aac"],
            Self::Opus => vec!["-f", "opus", "-c:a", "libopus"],
            Self::Flac => vec!["-f", "flac", "-c:a", "flac"],
        }
    }

    /// Get default bitrate for format (in kbps)
    pub fn default_bitrate(&self) -> u32 {
        match self {
            Self::Mp3 => 320,
            Self::Aac => 256,
            Self::Opus => 128,
            Self::Flac => 0, // Lossless, no bitrate
        }
    }

    /// Validate bitrate for format
    pub fn validate_bitrate(&self, bitrate: u32) -> Result<u32, TranscodeError> {
        match self {
            Self::Flac => Ok(0), // Ignore bitrate for lossless
            _ => {
                // Valid bitrates: 64, 96, 128, 192, 256, 320
                if matches!(bitrate, 64 | 96 | 128 | 192 | 256 | 320) {
                    Ok(bitrate)
                } else {
                    Err(TranscodeError::InvalidBitrate(bitrate))
                }
            }
        }
    }

    /// Get MIME type for format
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Aac => "audio/aac",
            Self::Opus => "audio/opus",
            Self::Flac => "audio/flac",
        }
    }
}

/// Transcoding options
#[derive(Debug, Clone)]
pub struct TranscodeOptions {
    pub format: TranscodeFormat,
    pub bitrate: u32,
}

impl TranscodeOptions {
    /// Create new transcode options with default bitrate
    pub fn new(format: TranscodeFormat) -> Self {
        Self {
            bitrate: format.default_bitrate(),
            format,
        }
    }

    /// Create with custom bitrate (validates the bitrate)
    pub fn with_bitrate(format: TranscodeFormat, bitrate: u32) -> Result<Self, TranscodeError> {
        let validated_bitrate = format.validate_bitrate(bitrate)?;
        Ok(Self {
            format,
            bitrate: validated_bitrate,
        })
    }
}

/// Stream wrapper for FFmpeg output
///
/// Holds an `OwnedSemaphorePermit` that is released when the stream is dropped,
/// ensuring the concurrent transcoding limit is properly enforced.
pub struct TranscodeStream {
    inner: ReaderStream<tokio::process::ChildStdout>,
    child: Child,
    /// Semaphore permit - released when stream is dropped
    _permit: OwnedSemaphorePermit,
}

impl TranscodeStream {
    /// Create a new transcode stream from an FFmpeg child process
    fn new(mut child: Child, permit: OwnedSemaphorePermit) -> Result<Self, TranscodeError> {
        let stdout = child.stdout.take().ok_or_else(|| {
            TranscodeError::ProcessError("Failed to capture FFmpeg stdout".into())
        })?;

        Ok(Self {
            inner: ReaderStream::new(stdout),
            child,
            _permit: permit,
        })
    }
}

impl Stream for TranscodeStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl Drop for TranscodeStream {
    fn drop(&mut self) {
        // Kill FFmpeg process when stream is dropped
        if let Err(e) = self.child.start_kill() {
            tracing::warn!(error = %e, "Failed to kill FFmpeg process on drop");
        }
    }
}

/// Default maximum concurrent transcoding operations
pub const DEFAULT_MAX_CONCURRENT_TRANSCODES: usize = 4;

/// Transcoder service for converting audio formats
///
/// Enforces a configurable limit on concurrent transcoding operations
/// to prevent resource exhaustion from spawning too many FFmpeg processes.
#[derive(Clone)]
pub struct TranscoderService {
    /// Semaphore to limit concurrent transcoding operations
    semaphore: Arc<Semaphore>,
    /// Maximum concurrent transcodes (for logging/metrics)
    max_concurrent: usize,
}

impl std::fmt::Debug for TranscoderService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscoderService")
            .field("max_concurrent", &self.max_concurrent)
            .field("available_permits", &self.semaphore.available_permits())
            .finish()
    }
}

impl TranscoderService {
    /// Create a new transcoder service with the default concurrency limit
    pub fn new() -> Self {
        Self::with_max_concurrent(DEFAULT_MAX_CONCURRENT_TRANSCODES)
    }

    /// Create a new transcoder service with a custom concurrency limit
    pub fn with_max_concurrent(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    /// Check if FFmpeg is available
    ///
    /// Useful for startup validation to fail fast if FFmpeg is not installed.
    #[allow(dead_code)]
    pub async fn check_ffmpeg(&self) -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Get the number of currently active transcoding operations
    pub fn active_transcodes(&self) -> usize {
        self.max_concurrent - self.semaphore.available_permits()
    }

    /// Transcode an audio file to a different format
    ///
    /// Returns a stream of bytes that can be sent directly to the client.
    /// The stream is produced by FFmpeg in real-time.
    ///
    /// # Resource Limits
    ///
    /// This method acquires a semaphore permit before spawning FFmpeg.
    /// If all permits are in use, returns `TranscodeError::ResourceExhausted`.
    /// The permit is held by the returned `TranscodeStream` and released
    /// when the stream is dropped.
    pub async fn transcode(
        &self,
        input_path: &Path,
        options: &TranscodeOptions,
    ) -> Result<TranscodeStream, TranscodeError> {
        // Acquire semaphore permit (non-blocking - fail fast if at limit)
        let permit = self.semaphore.clone().try_acquire_owned().map_err(|_| {
            tracing::warn!(
                active = self.active_transcodes(),
                max = self.max_concurrent,
                "Transcoding limit reached"
            );
            TranscodeError::ResourceExhausted
        })?;

        tracing::debug!(
            format = ?options.format,
            bitrate = options.bitrate,
            path = %input_path.display(),
            active = self.active_transcodes(),
            "Starting transcode"
        );

        let mut cmd = Command::new("ffmpeg");

        // Input file
        cmd.arg("-i")
            .arg(input_path)
            // Suppress banner and stats
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error");

        // Add format-specific codec args
        for arg in options.format.ffmpeg_args() {
            cmd.arg(arg);
        }

        // Add bitrate for lossy formats
        if options.bitrate > 0 {
            cmd.arg("-b:a").arg(format!("{}k", options.bitrate));
        }

        // Output to stdout (pipe)
        cmd.arg("pipe:1");

        // Configure stdio - capture stderr for logging
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TranscodeError::FfmpegNotFound
            } else {
                TranscodeError::ProcessError(e.to_string())
            }
        })?;

        // Spawn a task to read and log stderr
        if let Some(stderr) = child.stderr.take() {
            let input_path_str = input_path.display().to_string();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::warn!(
                        target: "ffmpeg",
                        path = %input_path_str,
                        "{}", line
                    );
                }
            });
        }

        TranscodeStream::new(child, permit)
    }
}

impl Default for TranscoderService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_parse() {
        assert_eq!(TranscodeFormat::parse("mp3"), Some(TranscodeFormat::Mp3));
        assert_eq!(TranscodeFormat::parse("MP3"), Some(TranscodeFormat::Mp3));
        assert_eq!(TranscodeFormat::parse("aac"), Some(TranscodeFormat::Aac));
        assert_eq!(TranscodeFormat::parse("m4a"), Some(TranscodeFormat::Aac));
        assert_eq!(TranscodeFormat::parse("opus"), Some(TranscodeFormat::Opus));
        assert_eq!(TranscodeFormat::parse("ogg"), Some(TranscodeFormat::Opus));
        assert_eq!(TranscodeFormat::parse("flac"), Some(TranscodeFormat::Flac));
        assert_eq!(TranscodeFormat::parse("wav"), None);
        assert_eq!(TranscodeFormat::parse("invalid"), None);
    }

    #[test]
    fn test_default_bitrates() {
        assert_eq!(TranscodeFormat::Mp3.default_bitrate(), 320);
        assert_eq!(TranscodeFormat::Aac.default_bitrate(), 256);
        assert_eq!(TranscodeFormat::Opus.default_bitrate(), 128);
        assert_eq!(TranscodeFormat::Flac.default_bitrate(), 0);
    }

    #[test]
    fn test_validate_bitrate() {
        // Valid bitrates
        assert!(TranscodeFormat::Mp3.validate_bitrate(128).is_ok());
        assert!(TranscodeFormat::Mp3.validate_bitrate(320).is_ok());
        assert!(TranscodeFormat::Aac.validate_bitrate(256).is_ok());

        // Invalid bitrates
        assert!(TranscodeFormat::Mp3.validate_bitrate(100).is_err());
        assert!(TranscodeFormat::Mp3.validate_bitrate(400).is_err());

        // FLAC ignores bitrate
        assert!(TranscodeFormat::Flac.validate_bitrate(0).is_ok());
        assert!(TranscodeFormat::Flac.validate_bitrate(320).is_ok());
    }

    #[test]
    fn test_content_type() {
        assert_eq!(TranscodeFormat::Mp3.content_type(), "audio/mpeg");
        assert_eq!(TranscodeFormat::Aac.content_type(), "audio/aac");
        assert_eq!(TranscodeFormat::Opus.content_type(), "audio/opus");
        assert_eq!(TranscodeFormat::Flac.content_type(), "audio/flac");
    }

    #[test]
    fn test_transcode_options_new() {
        let opts = TranscodeOptions::new(TranscodeFormat::Mp3);
        assert_eq!(opts.format, TranscodeFormat::Mp3);
        assert_eq!(opts.bitrate, 320);
    }

    #[test]
    fn test_transcode_options_with_bitrate() {
        let opts = TranscodeOptions::with_bitrate(TranscodeFormat::Mp3, 128).unwrap();
        assert_eq!(opts.bitrate, 128);

        let err = TranscodeOptions::with_bitrate(TranscodeFormat::Mp3, 100);
        assert!(err.is_err());
    }
}
