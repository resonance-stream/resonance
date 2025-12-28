//! Track repository for centralized database operations
//!
//! This module provides all track-related database operations in a single location,
//! following the repository pattern.

use sqlx::PgPool;
use uuid::Uuid;

use super::utils::{escape_ilike, TRACK_COLUMNS};
use crate::models::Track;

/// Repository for track database operations
#[derive(Clone)]
pub struct TrackRepository {
    pool: PgPool,
}

impl TrackRepository {
    /// Create a new TrackRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a track by its unique ID
    pub async fn find_by_id(&self, track_id: Uuid) -> Result<Option<Track>, sqlx::Error> {
        let sql = format!("SELECT {} FROM tracks WHERE id = $1", TRACK_COLUMNS);
        sqlx::query_as::<_, Track>(&sql)
            .bind(track_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Find all tracks with pagination
    pub async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Track>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tracks ORDER BY title ASC LIMIT $1 OFFSET $2",
            TRACK_COLUMNS
        );
        sqlx::query_as::<_, Track>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Find tracks by album ID
    #[allow(dead_code)]
    pub async fn find_by_album(&self, album_id: Uuid) -> Result<Vec<Track>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tracks WHERE album_id = $1 ORDER BY disc_number ASC, track_number ASC",
            TRACK_COLUMNS
        );
        sqlx::query_as::<_, Track>(&sql)
            .bind(album_id)
            .fetch_all(&self.pool)
            .await
    }

    /// Find tracks by album ID with pagination
    pub async fn find_by_album_paginated(
        &self,
        album_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Track>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tracks WHERE album_id = $1 ORDER BY disc_number ASC, track_number ASC LIMIT $2 OFFSET $3",
            TRACK_COLUMNS
        );
        sqlx::query_as::<_, Track>(&sql)
            .bind(album_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Find tracks by artist ID
    pub async fn find_by_artist(
        &self,
        artist_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Track>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tracks WHERE artist_id = $1 ORDER BY play_count DESC, title ASC LIMIT $2 OFFSET $3",
            TRACK_COLUMNS
        );
        sqlx::query_as::<_, Track>(&sql)
            .bind(artist_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Search tracks by title
    ///
    /// Escapes ILIKE special characters to prevent pattern injection.
    pub async fn search(&self, query: &str, limit: i64) -> Result<Vec<Track>, sqlx::Error> {
        let escaped = escape_ilike(query);
        let sql = format!(
            r#"SELECT {} FROM tracks
            WHERE title ILIKE $1
            ORDER BY
                CASE WHEN title ILIKE $2 THEN 0 ELSE 1 END,
                play_count DESC
            LIMIT $3"#,
            TRACK_COLUMNS
        );
        sqlx::query_as::<_, Track>(&sql)
            .bind(format!("%{}%", escaped))
            .bind(format!("{}%", escaped))
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// Find top tracks (most played)
    pub async fn find_top_tracks(&self, limit: i64) -> Result<Vec<Track>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tracks WHERE play_count > 0 ORDER BY play_count DESC LIMIT $1",
            TRACK_COLUMNS
        );
        sqlx::query_as::<_, Track>(&sql)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// Find recently played tracks
    #[allow(dead_code)]
    pub async fn find_recently_played(&self, limit: i64) -> Result<Vec<Track>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tracks WHERE last_played_at IS NOT NULL ORDER BY last_played_at DESC LIMIT $1",
            TRACK_COLUMNS
        );
        sqlx::query_as::<_, Track>(&sql)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// Find recently added tracks
    #[allow(dead_code)]
    pub async fn find_recent(&self, limit: i64) -> Result<Vec<Track>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM tracks ORDER BY created_at DESC LIMIT $1",
            TRACK_COLUMNS
        );
        sqlx::query_as::<_, Track>(&sql)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    /// Get total count of tracks
    #[allow(dead_code)]
    pub async fn count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
            .fetch_one(&self.pool)
            .await
    }

    /// Increment play count for a track
    #[allow(dead_code)]
    pub async fn increment_play_count(&self, track_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE tracks
            SET play_count = play_count + 1,
                last_played_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(track_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Increment skip count for a track
    #[allow(dead_code)]
    pub async fn increment_skip_count(&self, track_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE tracks
            SET skip_count = skip_count + 1,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(track_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
