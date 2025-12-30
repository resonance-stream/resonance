//! Playlist GraphQL type
//!
//! This module defines the GraphQL type for playlists with relationship resolvers.

use async_graphql::dataloader::DataLoader;
use async_graphql::{Context, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::graphql::loaders::TrackLoader;
use crate::graphql::pagination::{clamp_limit, clamp_offset, MAX_PLAYLIST_TRACKS};
use crate::models::playlist::{
    SmartPlaylistRule as DbSmartPlaylistRule, SmartPlaylistRules as DbSmartPlaylistRules,
};
use crate::models::Playlist as DbPlaylist;
use crate::models::PlaylistTrack as DbPlaylistTrack;
use crate::repositories::PlaylistRepository;

use super::library::PlaylistType;
use super::track::Track;

// =============================================================================
// Smart Playlist GraphQL Types
// =============================================================================

/// Match mode for smart playlist rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, async_graphql::Enum)]
pub enum SmartPlaylistMatchMode {
    /// All rules must match (AND logic)
    All,
    /// Any rule can match (OR logic)
    Any,
}

impl From<&str> for SmartPlaylistMatchMode {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "any" => Self::Any,
            _ => Self::All, // Default to All for unknown values
        }
    }
}

/// Sort order for smart playlist results
#[derive(Debug, Clone, Copy, PartialEq, Eq, async_graphql::Enum)]
pub enum SortOrder {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

impl From<&str> for SortOrder {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "desc" => Self::Desc,
            _ => Self::Asc, // Default to Asc for unknown values
        }
    }
}

/// A single rule for smart playlist evaluation
#[derive(Debug, Clone, SimpleObject)]
pub struct SmartPlaylistRule {
    /// Field to match (genre, artist, mood, energy, similar_to, etc.)
    pub field: String,
    /// Operator (equals, contains, greater_than, combined, semantic, etc.)
    pub operator: String,
    /// Value to match against (can be string, number, array, or object)
    pub value: serde_json::Value,
}

impl From<DbSmartPlaylistRule> for SmartPlaylistRule {
    fn from(rule: DbSmartPlaylistRule) -> Self {
        Self {
            field: rule.field,
            operator: rule.operator,
            value: rule.value,
        }
    }
}

/// Smart playlist rules configuration
#[derive(Debug, Clone, SimpleObject)]
pub struct SmartPlaylistRules {
    /// Match mode: All (AND) or Any (OR)
    pub match_mode: SmartPlaylistMatchMode,
    /// List of rules to apply
    pub rules: Vec<SmartPlaylistRule>,
    /// Maximum number of tracks (optional)
    pub limit: Option<i32>,
    /// Field to sort by (optional)
    pub sort_by: Option<String>,
    /// Sort direction (optional)
    pub sort_order: Option<SortOrder>,
}

impl From<DbSmartPlaylistRules> for SmartPlaylistRules {
    fn from(rules: DbSmartPlaylistRules) -> Self {
        Self {
            match_mode: SmartPlaylistMatchMode::from(rules.match_mode.as_str()),
            rules: rules.rules.into_iter().map(Into::into).collect(),
            limit: rules.limit,
            sort_by: rules.sort_by,
            sort_order: rules.sort_order.as_deref().map(SortOrder::from),
        }
    }
}

// =============================================================================
// Playlist GraphQL Type
// =============================================================================

/// Playlist information exposed via GraphQL
pub struct Playlist {
    inner: DbPlaylist,
}

impl Playlist {
    /// Create a new GraphQL Playlist from a database Playlist
    pub fn new(playlist: DbPlaylist) -> Self {
        Self { inner: playlist }
    }
}

impl From<DbPlaylist> for Playlist {
    fn from(playlist: DbPlaylist) -> Self {
        Self::new(playlist)
    }
}

#[Object]
impl Playlist {
    /// Unique playlist identifier
    async fn id(&self) -> Uuid {
        self.inner.id
    }

    /// User who owns this playlist
    async fn user_id(&self) -> Uuid {
        self.inner.user_id
    }

    /// Playlist name
    async fn name(&self) -> &str {
        &self.inner.name
    }

    /// Playlist description
    async fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// URL to playlist cover image
    async fn image_url(&self) -> Option<&str> {
        self.inner.image_url.as_deref()
    }

    /// Whether playlist is publicly visible
    async fn is_public(&self) -> bool {
        self.inner.is_public
    }

    /// Whether other users can add tracks
    async fn is_collaborative(&self) -> bool {
        self.inner.is_collaborative
    }

    /// Type of playlist
    async fn playlist_type(&self) -> PlaylistType {
        self.inner.playlist_type.into()
    }

    /// Smart playlist rules (only present for smart playlists)
    async fn smart_rules(&self) -> Option<SmartPlaylistRules> {
        self.inner.smart_rules.clone().map(Into::into)
    }

    /// Number of tracks in playlist
    async fn track_count(&self) -> i32 {
        self.inner.track_count
    }

    /// Total duration in milliseconds
    async fn total_duration_ms(&self) -> i64 {
        self.inner.total_duration_ms
    }

    /// Formatted total duration
    async fn formatted_duration(&self) -> String {
        self.inner.formatted_duration()
    }

    /// Creation timestamp
    async fn created_at(&self) -> DateTime<Utc> {
        self.inner.created_at
    }

    /// Last update timestamp
    async fn updated_at(&self) -> DateTime<Utc> {
        self.inner.updated_at
    }

    // Relationship resolvers

    /// Tracks in this playlist (uses DataLoader for batched fetching)
    async fn tracks(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<PlaylistTrackEntry>> {
        // Enforce pagination limits using shared helpers
        let limit = clamp_limit(limit, MAX_PLAYLIST_TRACKS);
        let offset = clamp_offset(offset);

        let playlist_repo = ctx.data::<PlaylistRepository>()?;
        let track_loader = ctx.data::<DataLoader<TrackLoader>>()?;

        let playlist_tracks = playlist_repo
            .get_tracks(self.inner.id, limit, offset)
            .await?;

        // Batch load all tracks at once using DataLoader
        let track_ids: Vec<Uuid> = playlist_tracks.iter().map(|pt| pt.track_id).collect();
        let tracks = track_loader.load_many(track_ids).await?;

        // Build entries, preserving playlist order; skip missing tracks with warning
        let mut entries = Vec::with_capacity(playlist_tracks.len());
        for pt in playlist_tracks {
            let Some(track) = tracks.get(&pt.track_id) else {
                tracing::warn!(
                    playlist_id = %self.inner.id,
                    track_id = %pt.track_id,
                    "Playlist references missing track, skipping"
                );
                continue;
            };

            entries.push(PlaylistTrackEntry {
                playlist_track: pt,
                track: Track::from(track.clone()),
            });
        }

        Ok(entries)
    }
}

/// A track entry in a playlist with metadata
pub struct PlaylistTrackEntry {
    playlist_track: DbPlaylistTrack,
    track: Track,
}

#[Object]
impl PlaylistTrackEntry {
    /// The track
    async fn track(&self) -> &Track {
        &self.track
    }

    /// Position in playlist
    async fn position(&self) -> i32 {
        self.playlist_track.position
    }

    /// User who added this track
    async fn added_by(&self) -> Option<Uuid> {
        self.playlist_track.added_by
    }

    /// When the track was added
    async fn added_at(&self) -> DateTime<Utc> {
        self.playlist_track.added_at
    }
}

/// Statistics about a playlist
#[allow(dead_code)]
#[derive(Debug, Clone, SimpleObject)]
pub struct PlaylistStats {
    /// Number of tracks
    pub track_count: i32,
    /// Total duration in milliseconds
    pub total_duration_ms: i64,
    /// Formatted duration
    pub formatted_duration: String,
}
