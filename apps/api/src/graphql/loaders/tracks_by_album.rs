//! Tracks-by-Album DataLoader for batched fetching
//!
//! This loader batches multiple album ID lookups into a single database query,
//! returning all tracks for each album. This solves the N+1 problem when
//! loading tracks for multiple albums.

use async_graphql::dataloader::Loader;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::Track;
use crate::repositories::utils::TRACK_COLUMNS;

/// DataLoader for batching tracks-by-album queries
#[derive(Clone)]
pub struct TracksByAlbumLoader {
    pool: PgPool,
}

impl TracksByAlbumLoader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Maximum tracks to return per album (prevents loading unbounded data)
const MAX_TRACKS_PER_ALBUM: i32 = 200;

impl Loader<Uuid> for TracksByAlbumLoader {
    type Value = Vec<Track>;
    type Error = Arc<sqlx::Error>;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        // Guard against empty keys
        if keys.is_empty() {
            return Ok(HashMap::new());
        }

        // Uses ROW_NUMBER() window function to limit tracks per album at the database level
        let sql = format!(
            r#"
            SELECT {cols} FROM (
                SELECT
                    {cols},
                    ROW_NUMBER() OVER (
                        PARTITION BY album_id
                        ORDER BY disc_number ASC NULLS LAST, track_number ASC NULLS LAST
                    ) AS rn
                FROM tracks
                WHERE album_id = ANY($1)
            ) t
            WHERE t.rn <= $2
            ORDER BY album_id, disc_number ASC NULLS LAST, track_number ASC NULLS LAST
            "#,
            cols = TRACK_COLUMNS
        );
        let tracks: Vec<Track> = sqlx::query_as(&sql)
            .bind(keys)
            .bind(MAX_TRACKS_PER_ALBUM)
            .fetch_all(&self.pool)
            .await
            .map_err(Arc::new)?;

        // Group tracks by album_id
        let mut result: HashMap<Uuid, Vec<Track>> = HashMap::new();
        for track in tracks {
            if let Some(album_id) = track.album_id {
                result.entry(album_id).or_default().push(track);
            }
        }

        // Ensure all requested keys have an entry (even if empty)
        for key in keys {
            result.entry(*key).or_default();
        }

        Ok(result)
    }
}
