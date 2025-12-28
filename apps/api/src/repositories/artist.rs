//! Artist repository for centralized database operations
//!
//! This module provides all artist-related database operations in a single location,
//! following the repository pattern.

use sqlx::PgPool;
use uuid::Uuid;

use super::utils::{escape_ilike, ARTIST_COLUMNS};
use crate::models::Artist;

/// Repository for artist database operations
#[derive(Clone)]
pub struct ArtistRepository {
    pool: PgPool,
}

impl ArtistRepository {
    /// Create a new ArtistRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find an artist by their unique ID
    pub async fn find_by_id(&self, artist_id: Uuid) -> Result<Option<Artist>, sqlx::Error> {
        let sql = format!("SELECT {} FROM artists WHERE id = $1", ARTIST_COLUMNS);
        sqlx::query_as::<_, Artist>(&sql)
            .bind(artist_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Find all artists with pagination
    pub async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Artist>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM artists ORDER BY sort_name ASC NULLS LAST, name ASC LIMIT $1 OFFSET $2",
            ARTIST_COLUMNS
        );
        sqlx::query_as::<_, Artist>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Search artists by name
    ///
    /// Escapes ILIKE special characters to prevent pattern injection.
    pub async fn search(&self, query: &str, limit: i64) -> Result<Vec<Artist>, sqlx::Error> {
        let escaped = escape_ilike(query);
        let sql = format!(
            r#"SELECT {} FROM artists
            WHERE name ILIKE $1 OR sort_name ILIKE $1
            ORDER BY
                CASE WHEN name ILIKE $2 THEN 0 ELSE 1 END,
                name ASC
            LIMIT $3"#,
            ARTIST_COLUMNS
        );
        sqlx::query_as::<_, Artist>(&sql)
            .bind(format!("%{}%", escaped))
            .bind(format!("{}%", escaped)) // Prioritize prefix matches
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// Find artists by genre
    pub async fn find_by_genre(
        &self,
        genre: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Artist>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM artists WHERE $1 = ANY(genres) ORDER BY name ASC LIMIT $2 OFFSET $3",
            ARTIST_COLUMNS
        );
        sqlx::query_as::<_, Artist>(&sql)
            .bind(genre)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Get total count of artists
    #[allow(dead_code)]
    pub async fn count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM artists")
            .fetch_one(&self.pool)
            .await
    }

    /// Find artist by MusicBrainz ID
    #[allow(dead_code)]
    pub async fn find_by_mbid(&self, mbid: Uuid) -> Result<Option<Artist>, sqlx::Error> {
        let sql = format!("SELECT {} FROM artists WHERE mbid = $1", ARTIST_COLUMNS);
        sqlx::query_as::<_, Artist>(&sql)
            .bind(mbid)
            .fetch_optional(&self.pool)
            .await
    }

    /// Find artist by Lidarr ID
    #[allow(dead_code)]
    pub async fn find_by_lidarr_id(&self, lidarr_id: i32) -> Result<Option<Artist>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM artists WHERE lidarr_id = $1",
            ARTIST_COLUMNS
        );
        sqlx::query_as::<_, Artist>(&sql)
            .bind(lidarr_id)
            .fetch_optional(&self.pool)
            .await
    }
}
