//! Album DataLoader for batched fetching
//!
//! This loader batches multiple album ID lookups into a single database query,
//! solving the N+1 problem when loading albums for multiple tracks.

use async_graphql::dataloader::Loader;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::Album;
use crate::repositories::utils::ALBUM_COLUMNS;

/// DataLoader for batching album queries
#[derive(Clone)]
pub struct AlbumLoader {
    pool: PgPool,
}

impl AlbumLoader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Loader<Uuid> for AlbumLoader {
    type Value = Album;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let sql = format!("SELECT {} FROM albums WHERE id = ANY($1)", ALBUM_COLUMNS);
        let albums: Vec<Album> = sqlx::query_as(&sql)
            .bind(keys)
            .fetch_all(&self.pool)
            .await
            .map_err(Arc::new)?;

        Ok(albums.into_iter().map(|a| (a.id, a)).collect())
    }
}
