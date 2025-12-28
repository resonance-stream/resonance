//! Playlist models for Resonance
//!
//! This module contains the database models for playlists,
//! including smart playlists with rule-based track selection.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Playlist type enum matching PostgreSQL playlist_type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "playlist_type", rename_all = "lowercase")]
pub enum PlaylistType {
    #[default]
    Manual,
    Smart,
    Discover,
    Radio,
}

/// Smart playlist rule for automatic track selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartPlaylistRule {
    /// Field to match (genre, artist, mood, etc.)
    pub field: String,
    /// Operator (equals, contains, greater_than, etc.)
    pub operator: String,
    /// Value to match against
    pub value: serde_json::Value,
}

/// Smart playlist rules configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmartPlaylistRules {
    /// Match mode: "all" or "any"
    pub match_mode: String,
    /// List of rules to apply
    pub rules: Vec<SmartPlaylistRule>,
    /// Maximum number of tracks
    pub limit: Option<i32>,
    /// Sort order field
    pub sort_by: Option<String>,
    /// Sort direction: "asc" or "desc"
    pub sort_order: Option<String>,
}

/// Playlist record from the playlists table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Playlist {
    /// Unique playlist identifier
    pub id: Uuid,

    /// User who owns this playlist
    pub user_id: Uuid,

    /// Playlist name
    pub name: String,

    /// Playlist description
    pub description: Option<String>,

    /// URL to playlist cover image
    pub image_url: Option<String>,

    /// Whether playlist is publicly visible
    pub is_public: bool,

    /// Whether other users can add tracks
    pub is_collaborative: bool,

    /// Type of playlist (manual, smart, discover, radio)
    pub playlist_type: PlaylistType,

    /// Smart playlist rules (for smart playlists)
    #[sqlx(json)]
    pub smart_rules: Option<SmartPlaylistRules>,

    /// Number of tracks in playlist
    pub track_count: i32,

    /// Total duration in milliseconds
    pub total_duration_ms: i64,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Playlist {
    /// Returns a formatted total duration (e.g., "1 hr 23 min")
    pub fn formatted_duration(&self) -> String {
        let total_seconds = self.total_duration_ms / 1000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;

        if hours > 0 {
            format!("{} hr {} min", hours, minutes)
        } else {
            format!("{} min", minutes)
        }
    }
}

/// Playlist track relationship from playlist_tracks table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PlaylistTrack {
    /// Unique relationship identifier
    pub id: Uuid,

    /// Playlist this track belongs to
    pub playlist_id: Uuid,

    /// Track in the playlist
    pub track_id: Uuid,

    /// User who added this track (for collaborative playlists)
    pub added_by: Option<Uuid>,

    /// Position in playlist (for ordering)
    pub position: i32,

    /// When the track was added
    pub added_at: DateTime<Utc>,
}

/// Playlist collaborator from playlist_collaborators table
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PlaylistCollaborator {
    /// Playlist being collaborated on
    pub playlist_id: Uuid,

    /// User who is a collaborator
    pub user_id: Uuid,

    /// Whether the collaborator can edit (add/remove tracks)
    pub can_edit: bool,

    /// When the collaborator was added
    pub added_at: DateTime<Utc>,
}

/// Playlist creation input
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePlaylist {
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub is_collaborative: Option<bool>,
    pub playlist_type: Option<PlaylistType>,
    pub smart_rules: Option<SmartPlaylistRules>,
}

/// Playlist update input
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePlaylist {
    pub name: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub is_public: Option<bool>,
    pub is_collaborative: Option<bool>,
    pub smart_rules: Option<SmartPlaylistRules>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playlist_type_default() {
        assert_eq!(PlaylistType::default(), PlaylistType::Manual);
    }

    #[test]
    fn test_playlist_formatted_duration() {
        let mut playlist = create_test_playlist();

        // 1 hour 23 minutes
        playlist.total_duration_ms = 4980000;
        assert_eq!(playlist.formatted_duration(), "1 hr 23 min");

        // 45 minutes
        playlist.total_duration_ms = 2700000;
        assert_eq!(playlist.formatted_duration(), "45 min");

        // 2 hours 0 minutes
        playlist.total_duration_ms = 7200000;
        assert_eq!(playlist.formatted_duration(), "2 hr 0 min");
    }

    #[test]
    fn test_smart_playlist_rules_serialization() {
        let rules = SmartPlaylistRules {
            match_mode: "all".to_string(),
            rules: vec![
                SmartPlaylistRule {
                    field: "genre".to_string(),
                    operator: "contains".to_string(),
                    value: serde_json::json!("Rock"),
                },
                SmartPlaylistRule {
                    field: "energy".to_string(),
                    operator: "greater_than".to_string(),
                    value: serde_json::json!(0.7),
                },
            ],
            limit: Some(50),
            sort_by: Some("added_at".to_string()),
            sort_order: Some("desc".to_string()),
        };

        let json = serde_json::to_string(&rules).expect("serialization should succeed");
        assert!(json.contains("genre"));
        assert!(json.contains("greater_than"));
    }

    fn create_test_playlist() -> Playlist {
        Playlist {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Test Playlist".to_string(),
            description: Some("A test playlist".to_string()),
            image_url: None,
            is_public: false,
            is_collaborative: false,
            playlist_type: PlaylistType::Manual,
            smart_rules: None,
            track_count: 10,
            total_duration_ms: 3600000,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
