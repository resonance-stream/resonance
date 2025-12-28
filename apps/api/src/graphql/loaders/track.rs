//! Track DataLoader for batched fetching
//!
//! This loader batches multiple track ID lookups into a single database query,
//! solving the N+1 problem when loading tracks for playlists.

use async_graphql::dataloader::Loader;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::Track;
use crate::repositories::utils::TRACK_COLUMNS;

/// DataLoader for batching track queries
#[derive(Clone)]
pub struct TrackLoader {
    pool: PgPool,
}

impl TrackLoader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Loader<Uuid> for TrackLoader {
    type Value = Track;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // Guard against empty keys to avoid unnecessary database query
        if keys.is_empty() {
            return Ok(HashMap::new());
        }

        let sql = format!("SELECT {} FROM tracks WHERE id = ANY($1)", TRACK_COLUMNS);
        let tracks: Vec<Track> = sqlx::query_as(&sql)
            .bind(keys)
            .fetch_all(&self.pool)
            .await
            .map_err(Arc::new)?;

        Ok(tracks.into_iter().map(|t| (t.id, t)).collect())
    }
}
