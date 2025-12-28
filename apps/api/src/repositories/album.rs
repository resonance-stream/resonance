//! Album repository for centralized database operations
//!
//! This module provides all album-related database operations in a single location,
//! following the repository pattern.

use sqlx::PgPool;
use uuid::Uuid;

use super::utils::{escape_ilike, ALBUM_COLUMNS};
use crate::models::Album;

/// Repository for album database operations
#[derive(Clone)]
pub struct AlbumRepository {
    pool: PgPool,
}

impl AlbumRepository {
    /// Create a new AlbumRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find an album by its unique ID
    pub async fn find_by_id(&self, album_id: Uuid) -> Result<Option<Album>, sqlx::Error> {
        let sql = format!("SELECT {} FROM albums WHERE id = $1", ALBUM_COLUMNS);
        sqlx::query_as::<_, Album>(&sql)
            .bind(album_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Find all albums with pagination
    pub async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Album>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM albums ORDER BY release_date DESC NULLS LAST, title ASC LIMIT $1 OFFSET $2",
            ALBUM_COLUMNS
        );
        sqlx::query_as::<_, Album>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Find albums by artist ID
    pub async fn find_by_artist(
        &self,
        artist_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Album>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM albums WHERE artist_id = $1 ORDER BY release_date DESC NULLS LAST, title ASC LIMIT $2 OFFSET $3",
            ALBUM_COLUMNS
        );
        sqlx::query_as::<_, Album>(&sql)
            .bind(artist_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Search albums by title
    ///
    /// Escapes ILIKE special characters to prevent pattern injection.
    pub async fn search(&self, query: &str, limit: i64) -> Result<Vec<Album>, sqlx::Error> {
        let escaped = escape_ilike(query);
        let sql = format!(
            r#"SELECT {} FROM albums
            WHERE title ILIKE $1
            ORDER BY
                CASE WHEN title ILIKE $2 THEN 0 ELSE 1 END,
                release_date DESC NULLS LAST
            LIMIT $3"#,
            ALBUM_COLUMNS
        );
        sqlx::query_as::<_, Album>(&sql)
            .bind(format!("%{}%", escaped))
            .bind(format!("{}%", escaped))
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// Find recently added albums
    pub async fn find_recent(&self, limit: i64) -> Result<Vec<Album>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM albums ORDER BY created_at DESC LIMIT $1",
            ALBUM_COLUMNS
        );
        sqlx::query_as::<_, Album>(&sql)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// Get total count of albums
    #[allow(dead_code)]
    pub async fn count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM albums")
            .fetch_one(&self.pool)
            .await
    }

    /// Get album count for an artist
    #[allow(dead_code)]
    pub async fn count_by_artist(&self, artist_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM albums WHERE artist_id = $1")
            .bind(artist_id)
            .fetch_one(&self.pool)
            .await
    }
}
