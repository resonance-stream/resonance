//! Tracks-by-Artist DataLoader for batched fetching
//!
//! This loader batches multiple artist ID lookups into a single database query,
//! returning top tracks for each artist ordered by play count. This solves the
//! N+1 problem when loading top tracks for multiple artists.

use async_graphql::dataloader::Loader;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::Track;
use crate::repositories::utils::TRACK_COLUMNS;

/// DataLoader for batching tracks-by-artist queries
#[derive(Clone)]
pub struct TracksByArtistLoader {
    pool: PgPool,
}

impl TracksByArtistLoader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Loader<Uuid> for TracksByArtistLoader {
    type Value = Vec<Track>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // Fetch top tracks by play count for all requested artists
        let sql = format!(
            "SELECT {} FROM tracks WHERE artist_id = ANY($1) ORDER BY artist_id, play_count DESC NULLS LAST, title ASC",
            TRACK_COLUMNS
        );
        let tracks: Vec<Track> = sqlx::query_as(&sql)
            .bind(keys)
            .fetch_all(&self.pool)
            .await
            .map_err(Arc::new)?;

        // Group tracks by artist_id
        let mut result: HashMap<Uuid, Vec<Track>> = HashMap::new();
        for track in tracks {
            result.entry(track.artist_id).or_default().push(track);
        }

        // Ensure all requested keys have an entry (even if empty)
        for key in keys {
            result.entry(*key).or_default();
        }

        Ok(result)
    }
}
