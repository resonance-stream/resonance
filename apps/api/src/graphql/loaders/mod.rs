//! DataLoader implementations for GraphQL
//!
//! This module provides DataLoader implementations to solve N+1 query problems
//! in GraphQL relationship resolvers. Each loader batches multiple requests
//! into a single database query.
//!
//! There are two types of loaders:
//! - Single-entity loaders: Return `Option<T>` for a single entity by ID
//! - Collection loaders: Return `Vec<T>` for related entities by parent ID

mod album;
mod albums_by_artist;
mod artist;
mod track;
mod tracks_by_album;
mod tracks_by_artist;

pub use album::AlbumLoader;
pub use albums_by_artist::AlbumsByArtistLoader;
pub use artist::ArtistLoader;
pub use track::TrackLoader;
pub use tracks_by_album::TracksByAlbumLoader;
pub use tracks_by_artist::TracksByArtistLoader;

use sqlx::PgPool;

/// Create all data loaders for the GraphQL schema
///
/// This is a convenience function for creating all loaders at once.
/// Currently the schema builder creates loaders individually for more
/// flexibility, but this function is available for simplified setup.
#[allow(dead_code)]
pub fn create_loaders(pool: PgPool) -> Loaders {
    Loaders {
        artist: ArtistLoader::new(pool.clone()),
        album: AlbumLoader::new(pool.clone()),
        track: TrackLoader::new(pool.clone()),
        albums_by_artist: AlbumsByArtistLoader::new(pool.clone()),
        tracks_by_album: TracksByAlbumLoader::new(pool.clone()),
        tracks_by_artist: TracksByArtistLoader::new(pool),
    }
}

/// Container for all DataLoader instances
///
/// This struct bundles all loaders for convenient passing around.
/// Currently the schema builder injects loaders individually, but this
/// is available for alternative patterns or future use.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Loaders {
    pub artist: ArtistLoader,
    pub album: AlbumLoader,
    pub track: TrackLoader,
    pub albums_by_artist: AlbumsByArtistLoader,
    pub tracks_by_album: TracksByAlbumLoader,
    pub tracks_by_artist: TracksByArtistLoader,
}
