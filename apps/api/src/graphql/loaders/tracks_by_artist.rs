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

/// Maximum tracks to return per artist (prevents loading unbounded data)
const MAX_TRACKS_PER_ARTIST: i32 = 50;

impl Loader<Uuid> for TracksByArtistLoader {
    type Value = Vec<Track>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // Guard against empty keys
        if keys.is_empty() {
            return Ok(HashMap::new());
        }

        // Fetch top tracks by play count for all requested artists
        // Uses ROW_NUMBER() window function to limit tracks per artist at the database level
        let sql = format!(
            r#"
            SELECT {cols} FROM (
                SELECT
                    {cols},
                    ROW_NUMBER() OVER (
                        PARTITION BY artist_id
                        ORDER BY play_count DESC NULLS LAST, title ASC
                    ) AS rn
                FROM tracks
                WHERE artist_id = ANY($1)
            ) t
            WHERE t.rn <= $2
            ORDER BY artist_id, play_count DESC NULLS LAST, title ASC
            "#,
            cols = TRACK_COLUMNS
        );
        let tracks: Vec<Track> = sqlx::query_as(&sql)
            .bind(keys)
            .bind(MAX_TRACKS_PER_ARTIST)
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
