//! Track model for Resonance
//!
//! This module contains the database model for tracks
//! with audio features, AI tags, and playback statistics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Audio format enum matching PostgreSQL audio_format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audio_format", rename_all = "lowercase")]
pub enum AudioFormat {
    Flac,
    #[default]
    Mp3,
    Aac,
    Opus,
    Ogg,
    Wav,
    Alac,
    Other,
}

impl AudioFormat {
    /// Returns whether this format is lossless
    pub fn is_lossless(&self) -> bool {
        matches!(self, Self::Flac | Self::Wav | Self::Alac)
    }

    /// Returns the typical file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
            Self::Aac => "m4a",
            Self::Opus => "opus",
            Self::Ogg => "ogg",
            Self::Wav => "wav",
            Self::Alac => "m4a",
            Self::Other => "bin",
        }
    }
}

/// Audio features extracted from the track
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioFeatures {
    /// Beats per minute
    pub bpm: Option<f64>,
    /// Musical key (e.g., "C", "G#")
    pub key: Option<String>,
    /// Mode (major/minor)
    pub mode: Option<String>,
    /// Loudness in dB
    pub loudness: Option<f64>,
    /// Energy level (0.0 - 1.0)
    pub energy: Option<f64>,
    /// Danceability (0.0 - 1.0)
    pub danceability: Option<f64>,
    /// Valence/happiness (0.0 - 1.0)
    pub valence: Option<f64>,
    /// Acousticness (0.0 - 1.0)
    pub acousticness: Option<f64>,
    /// Instrumentalness (0.0 - 1.0)
    pub instrumentalness: Option<f64>,
    /// Speechiness (0.0 - 1.0)
    pub speechiness: Option<f64>,
}

/// Synced lyrics with timestamps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncedLyricLine {
    /// Start time in milliseconds
    pub time_ms: i64,
    /// Lyric text
    pub text: String,
}

/// Track record from the tracks table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Track {
    /// Unique track identifier
    pub id: Uuid,

    /// Track title
    pub title: String,

    /// Album this track belongs to (optional for singles)
    pub album_id: Option<Uuid>,

    /// Artist who created this track
    pub artist_id: Uuid,

    /// MusicBrainz identifier
    pub mbid: Option<Uuid>,

    // File information
    /// Path to audio file
    pub file_path: String,

    /// File size in bytes
    pub file_size: i64,

    /// Audio format (FLAC, MP3, etc.)
    pub file_format: AudioFormat,

    /// SHA-256 hash of the file
    pub file_hash: Option<String>,

    // Audio properties
    /// Duration in milliseconds
    pub duration_ms: i32,

    /// Bit rate in kbps
    pub bit_rate: Option<i32>,

    /// Sample rate in Hz
    pub sample_rate: Option<i32>,

    /// Number of audio channels
    pub channels: Option<i16>,

    /// Bit depth
    pub bit_depth: Option<i16>,

    // Track metadata
    /// Track number on album
    pub track_number: Option<i16>,

    /// Disc number for multi-disc albums
    pub disc_number: Option<i16>,

    /// Genre tags
    pub genres: Vec<String>,

    /// Explicit content flag
    pub explicit: bool,

    /// Static lyrics text
    pub lyrics: Option<String>,

    /// Time-synced lyrics
    #[sqlx(json)]
    pub synced_lyrics: Option<Vec<SyncedLyricLine>>,

    // Audio features
    /// Extracted audio features (BPM, key, energy, etc.)
    #[sqlx(json)]
    pub audio_features: AudioFeatures,

    // AI-generated data
    /// AI-detected mood tags
    pub ai_mood: Vec<String>,

    /// AI-generated descriptive tags
    pub ai_tags: Vec<String>,

    /// AI-generated description
    pub ai_description: Option<String>,

    // Playback statistics
    /// Total play count
    pub play_count: i32,

    /// Total skip count
    pub skip_count: i32,

    /// Last played timestamp
    pub last_played_at: Option<DateTime<Utc>>,

    // Timestamps
    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Track {
    /// Returns a formatted duration string (e.g., "3:45")
    pub fn formatted_duration(&self) -> String {
        let total_seconds = self.duration_ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{}:{:02}", minutes, seconds)
    }

    /// Returns whether this track has Hi-Res audio
    pub fn is_hires(&self) -> bool {
        self.file_format.is_lossless()
            && self.sample_rate.is_some_and(|sr| sr > 44100)
            && self.bit_depth.is_some_and(|bd| bd > 16)
    }
}

/// Track creation input
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTrack {
    pub title: String,
    pub album_id: Option<Uuid>,
    pub artist_id: Uuid,
    pub file_path: String,
    pub file_size: i64,
    pub file_format: AudioFormat,
    pub duration_ms: i32,
    pub track_number: Option<i16>,
    pub disc_number: Option<i16>,
    pub genres: Option<Vec<String>>,
    pub explicit: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_is_lossless() {
        assert!(AudioFormat::Flac.is_lossless());
        assert!(AudioFormat::Wav.is_lossless());
        assert!(AudioFormat::Alac.is_lossless());
        assert!(!AudioFormat::Mp3.is_lossless());
        assert!(!AudioFormat::Aac.is_lossless());
    }

    #[test]
    fn test_audio_format_extension() {
        assert_eq!(AudioFormat::Flac.extension(), "flac");
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Aac.extension(), "m4a");
    }

    #[test]
    fn test_track_formatted_duration() {
        let mut track = create_test_track();
        track.duration_ms = 225000; // 3:45
        assert_eq!(track.formatted_duration(), "3:45");

        track.duration_ms = 60000; // 1:00
        assert_eq!(track.formatted_duration(), "1:00");

        track.duration_ms = 5000; // 0:05
        assert_eq!(track.formatted_duration(), "0:05");
    }

    #[test]
    fn test_track_is_hires() {
        let mut track = create_test_track();

        // Standard CD quality - not Hi-Res
        track.file_format = AudioFormat::Flac;
        track.sample_rate = Some(44100);
        track.bit_depth = Some(16);
        assert!(!track.is_hires());

        // Hi-Res: 24-bit 96kHz FLAC
        track.sample_rate = Some(96000);
        track.bit_depth = Some(24);
        assert!(track.is_hires());

        // MP3 can't be Hi-Res
        track.file_format = AudioFormat::Mp3;
        assert!(!track.is_hires());
    }

    fn create_test_track() -> Track {
        Track {
            id: Uuid::new_v4(),
            title: "Test Track".to_string(),
            album_id: Some(Uuid::new_v4()),
            artist_id: Uuid::new_v4(),
            mbid: None,
            file_path: "/music/test.flac".to_string(),
            file_size: 30000000,
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
}
