//! Artist DataLoader for batched fetching
//!
//! This loader batches multiple artist ID lookups into a single database query,
//! solving the N+1 problem when loading artists for multiple albums or tracks.

use async_graphql::dataloader::Loader;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::Artist;
use crate::repositories::utils::ARTIST_COLUMNS;

/// DataLoader for batching artist queries
#[derive(Clone)]
pub struct ArtistLoader {
    pool: PgPool,
}

impl ArtistLoader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Loader<Uuid> for ArtistLoader {
    type Value = Artist;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let sql = format!("SELECT {} FROM artists WHERE id = ANY($1)", ARTIST_COLUMNS);
        let artists: Vec<Artist> = sqlx::query_as(&sql)
            .bind(keys)
            .fetch_all(&self.pool)
            .await
            .map_err(Arc::new)?;

        Ok(artists.into_iter().map(|a| (a.id, a)).collect())
    }
}
