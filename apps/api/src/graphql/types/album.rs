//! Album GraphQL type
//!
//! This module defines the GraphQL type for albums with relationship resolvers.

use async_graphql::dataloader::DataLoader;
use async_graphql::{Context, Object, Result, SimpleObject};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use uuid::Uuid;

use crate::graphql::loaders::{ArtistLoader, TracksByAlbumLoader};
use crate::graphql::pagination::{clamp_limit, clamp_offset, MAX_NESTED_LIMIT};
use crate::models::album::CoverArtColors as DbCoverArtColors;
use crate::models::Album as DbAlbum;

use super::artist::Artist;
use super::library::AlbumType;
use super::track::Track;

/// Cover art color palette
#[derive(Debug, Clone, SimpleObject)]
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

impl From<DbCoverArtColors> for CoverArtColors {
    fn from(colors: DbCoverArtColors) -> Self {
        Self {
            primary: colors.primary,
            secondary: colors.secondary,
            accent: colors.accent,
            vibrant: colors.vibrant,
            muted: colors.muted,
        }
    }
}

/// Album information exposed via GraphQL
pub struct Album {
    inner: DbAlbum,
}

impl Album {
    /// Create a new GraphQL Album from a database Album
    pub fn new(album: DbAlbum) -> Self {
        Self { inner: album }
    }
}

impl From<DbAlbum> for Album {
    fn from(album: DbAlbum) -> Self {
        Self::new(album)
    }
}

#[Object]
impl Album {
    /// Unique album identifier
    async fn id(&self) -> Uuid {
        self.inner.id
    }

    /// Album title
    async fn title(&self) -> &str {
        &self.inner.title
    }

    /// Artist ID
    async fn artist_id(&self) -> Uuid {
        self.inner.artist_id
    }

    /// MusicBrainz identifier
    async fn mbid(&self) -> Option<Uuid> {
        self.inner.mbid
    }

    /// Release date
    async fn release_date(&self) -> Option<NaiveDate> {
        self.inner.release_date
    }

    /// Release year (convenience field)
    async fn release_year(&self) -> Option<i32> {
        self.inner.release_date.map(|d| d.year())
    }

    /// Type of album
    async fn album_type(&self) -> AlbumType {
        self.inner.album_type.into()
    }

    /// Genre tags
    async fn genres(&self) -> &[String] {
        &self.inner.genres
    }

    /// Total number of tracks
    async fn total_tracks(&self) -> Option<i32> {
        self.inner.total_tracks
    }

    /// Total duration in milliseconds
    async fn total_duration_ms(&self) -> Option<i64> {
        self.inner.total_duration_ms
    }

    /// Formatted total duration (e.g., "45 min")
    async fn formatted_duration(&self) -> Option<String> {
        self.inner.total_duration_ms.map(|ms| {
            let total_seconds = ms / 1000;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;

            if hours > 0 {
                format!("{} hr {} min", hours, minutes)
            } else {
                format!("{} min", minutes)
            }
        })
    }

    /// Path to cover art file
    async fn cover_art_path(&self) -> Option<&str> {
        self.inner.cover_art_path.as_deref()
    }

    /// Cover art URL (for API access)
    async fn cover_art_url(&self) -> Option<String> {
        self.inner
            .cover_art_path
            .as_ref()
            .map(|_| format!("/api/albums/{}/cover", self.inner.id))
    }

    /// Extracted color palette for visualizer
    async fn cover_art_colors(&self) -> CoverArtColors {
        self.inner.cover_art_colors.clone().into()
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

    /// Artist who created this album (uses DataLoader for batched fetching)
    async fn artist(&self, ctx: &Context<'_>) -> Result<Option<Artist>> {
        let loader = ctx.data::<DataLoader<ArtistLoader>>()?;
        let artist = loader.load_one(self.inner.artist_id).await?;
        Ok(artist.map(Artist::from))
    }

    /// Tracks on this album (uses DataLoader for batched fetching)
    ///
    /// When fetching tracks for many albums, requests are batched into a single query.
    /// Pagination is applied client-side from the cached results.
    async fn tracks(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Track>> {
        let loader = ctx.data::<DataLoader<TracksByAlbumLoader>>()?;
        let tracks = loader.load_one(self.inner.id).await?;

        // Apply pagination with limits
        let limit = clamp_limit(limit, MAX_NESTED_LIMIT) as usize;
        let offset = clamp_offset(offset) as usize;

        Ok(tracks
            .unwrap_or_default()
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(Track::from)
            .collect())
    }
}
