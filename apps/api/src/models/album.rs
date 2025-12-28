//! Album model for Resonance
//!
//! This module contains the database model for albums
//! with cover art color extraction for visualizer.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Album type enum matching PostgreSQL album_type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "album_type", rename_all = "lowercase")]
pub enum AlbumType {
    #[default]
    Album,
    Single,
    Ep,
    Compilation,
    Live,
    Remix,
    Soundtrack,
    Other,
}

/// Cover art color palette extracted for visualizer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverArtColors {
    /// Primary dominant color
    pub primary: Option<String>,
    /// Secondary color
    pub secondary: Option<String>,
    /// Accent color
    pub accent: Option<String>,
    /// Most vibrant color
    pub vibrant: Option<String>,
    /// Most muted color
    pub muted: Option<String>,
}

/// Album record from the albums table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Album {
    /// Unique album identifier
    pub id: Uuid,

    /// Album title
    pub title: String,

    /// Artist who created this album
    pub artist_id: Uuid,

    /// MusicBrainz identifier
    pub mbid: Option<Uuid>,

    /// Lidarr album ID for integration
    pub lidarr_id: Option<i32>,

    /// Release date
    pub release_date: Option<NaiveDate>,

    /// Type of album (album, single, EP, etc.)
    pub album_type: AlbumType,

    /// Genre tags
    pub genres: Vec<String>,

    /// Total number of tracks
    pub total_tracks: Option<i32>,

    /// Total duration in milliseconds
    pub total_duration_ms: Option<i64>,

    /// Path to cover art file
    pub cover_art_path: Option<String>,

    /// Extracted color palette for visualizer
    #[sqlx(json)]
    pub cover_art_colors: CoverArtColors,

    /// External URLs (Spotify, Apple Music, etc.)
    #[sqlx(json)]
    pub external_urls: serde_json::Value,

    /// Additional metadata
    #[sqlx(json)]
    pub metadata: serde_json::Value,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Album creation input
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAlbum {
    pub title: String,
    pub artist_id: Uuid,
    pub mbid: Option<Uuid>,
    pub lidarr_id: Option<i32>,
    pub release_date: Option<NaiveDate>,
    pub album_type: Option<AlbumType>,
    pub genres: Option<Vec<String>>,
    pub cover_art_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_album_type_default() {
        assert_eq!(AlbumType::default(), AlbumType::Album);
    }

    #[test]
    fn test_cover_art_colors_default() {
        let colors = CoverArtColors::default();
        assert!(colors.primary.is_none());
        assert!(colors.secondary.is_none());
    }

    #[test]
    fn test_album_serialization() {
        let album = Album {
            id: Uuid::new_v4(),
            title: "Test Album".to_string(),
            artist_id: Uuid::new_v4(),
            mbid: None,
            lidarr_id: None,
            release_date: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
            album_type: AlbumType::Album,
            genres: vec!["Rock".to_string()],
            total_tracks: Some(12),
            total_duration_ms: Some(3600000),
            cover_art_path: Some("/covers/album.jpg".to_string()),
            cover_art_colors: CoverArtColors {
                primary: Some("#1a1a2e".to_string()),
                secondary: Some("#16213e".to_string()),
                accent: Some("#0f3460".to_string()),
                vibrant: Some("#e94560".to_string()),
                muted: Some("#533483".to_string()),
            },
            external_urls: serde_json::json!({}),
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&album).expect("serialization should succeed");
        assert!(json.contains("Test Album"));
        assert!(json.contains("#e94560"));
    }
}
