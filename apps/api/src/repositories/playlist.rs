//! Playlist repository for centralized database operations
//!
//! This module provides all playlist-related database operations in a single location,
//! following the repository pattern.

use sqlx::PgPool;
use uuid::Uuid;

use super::utils::{escape_ilike, PLAYLIST_COLUMNS};
use crate::models::{Playlist, PlaylistTrack};

/// Repository for playlist database operations
#[derive(Clone)]
pub struct PlaylistRepository {
    pool: PgPool,
}

impl PlaylistRepository {
    /// Create a new PlaylistRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a playlist by its unique ID
    pub async fn find_by_id(&self, playlist_id: Uuid) -> Result<Option<Playlist>, sqlx::Error> {
        let sql = format!("SELECT {} FROM playlists WHERE id = $1", PLAYLIST_COLUMNS);
        sqlx::query_as::<_, Playlist>(&sql)
            .bind(playlist_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Find all playlists for a user
    pub async fn find_by_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Playlist>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM playlists WHERE user_id = $1 ORDER BY updated_at DESC LIMIT $2 OFFSET $3",
            PLAYLIST_COLUMNS
        );
        sqlx::query_as::<_, Playlist>(&sql)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Find public playlists with pagination
    pub async fn find_public(&self, limit: i64, offset: i64) -> Result<Vec<Playlist>, sqlx::Error> {
        let sql = format!(
            "SELECT {} FROM playlists WHERE is_public = true ORDER BY track_count DESC LIMIT $1 OFFSET $2",
            PLAYLIST_COLUMNS
        );
        sqlx::query_as::<_, Playlist>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Find collaborative playlists for a user (including those they're a collaborator on)
    #[allow(dead_code)]
    pub async fn find_collaborative_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Playlist>, sqlx::Error> {
        // Note: Using p. prefix for columns due to JOIN
        sqlx::query_as::<_, Playlist>(
            r#"
            SELECT DISTINCT
                p.id, p.user_id, p.name, p.description, p.image_url,
                p.is_public, p.is_collaborative, p.playlist_type,
                p.smart_rules, p.track_count, p.total_duration_ms,
                p.created_at, p.updated_at
            FROM playlists p
            LEFT JOIN playlist_collaborators pc ON p.id = pc.playlist_id
            WHERE p.is_collaborative = true
                AND (p.user_id = $1 OR pc.user_id = $1)
            ORDER BY p.updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get tracks in a playlist
    pub async fn get_tracks(
        &self,
        playlist_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PlaylistTrack>, sqlx::Error> {
        sqlx::query_as::<_, PlaylistTrack>(
            r#"
            SELECT
                id, playlist_id, track_id, added_by, position, added_at
            FROM playlist_tracks
            WHERE playlist_id = $1
            ORDER BY position ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(playlist_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Check if a user can access a playlist (owner, collaborator, or public)
    pub async fn can_access(&self, playlist_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM playlists p
                LEFT JOIN playlist_collaborators pc ON p.id = pc.playlist_id
                WHERE p.id = $1
                    AND (p.is_public = true OR p.user_id = $2 OR pc.user_id = $2)
            )
            "#,
        )
        .bind(playlist_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.unwrap_or(false))
    }

    /// Check if a user can edit a playlist (owner or collaborator with edit permission)
    #[allow(dead_code)]
    pub async fn can_edit(&self, playlist_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM playlists p
                LEFT JOIN playlist_collaborators pc ON p.id = pc.playlist_id AND pc.can_edit = true
                WHERE p.id = $1
                    AND (p.user_id = $2 OR pc.user_id = $2)
            )
            "#,
        )
        .bind(playlist_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.unwrap_or(false))
    }

    /// Get count of playlists for a user
    #[allow(dead_code)]
    pub async fn count_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM playlists WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
    }

    /// Search playlists by name (only public playlists)
    ///
    /// Escapes ILIKE special characters to prevent pattern injection.
    #[allow(dead_code)]
    pub async fn search(&self, query: &str, limit: i64) -> Result<Vec<Playlist>, sqlx::Error> {
        let escaped = escape_ilike(query);
        let sql = format!(
            r#"SELECT {} FROM playlists
            WHERE is_public = true AND name ILIKE $1
            ORDER BY
                CASE WHEN name ILIKE $2 THEN 0 ELSE 1 END,
                track_count DESC
            LIMIT $3"#,
            PLAYLIST_COLUMNS
        );
        sqlx::query_as::<_, Playlist>(&sql)
            .bind(format!("%{}%", escaped))
            .bind(format!("{}%", escaped))
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }
}
