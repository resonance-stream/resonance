//! GraphQL type definitions for Resonance
//!
//! This module contains the GraphQL object types that are exposed
//! through the API, including user and authentication types.

// Re-exports for public API - some types not yet consumed externally
#![allow(unused_imports)]

mod admin;
mod album;
mod artist;
pub mod chat;
mod library;
mod playlist;
mod search;
mod system_settings;
mod track;
mod user;

pub use admin::{AdminSession, AdminUserDetail, AdminUserList, AdminUserListItem, SystemStats};
pub use album::{Album, CoverArtColors};
pub use artist::Artist;
pub use chat::{ChatConversation, ChatConversationWithMessages, ChatMessage, ChatRole};
pub use library::{AlbumType, AudioFormat, PlaylistType};
pub use playlist::{
    Playlist, PlaylistTrackEntry, SmartPlaylistMatchMode, SmartPlaylistRule, SmartPlaylistRules,
    SortOrder,
};
pub use search::{
    ArtistTag, MoodTag, ScoredTrack, SemanticSearchResult, SimilarArtist, SimilarTrack,
    SimilarityMethod, SimilarityType,
};
pub use system_settings::{ServiceType, SetupStatus, SystemSettingInfo};
pub use track::{AudioFeatures, Track};
pub use user::{AuthPayload, RefreshPayload, User, UserPreferencesType, UserRole};
