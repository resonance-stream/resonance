//! Albums-by-Artist DataLoader for batched fetching
//!
//! This loader batches multiple artist ID lookups into a single database query,
//! returning all albums for each artist. This solves the N+1 problem when
//! loading albums for multiple artists.

use async_graphql::dataloader::Loader;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::Album;
use crate::repositories::utils::ALBUM_COLUMNS;

/// DataLoader for batching albums-by-artist queries
#[derive(Clone)]
pub struct AlbumsByArtistLoader {
    pool: PgPool,
}

impl AlbumsByArtistLoader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Loader<Uuid> for AlbumsByArtistLoader {
    type Value = Vec<Album>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // Guard against empty keys to avoid unnecessary database query
        if keys.is_empty() {
            return Ok(HashMap::new());
        }

        let sql = format!(
            "SELECT {} FROM albums WHERE artist_id = ANY($1) ORDER BY artist_id, release_date DESC NULLS LAST",
            ALBUM_COLUMNS
        );
        let albums: Vec<Album> = sqlx::query_as(&sql)
            .bind(keys)
            .fetch_all(&self.pool)
            .await
            .map_err(Arc::new)?;

        // Group albums by artist_id
        let mut result: HashMap<Uuid, Vec<Album>> = HashMap::new();
        for album in albums {
            result.entry(album.artist_id).or_default().push(album);
        }

        // Ensure all requested keys have an entry (even if empty)
        for key in keys {
            result.entry(*key).or_default();
        }

        Ok(result)
    }
}
