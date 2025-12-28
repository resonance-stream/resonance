//! Artist model for Resonance
//!
//! This module contains the database model for artists
//! with MusicBrainz and Lidarr integration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Artist record from the artists table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Artist {
    /// Unique artist identifier
    pub id: Uuid,

    /// Artist name
    pub name: String,

    /// Sort name for alphabetical ordering
    pub sort_name: Option<String>,

    /// MusicBrainz identifier
    pub mbid: Option<Uuid>,

    /// Lidarr artist ID for integration
    pub lidarr_id: Option<i32>,

    /// Artist biography/description
    pub biography: Option<String>,

    /// URL to artist image
    pub image_url: Option<String>,

    /// Genre tags
    pub genres: Vec<String>,

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

/// Artist creation input
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct CreateArtist {
    pub name: String,
    pub sort_name: Option<String>,
    pub mbid: Option<Uuid>,
    pub lidarr_id: Option<i32>,
    pub biography: Option<String>,
    pub image_url: Option<String>,
    pub genres: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artist_serialization() {
        let artist = Artist {
            id: Uuid::new_v4(),
            name: "Test Artist".to_string(),
            sort_name: Some("Artist, Test".to_string()),
            mbid: Some(Uuid::new_v4()),
            lidarr_id: Some(123),
            biography: Some("A test artist".to_string()),
            image_url: Some("https://example.com/artist.jpg".to_string()),
            genres: vec!["Rock".to_string(), "Alternative".to_string()],
            external_urls: serde_json::json!({"spotify": "https://spotify.com/artist/123"}),
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&artist).expect("serialization should succeed");
        assert!(json.contains("Test Artist"));
        assert!(json.contains("Rock"));
    }
}
