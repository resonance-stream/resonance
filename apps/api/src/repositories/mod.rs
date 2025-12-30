//! Database repository layer for Resonance
//!
//! This module provides the data access layer, centralizing all database
//! operations into reusable repositories. This pattern:
//! - Reduces code duplication across services and middleware
//! - Provides a single source of truth for database queries
//! - Makes testing easier through dependency injection
//! - Keeps SQL queries consistent across the codebase

// Allow unused imports - some repositories are prepared for future features
#![allow(unused_imports)]

pub mod album;
pub mod artist;
pub mod device;
pub mod playlist;
pub mod queue;
pub mod session;
pub mod track;
pub mod user;
pub mod utils;

pub use album::AlbumRepository;
pub use artist::ArtistRepository;
pub use device::DeviceRepository;
pub use playlist::PlaylistRepository;
pub use queue::{QueueError, QueueRepository, QueueResult};
pub use session::SessionRepository;
pub use track::{TrackRepository, TrackScrobbleInfo};
pub use user::UserRepository;
