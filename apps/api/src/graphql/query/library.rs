//! Library queries for Resonance GraphQL API
//!
//! This module provides queries for browsing the music library:
//! - Artists: List and search artists
//! - Albums: List and search albums
//! - Tracks: List and search tracks

use async_graphql::{Context, Object, Result, ID};
use uuid::Uuid;

use crate::graphql::pagination::{clamp_limit, clamp_offset, MAX_LIMIT, MAX_SEARCH_LIMIT};
use crate::graphql::types::{Album, Artist, Track};
use crate::repositories::{AlbumRepository, ArtistRepository, TrackRepository};

/// Library-related queries for browsing artists, albums, and tracks
#[derive(Default)]
pub struct LibraryQuery;

#[Object]
impl LibraryQuery {
    // ==================== Artist Queries ====================

    /// Get an artist by ID
    async fn artist(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Artist>> {
        let repo = ctx.data::<ArtistRepository>()?;
        let uuid = Uuid::parse_str(&id)?;
        let artist = repo.find_by_id(uuid).await?;
        Ok(artist.map(Artist::from))
    }

    /// List all artists with pagination
    async fn artists(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Artist>> {
        let repo = ctx.data::<ArtistRepository>()?;
        let artists = repo
            .find_all(clamp_limit(limit, MAX_LIMIT), clamp_offset(offset))
            .await?;
        Ok(artists.into_iter().map(Artist::from).collect())
    }

    /// Search artists by name
    async fn search_artists(
        &self,
        ctx: &Context<'_>,
        query: String,
        #[graphql(default = 20)] limit: i32,
    ) -> Result<Vec<Artist>> {
        let repo = ctx.data::<ArtistRepository>()?;
        let artists = repo
            .search(&query, clamp_limit(limit, MAX_SEARCH_LIMIT))
            .await?;
        Ok(artists.into_iter().map(Artist::from).collect())
    }

    /// Get artists by genre
    async fn artists_by_genre(
        &self,
        ctx: &Context<'_>,
        genre: String,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Artist>> {
        let repo = ctx.data::<ArtistRepository>()?;
        let artists = repo
            .find_by_genre(&genre, clamp_limit(limit, MAX_LIMIT), clamp_offset(offset))
            .await?;
        Ok(artists.into_iter().map(Artist::from).collect())
    }

    // ==================== Album Queries ====================

    /// Get an album by ID
    async fn album(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Album>> {
        let repo = ctx.data::<AlbumRepository>()?;
        let uuid = Uuid::parse_str(&id)?;
        let album = repo.find_by_id(uuid).await?;
        Ok(album.map(Album::from))
    }

    /// List all albums with pagination
    async fn albums(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Album>> {
        let repo = ctx.data::<AlbumRepository>()?;
        let albums = repo
            .find_all(clamp_limit(limit, MAX_LIMIT), clamp_offset(offset))
            .await?;
        Ok(albums.into_iter().map(Album::from).collect())
    }

    /// Get albums by artist
    async fn albums_by_artist(
        &self,
        ctx: &Context<'_>,
        artist_id: ID,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Album>> {
        let repo = ctx.data::<AlbumRepository>()?;
        let uuid = Uuid::parse_str(&artist_id)?;
        let albums = repo
            .find_by_artist(uuid, clamp_limit(limit, MAX_LIMIT), clamp_offset(offset))
            .await?;
        Ok(albums.into_iter().map(Album::from).collect())
    }

    /// Search albums by title
    async fn search_albums(
        &self,
        ctx: &Context<'_>,
        query: String,
        #[graphql(default = 20)] limit: i32,
    ) -> Result<Vec<Album>> {
        let repo = ctx.data::<AlbumRepository>()?;
        let albums = repo
            .search(&query, clamp_limit(limit, MAX_SEARCH_LIMIT))
            .await?;
        Ok(albums.into_iter().map(Album::from).collect())
    }

    /// Get recently added albums
    async fn recent_albums(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i32,
    ) -> Result<Vec<Album>> {
        let repo = ctx.data::<AlbumRepository>()?;
        let albums = repo
            .find_recent(clamp_limit(limit, MAX_SEARCH_LIMIT))
            .await?;
        Ok(albums.into_iter().map(Album::from).collect())
    }

    // ==================== Track Queries ====================

    /// Get a track by ID
    async fn track(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Track>> {
        let repo = ctx.data::<TrackRepository>()?;
        let uuid = Uuid::parse_str(&id)?;
        let track = repo.find_by_id(uuid).await?;
        Ok(track.map(Track::from))
    }

    /// List all tracks with pagination
    async fn tracks(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Track>> {
        let repo = ctx.data::<TrackRepository>()?;
        let tracks = repo
            .find_all(clamp_limit(limit, MAX_LIMIT), clamp_offset(offset))
            .await?;
        Ok(tracks.into_iter().map(Track::from).collect())
    }

    /// Get tracks by album
    async fn tracks_by_album(
        &self,
        ctx: &Context<'_>,
        album_id: ID,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Track>> {
        let repo = ctx.data::<TrackRepository>()?;
        let uuid = Uuid::parse_str(&album_id)?;
        let tracks = repo
            .find_by_album_paginated(uuid, clamp_limit(limit, MAX_LIMIT), clamp_offset(offset))
            .await?;
        Ok(tracks.into_iter().map(Track::from).collect())
    }

    /// Get tracks by artist
    async fn tracks_by_artist(
        &self,
        ctx: &Context<'_>,
        artist_id: ID,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Track>> {
        let repo = ctx.data::<TrackRepository>()?;
        let uuid = Uuid::parse_str(&artist_id)?;
        let tracks = repo
            .find_by_artist(uuid, clamp_limit(limit, MAX_LIMIT), clamp_offset(offset))
            .await?;
        Ok(tracks.into_iter().map(Track::from).collect())
    }

    /// Search tracks by title
    async fn search_tracks(
        &self,
        ctx: &Context<'_>,
        query: String,
        #[graphql(default = 20)] limit: i32,
    ) -> Result<Vec<Track>> {
        let repo = ctx.data::<TrackRepository>()?;
        let tracks = repo
            .search(&query, clamp_limit(limit, MAX_SEARCH_LIMIT))
            .await?;
        Ok(tracks.into_iter().map(Track::from).collect())
    }

    /// Get top played tracks globally
    async fn top_tracks(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: i32,
    ) -> Result<Vec<Track>> {
        let repo = ctx.data::<TrackRepository>()?;
        let tracks = repo.find_top_tracks(clamp_limit(limit, MAX_LIMIT)).await?;
        Ok(tracks.into_iter().map(Track::from).collect())
    }
}
