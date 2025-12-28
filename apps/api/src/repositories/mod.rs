//! Database repository layer for Resonance
//!
//! This module provides the data access layer, centralizing all database
//! operations into reusable repositories. This pattern:
//! - Reduces code duplication across services and middleware
//! - Provides a single source of truth for database queries
//! - Makes testing easier through dependency injection
//! - Keeps SQL queries consistent across the codebase

pub mod album;
pub mod artist;
pub mod playlist;
pub mod session;
pub mod track;
pub mod user;
pub mod utils;

pub use album::AlbumRepository;
pub use artist::ArtistRepository;
pub use playlist::PlaylistRepository;
pub use session::SessionRepository;
pub use track::TrackRepository;
pub use user::UserRepository;
