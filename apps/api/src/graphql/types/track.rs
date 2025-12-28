//! Track GraphQL type
//!
//! This module defines the GraphQL type for tracks with relationship resolvers.

use async_graphql::dataloader::DataLoader;
use async_graphql::{Context, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::graphql::loaders::{AlbumLoader, ArtistLoader};
use crate::models::track::AudioFeatures as DbAudioFeatures;
use crate::models::Track as DbTrack;

use super::album::Album;
use super::artist::Artist;
use super::library::AudioFormat;

/// Audio features extracted from the track
#[derive(Debug, Clone, SimpleObject)]
pub struct AudioFeatures {
    /// Beats per minute
    pub bpm: Option<f64>,
    /// Musical key
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

impl From<DbAudioFeatures> for AudioFeatures {
    fn from(features: DbAudioFeatures) -> Self {
        Self {
            bpm: features.bpm,
            key: features.key,
            mode: features.mode,
            loudness: features.loudness,
            energy: features.energy,
            danceability: features.danceability,
            valence: features.valence,
            acousticness: features.acousticness,
            instrumentalness: features.instrumentalness,
            speechiness: features.speechiness,
        }
    }
}

/// Track information exposed via GraphQL
pub struct Track {
    inner: DbTrack,
}

impl Track {
    /// Create a new GraphQL Track from a database Track
    pub fn new(track: DbTrack) -> Self {
        Self { inner: track }
    }
}

impl From<DbTrack> for Track {
    fn from(track: DbTrack) -> Self {
        Self::new(track)
    }
}

#[Object]
impl Track {
    /// Unique track identifier
    async fn id(&self) -> Uuid {
        self.inner.id
    }

    /// Track title
    async fn title(&self) -> &str {
        &self.inner.title
    }

    /// Album ID (if part of an album)
    async fn album_id(&self) -> Option<Uuid> {
        self.inner.album_id
    }

    /// Artist ID
    async fn artist_id(&self) -> Uuid {
        self.inner.artist_id
    }

    /// MusicBrainz identifier
    async fn mbid(&self) -> Option<Uuid> {
        self.inner.mbid
    }

    /// Audio format
    async fn file_format(&self) -> AudioFormat {
        self.inner.file_format.into()
    }

    /// Duration in milliseconds
    async fn duration_ms(&self) -> i32 {
        self.inner.duration_ms
    }

    /// Formatted duration (e.g., "3:45")
    async fn formatted_duration(&self) -> String {
        self.inner.formatted_duration()
    }

    /// Bit rate in kbps
    async fn bit_rate(&self) -> Option<i32> {
        self.inner.bit_rate
    }

    /// Sample rate in Hz
    async fn sample_rate(&self) -> Option<i32> {
        self.inner.sample_rate
    }

    /// Bit depth
    async fn bit_depth(&self) -> Option<i16> {
        self.inner.bit_depth
    }

    /// Whether this is Hi-Res audio
    async fn is_hires(&self) -> bool {
        self.inner.is_hires()
    }

    /// Whether this format is lossless
    async fn is_lossless(&self) -> bool {
        self.inner.file_format.is_lossless()
    }

    /// Track number on album
    async fn track_number(&self) -> Option<i16> {
        self.inner.track_number
    }

    /// Disc number
    async fn disc_number(&self) -> Option<i16> {
        self.inner.disc_number
    }

    /// Genre tags
    async fn genres(&self) -> &[String] {
        &self.inner.genres
    }

    /// Explicit content flag
    async fn explicit(&self) -> bool {
        self.inner.explicit
    }

    /// Static lyrics text
    async fn lyrics(&self) -> Option<&str> {
        self.inner.lyrics.as_deref()
    }

    /// Whether synced lyrics are available
    async fn has_synced_lyrics(&self) -> bool {
        self.inner.synced_lyrics.is_some()
    }

    /// Audio features
    async fn audio_features(&self) -> AudioFeatures {
        self.inner.audio_features.clone().into()
    }

    /// AI-detected mood tags
    async fn ai_mood(&self) -> &[String] {
        &self.inner.ai_mood
    }

    /// AI-generated descriptive tags
    async fn ai_tags(&self) -> &[String] {
        &self.inner.ai_tags
    }

    /// AI-generated description
    async fn ai_description(&self) -> Option<&str> {
        self.inner.ai_description.as_deref()
    }

    /// Total play count
    async fn play_count(&self) -> i32 {
        self.inner.play_count
    }

    /// Total skip count
    async fn skip_count(&self) -> i32 {
        self.inner.skip_count
    }

    /// Last played timestamp
    async fn last_played_at(&self) -> Option<DateTime<Utc>> {
        self.inner.last_played_at
    }

    /// Stream URL for this track
    async fn stream_url(&self) -> String {
        format!("/api/stream/{}", self.inner.id)
    }

    /// Creation timestamp
    async fn created_at(&self) -> DateTime<Utc> {
        self.inner.created_at
    }

    /// Last update timestamp
    async fn updated_at(&self) -> DateTime<Utc> {
        self.inner.updated_at
    }

    // Relationship resolvers (using DataLoader for batched fetching)

    /// Album this track belongs to
    async fn album(&self, ctx: &Context<'_>) -> Result<Option<Album>> {
        if let Some(album_id) = self.inner.album_id {
            let loader = ctx.data::<DataLoader<AlbumLoader>>()?;
            let album = loader.load_one(album_id).await?;
            Ok(album.map(Album::from))
        } else {
            Ok(None)
        }
    }

    /// Artist who created this track
    async fn artist(&self, ctx: &Context<'_>) -> Result<Option<Artist>> {
        let loader = ctx.data::<DataLoader<ArtistLoader>>()?;
        let artist = loader.load_one(self.inner.artist_id).await?;
        Ok(artist.map(Artist::from))
    }
}
