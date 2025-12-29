//! Audio feature extraction job
//!
//! Extracts audio features from tracks using Symphonia for analysis.
//! Features include loudness, energy, and basic audio characteristics.

use std::fs::File;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use uuid::Uuid;

use crate::error::{WorkerError, WorkerResult};
use crate::AppState;

/// Maximum file size for feature extraction (500 MB)
const MAX_FILE_SIZE_BYTES: u64 = 500 * 1024 * 1024;

/// Maximum samples to process (~17 minutes at 96kHz stereo)
const MAX_SAMPLES: u64 = 100_000_000;

/// Feature extraction job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureExtractionJob {
    /// Track ID (UUID as string) to process
    pub track_id: String,
}

impl FeatureExtractionJob {
    /// Parse track_id as UUID
    pub fn track_uuid(&self) -> Result<Uuid, uuid::Error> {
        Uuid::parse_str(&self.track_id)
    }
}

/// Extracted audio features matching database schema
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioFeatures {
    /// Beats per minute (tempo) - requires advanced BPM detection
    pub bpm: Option<f32>,

    /// Musical key (e.g., "C major", "A minor") - requires chromagram analysis
    pub key: Option<String>,

    /// Mode (major/minor) - derived from key detection
    pub mode: Option<String>,

    /// Overall loudness in LUFS (approximated from RMS)
    pub loudness: Option<f32>,

    /// Energy level (0.0 - 1.0) - derived from RMS
    pub energy: Option<f32>,

    /// Danceability (0.0 - 1.0) - requires rhythm analysis
    pub danceability: Option<f32>,

    /// Valence/mood (0.0 - 1.0) - requires spectral analysis
    pub valence: Option<f32>,

    /// Acousticness (0.0 - 1.0) - requires spectral analysis
    pub acousticness: Option<f32>,

    /// Instrumentalness (0.0 - 1.0) - requires voice detection
    pub instrumentalness: Option<f32>,

    /// Speechiness (0.0 - 1.0) - requires voice detection
    pub speechiness: Option<f32>,

    /// Peak amplitude (0.0 - 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak: Option<f32>,

    /// Dynamic range in dB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_range: Option<f32>,
}

/// Track info for feature extraction
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct TrackInfo {
    id: Uuid,
    file_path: String,
}

/// Audio analysis statistics collected during decoding
#[derive(Debug, Default)]
struct AudioStats {
    /// Sum of squared samples for RMS calculation
    sum_squared: f64,
    /// Total number of samples
    sample_count: u64,
    /// Peak absolute sample value
    peak: f32,
}

impl AudioStats {
    /// Calculate RMS (Root Mean Square) amplitude
    fn rms(&self) -> f32 {
        if self.sample_count == 0 {
            return 0.0;
        }
        (self.sum_squared / self.sample_count as f64).sqrt() as f32
    }

    /// Convert RMS to approximate LUFS
    /// Note: True LUFS requires K-weighting filter, this is an approximation
    fn approximate_lufs(&self) -> f32 {
        let rms = self.rms();
        if rms <= 0.0 {
            return -70.0; // Silence
        }
        // Convert to dB scale, approximate LUFS offset
        20.0 * rms.log10() - 0.691
    }

    /// Calculate energy as normalized RMS (0.0 - 1.0 scale)
    fn energy(&self) -> f32 {
        let rms = self.rms();
        // Normalize: typical music RMS is 0.1-0.3, map to 0-1 scale
        (rms * 3.0).min(1.0)
    }
}

/// Execute the feature extraction job
pub async fn execute(state: &AppState, job: &FeatureExtractionJob) -> WorkerResult<()> {
    tracing::info!("Extracting features for track ID: {}", job.track_id);

    // Parse track ID
    let track_id = job
        .track_uuid()
        .map_err(|e| WorkerError::InvalidJobData(format!("Invalid track ID: {}", e)))?;

    // Load track from database
    let track: TrackInfo = sqlx::query_as("SELECT id, file_path FROM tracks WHERE id = $1")
        .bind(track_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| WorkerError::InvalidJobData(format!("Track not found: {}", track_id)))?;

    // Get music library path for path traversal protection
    let library_path = state.config.music_library_path();

    // Security: Canonicalize paths and verify track is within library
    let canonical_library = library_path.canonicalize().map_err(|e| {
        WorkerError::Configuration(format!("Failed to canonicalize library path: {}", e))
    })?;

    let track_path = PathBuf::from(&track.file_path);
    let canonical_track = track_path.canonicalize().map_err(|_| {
        WorkerError::InvalidJobData(format!("Track file not found: {}", track.file_path))
    })?;

    if !canonical_track.starts_with(&canonical_library) {
        return Err(WorkerError::InvalidJobData(format!(
            "Track path {:?} is outside the music library",
            track.file_path
        )));
    }

    // Check file size before processing
    let metadata = std::fs::metadata(&canonical_track)?;
    if metadata.len() > MAX_FILE_SIZE_BYTES {
        tracing::warn!(
            "Track {} exceeds max file size ({} bytes > {} bytes), skipping",
            track_id,
            metadata.len(),
            MAX_FILE_SIZE_BYTES
        );
        return Ok(()); // Skip without error - very large files are not processed
    }

    // Run CPU-intensive extraction in blocking thread pool
    let path_for_extraction = canonical_track.clone();
    let extraction_result =
        tokio::task::spawn_blocking(move || extract_features(&path_for_extraction)).await;

    // Only update database if extraction succeeded (don't overwrite existing data with defaults)
    let features = match extraction_result {
        Ok(Ok(f)) => Some(f),
        Ok(Err(e)) => {
            tracing::warn!("Failed to extract features for track {}: {}", track_id, e);
            None
        }
        Err(e) => {
            // Differentiate panics from cancellations for better diagnostics
            if e.is_panic() {
                tracing::error!(
                    "Feature extraction task panicked for track {}: {}",
                    track_id,
                    e
                );
            } else {
                tracing::warn!(
                    "Feature extraction task cancelled for track {}: {}",
                    track_id,
                    e
                );
            }
            // Return error to enable job retries
            return Err(WorkerError::AudioProcessing(format!(
                "Feature extraction join error for {}: {}",
                track_id, e
            )));
        }
    };

    if let Some(features) = features {
        let features_json = serde_json::to_value(&features).map_err(|e| {
            WorkerError::InvalidJobData(format!("Failed to serialize features: {}", e))
        })?;

        sqlx::query("UPDATE tracks SET audio_features = $1, updated_at = NOW() WHERE id = $2")
            .bind(&features_json)
            .bind(track_id)
            .execute(&state.db)
            .await?;

        tracing::info!(
            "Feature extraction completed for track {}: loudness={:?}dB, energy={:?}",
            track_id,
            features.loudness,
            features.energy
        );
    }

    Ok(())
}

/// Extract audio features from a file using Symphonia
fn extract_features(path: &Path) -> WorkerResult<AudioFeatures> {
    let path_str = path.display().to_string();

    // Open the audio file
    let file = File::open(path)?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint based on file extension
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    // Probe the format
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| WorkerError::audio_decoding(&path_str, format!("Failed to probe: {}", e)))?;

    let mut format = probed.format;

    // Get the default track
    let track = format
        .default_track()
        .ok_or_else(|| WorkerError::AudioProcessing("No audio track found".to_string()))?;
    let selected_track_id = track.id;

    // Create a decoder
    let decoder_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|e| WorkerError::audio_decoding(&path_str, format!("Decoder error: {}", e)))?;

    // Collect audio statistics
    let mut stats = AudioStats::default();

    // Create sample buffer based on codec params
    let spec = symphonia::core::audio::SignalSpec::new(
        track.codec_params.sample_rate.unwrap_or(44100),
        track
            .codec_params
            .channels
            .unwrap_or(symphonia::core::audio::Channels::FRONT_LEFT),
    );
    let max_frames = track.codec_params.max_frames_per_packet.unwrap_or(4096) as u64;
    let mut sample_buf = SampleBuffer::<f32>::new(max_frames, spec);

    // Decode packets and analyze samples
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                // End of stream
                break;
            }
            Err(e) => {
                tracing::debug!("Error reading packet: {}", e);
                break;
            }
        };

        // Only process packets from the selected track (skip other streams)
        if packet.track_id() != selected_track_id {
            continue;
        }

        // Check sample limit to prevent unbounded processing
        if stats.sample_count >= MAX_SAMPLES {
            tracing::debug!("Sample limit reached, stopping analysis");
            break;
        }

        // Decode the packet
        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Resize buffer if needed
                if decoded.capacity() > sample_buf.capacity() as usize {
                    sample_buf = SampleBuffer::new(decoded.capacity() as u64, *decoded.spec());
                }

                // Copy decoded samples to buffer
                sample_buf.copy_interleaved_ref(decoded);

                // Analyze samples
                for &sample in sample_buf.samples() {
                    let abs_sample = sample.abs();
                    stats.sum_squared += (sample * sample) as f64;
                    stats.sample_count += 1;
                    if abs_sample > stats.peak {
                        stats.peak = abs_sample;
                    }

                    // Check limit during sample processing
                    if stats.sample_count >= MAX_SAMPLES {
                        break;
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Error decoding packet: {}", e);
                continue;
            }
        }
    }

    // Calculate features from statistics
    let rms = stats.rms();
    let dynamic_range = if stats.peak > f32::EPSILON && rms > f32::EPSILON {
        let ratio = stats.peak / rms;
        if ratio.is_finite() && ratio > 0.0 {
            Some(20.0 * ratio.log10())
        } else {
            None
        }
    } else {
        None
    };

    let features = AudioFeatures {
        loudness: Some(stats.approximate_lufs()),
        energy: Some(stats.energy()),
        peak: Some(stats.peak),
        dynamic_range,
        // Advanced features require specialized algorithms
        bpm: None,
        key: None,
        mode: None,
        danceability: None,
        valence: None,
        acousticness: None,
        instrumentalness: None,
        speechiness: None,
    };

    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_stats_rms() {
        let mut stats = AudioStats::default();
        // Add samples: 0.5, -0.5, 0.5, -0.5 (RMS should be 0.5)
        for sample in [0.5f32, -0.5, 0.5, -0.5] {
            stats.sum_squared += (sample * sample) as f64;
            stats.sample_count += 1;
        }
        let rms = stats.rms();
        assert!((rms - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_audio_stats_empty() {
        let stats = AudioStats::default();
        assert_eq!(stats.rms(), 0.0);
        assert_eq!(stats.energy(), 0.0);
    }

    #[test]
    fn test_audio_features_default() {
        let features = AudioFeatures::default();
        assert!(features.bpm.is_none());
        assert!(features.loudness.is_none());
        assert!(features.energy.is_none());
    }

    #[test]
    fn test_feature_extraction_job_parse() {
        let job = FeatureExtractionJob {
            track_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        assert!(job.track_uuid().is_ok());

        let invalid_job = FeatureExtractionJob {
            track_id: "invalid".to_string(),
        };
        assert!(invalid_job.track_uuid().is_err());
    }
}
