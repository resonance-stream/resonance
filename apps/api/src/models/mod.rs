//! Database models and types for Resonance
//!
//! This module contains SQLx models for:
//! - Users and authentication
//! - Artists, albums, and tracks
//! - Playlists and user library
//! - Recommendations and AI features

pub mod user;

// Re-export commonly used types for external consumers
#[allow(unused_imports)]
pub use user::{
    AuthTokens, Claims, DeviceInfo, DeviceType, PublicUser, RefreshClaims, Session, User,
    UserPreferences, UserRole,
};

// Future modules:
// pub mod artist;
// pub mod album;
// pub mod track;
// pub mod playlist;
// pub mod listening_history;
