//! GraphQL type definitions for Resonance
//!
//! This module contains the GraphQL object types that are exposed
//! through the API, including user and authentication types.

// Re-exports for public API - some types not yet consumed externally
#![allow(unused_imports)]

mod album;
mod artist;
mod library;
mod playlist;
mod search;
mod track;
mod user;

pub use album::{Album, CoverArtColors};
pub use artist::Artist;
pub use library::{AlbumType, AudioFormat, PlaylistType};
pub use playlist::{
    Playlist, PlaylistTrackEntry, SmartPlaylistMatchMode, SmartPlaylistRule, SmartPlaylistRules,
    SortOrder,
};
pub use search::{ArtistTag, MoodTag, ScoredTrack, SemanticSearchResult, SimilarArtist};
pub use track::{AudioFeatures, Track};
pub use user::{AuthPayload, RefreshPayload, User, UserRole};
