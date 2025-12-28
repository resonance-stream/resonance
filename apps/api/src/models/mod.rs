//! Database models and types for Resonance
//!
//! This module contains SQLx models for:
//! - Users and authentication
//! - Artists, albums, and tracks
//! - Playlists and user library
//! - Recommendations and AI features

// Re-exports for public API - some types not yet consumed externally
#![allow(unused_imports)]

pub mod album;
pub mod artist;
pub mod playlist;
pub mod track;
pub mod user;

// Re-export commonly used types for external consumers
pub use album::{Album, AlbumType, CoverArtColors, CreateAlbum};
pub use artist::{Artist, CreateArtist};
pub use playlist::{
    CreatePlaylist, Playlist, PlaylistCollaborator, PlaylistTrack, PlaylistType,
    SmartPlaylistRules, UpdatePlaylist,
};
pub use track::{AudioFeatures, AudioFormat, CreateTrack, SyncedLyricLine, Track};
pub use user::{
    AuthTokens, Claims, DeviceInfo, DeviceType, PublicUser, RefreshClaims, RequestMetadata,
    Session, User, UserPreferences, UserRole,
};
