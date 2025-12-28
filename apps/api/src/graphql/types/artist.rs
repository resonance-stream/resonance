//! Artist GraphQL type
//!
//! This module defines the GraphQL type for artists with relationship resolvers.

use async_graphql::dataloader::DataLoader;
use async_graphql::{Context, Object, Result};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::graphql::loaders::{AlbumsByArtistLoader, TracksByArtistLoader};
use crate::graphql::pagination::{clamp_limit, clamp_offset, MAX_NESTED_LIMIT};
use crate::models::Artist as DbArtist;
use crate::repositories::AlbumRepository;

use super::album::Album;
use super::track::Track;

/// Artist information exposed via GraphQL
pub struct Artist {
    inner: DbArtist,
}

impl Artist {
    /// Create a new GraphQL Artist from a database Artist
    pub fn new(artist: DbArtist) -> Self {
        Self { inner: artist }
    }
}

impl From<DbArtist> for Artist {
    fn from(artist: DbArtist) -> Self {
        Self::new(artist)
    }
}

#[Object]
impl Artist {
    /// Unique artist identifier
    async fn id(&self) -> Uuid {
        self.inner.id
    }

    /// Artist name
    async fn name(&self) -> &str {
        &self.inner.name
    }

    /// Sort name for alphabetical ordering
    async fn sort_name(&self) -> Option<&str> {
        self.inner.sort_name.as_deref()
    }

    /// MusicBrainz identifier
    async fn mbid(&self) -> Option<Uuid> {
        self.inner.mbid
    }

    /// Artist biography/description
    async fn biography(&self) -> Option<&str> {
        self.inner.biography.as_deref()
    }

    /// URL to artist image
    async fn image_url(&self) -> Option<&str> {
        self.inner.image_url.as_deref()
    }

    /// Genre tags
    async fn genres(&self) -> &[String] {
        &self.inner.genres
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

    /// Albums by this artist (uses DataLoader for batched fetching)
    ///
    /// When fetching albums for many artists, requests are batched into a single query.
    /// Pagination is applied client-side from the cached results.
    async fn albums(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Album>> {
        let loader = ctx.data::<DataLoader<AlbumsByArtistLoader>>()?;
        let albums = loader.load_one(self.inner.id).await?;

        // Apply pagination with limits
        let limit = clamp_limit(limit, MAX_NESTED_LIMIT) as usize;
        let offset = clamp_offset(offset) as usize;

        Ok(albums
            .unwrap_or_default()
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(Album::from)
            .collect())
    }

    /// Total number of albums (efficient COUNT query)
    async fn album_count(&self, ctx: &Context<'_>) -> Result<i32> {
        let repo = ctx.data::<AlbumRepository>()?;
        let count = repo.count_by_artist(self.inner.id).await?;
        Ok(count as i32)
    }

    /// Top tracks by this artist (by play count, uses DataLoader for batched fetching)
    ///
    /// When fetching top tracks for many artists, requests are batched into a single query.
    /// Tracks are ordered by play_count descending.
    async fn top_tracks(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 10)] limit: i32,
    ) -> Result<Vec<Track>> {
        let loader = ctx.data::<DataLoader<TracksByArtistLoader>>()?;
        let tracks = loader.load_one(self.inner.id).await?;

        // Apply pagination limit
        let limit = clamp_limit(limit, MAX_NESTED_LIMIT) as usize;

        Ok(tracks
            .unwrap_or_default()
            .into_iter()
            .take(limit)
            .map(Track::from)
            .collect())
    }
}
