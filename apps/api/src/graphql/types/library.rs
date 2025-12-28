//! Library-related GraphQL types and enums
//!
//! This module defines shared enums for the music library.

use async_graphql::Enum;

use crate::models::album::AlbumType as DbAlbumType;
use crate::models::playlist::PlaylistType as DbPlaylistType;
use crate::models::track::AudioFormat as DbAudioFormat;

/// Album type enum for GraphQL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum AlbumType {
    /// Full-length album
    Album,
    /// Single release
    Single,
    /// Extended play
    Ep,
    /// Compilation album
    Compilation,
    /// Live recording
    Live,
    /// Remix album
    Remix,
    /// Soundtrack
    Soundtrack,
    /// Other type
    Other,
}

impl From<DbAlbumType> for AlbumType {
    fn from(album_type: DbAlbumType) -> Self {
        match album_type {
            DbAlbumType::Album => Self::Album,
            DbAlbumType::Single => Self::Single,
            DbAlbumType::Ep => Self::Ep,
            DbAlbumType::Compilation => Self::Compilation,
            DbAlbumType::Live => Self::Live,
            DbAlbumType::Remix => Self::Remix,
            DbAlbumType::Soundtrack => Self::Soundtrack,
            DbAlbumType::Other => Self::Other,
        }
    }
}

/// Audio format enum for GraphQL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum AudioFormat {
    /// FLAC lossless
    Flac,
    /// MP3 lossy
    Mp3,
    /// AAC lossy
    Aac,
    /// Opus lossy
    Opus,
    /// OGG Vorbis
    Ogg,
    /// WAV uncompressed
    Wav,
    /// Apple Lossless
    Alac,
    /// Other format
    Other,
}

impl From<DbAudioFormat> for AudioFormat {
    fn from(format: DbAudioFormat) -> Self {
        match format {
            DbAudioFormat::Flac => Self::Flac,
            DbAudioFormat::Mp3 => Self::Mp3,
            DbAudioFormat::Aac => Self::Aac,
            DbAudioFormat::Opus => Self::Opus,
            DbAudioFormat::Ogg => Self::Ogg,
            DbAudioFormat::Wav => Self::Wav,
            DbAudioFormat::Alac => Self::Alac,
            DbAudioFormat::Other => Self::Other,
        }
    }
}

/// Playlist type enum for GraphQL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum PlaylistType {
    /// Manually curated playlist
    Manual,
    /// Smart playlist with rules
    Smart,
    /// Discovery/recommendation playlist
    Discover,
    /// Radio-style continuous playlist
    Radio,
}

impl From<DbPlaylistType> for PlaylistType {
    fn from(playlist_type: DbPlaylistType) -> Self {
        match playlist_type {
            DbPlaylistType::Manual => Self::Manual,
            DbPlaylistType::Smart => Self::Smart,
            DbPlaylistType::Discover => Self::Discover,
            DbPlaylistType::Radio => Self::Radio,
        }
    }
}
