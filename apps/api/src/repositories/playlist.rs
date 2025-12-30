//! Playlist repository for centralized database operations
//!
//! This module provides all playlist-related database operations in a single location,
//! following the repository pattern.

use sqlx::PgPool;
use uuid::Uuid;

use super::utils::{escape_ilike, PLAYLIST_COLUMNS};
use crate::models::playlist::{PlaylistType, SmartPlaylistRules};
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

    // =========================================================================
    // CRUD Operations
    // =========================================================================

    /// Create a new playlist
    ///
    /// Creates a new playlist with the given parameters. For smart playlists,
    /// the smart_rules parameter should be provided.
    pub async fn create(
        &self,
        user_id: Uuid,
        name: &str,
        description: Option<&str>,
        is_public: bool,
        playlist_type: PlaylistType,
        smart_rules: Option<SmartPlaylistRules>,
    ) -> Result<Playlist, sqlx::Error> {
        let id = Uuid::new_v4();
        // Use sqlx::types::Json for proper JSON binding without manual serialization
        let smart_rules_json = smart_rules.as_ref().map(sqlx::types::Json);

        let sql = format!(
            r#"
            INSERT INTO playlists (
                id, user_id, name, description, is_public,
                playlist_type, smart_rules, track_count, total_duration_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 0, 0)
            RETURNING {}
            "#,
            PLAYLIST_COLUMNS
        );

        sqlx::query_as::<_, Playlist>(&sql)
            .bind(id)
            .bind(user_id)
            .bind(name)
            .bind(description)
            .bind(is_public)
            .bind(playlist_type)
            .bind(smart_rules_json)
            .fetch_one(&self.pool)
            .await
    }

    /// Update an existing playlist
    ///
    /// Updates only the fields that are provided (not None).
    pub async fn update(
        &self,
        playlist_id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        image_url: Option<&str>,
        is_public: Option<bool>,
        smart_rules: Option<SmartPlaylistRules>,
    ) -> Result<Playlist, sqlx::Error> {
        // Build dynamic UPDATE query based on which fields are provided
        let mut set_clauses: Vec<String> = Vec::new();
        let mut param_index = 1;

        if name.is_some() {
            param_index += 1;
            set_clauses.push(format!("name = ${}", param_index));
        }
        if description.is_some() {
            param_index += 1;
            set_clauses.push(format!("description = ${}", param_index));
        }
        if image_url.is_some() {
            param_index += 1;
            set_clauses.push(format!("image_url = ${}", param_index));
        }
        if is_public.is_some() {
            param_index += 1;
            set_clauses.push(format!("is_public = ${}", param_index));
        }
        if smart_rules.is_some() {
            param_index += 1;
            set_clauses.push(format!("smart_rules = ${}", param_index));
        }

        // Always update updated_at
        set_clauses.push("updated_at = NOW()".to_string());

        let sql = format!(
            r#"
            UPDATE playlists
            SET {}
            WHERE id = $1
            RETURNING {}
            "#,
            set_clauses.join(", "),
            PLAYLIST_COLUMNS
        );

        // Use sqlx::types::Json for proper JSON binding without manual serialization
        let smart_rules_json = smart_rules.as_ref().map(sqlx::types::Json);

        // Build the query dynamically
        let mut query = sqlx::query_as::<_, Playlist>(&sql).bind(playlist_id);

        if let Some(n) = name {
            query = query.bind(n);
        }
        if let Some(d) = description {
            query = query.bind(d);
        }
        if let Some(i) = image_url {
            query = query.bind(i);
        }
        if let Some(p) = is_public {
            query = query.bind(p);
        }
        if let Some(json) = smart_rules_json {
            query = query.bind(json);
        }

        query.fetch_one(&self.pool).await
    }

    /// Delete a playlist and all its tracks
    ///
    /// This will cascade delete all playlist_tracks entries.
    /// Returns the number of tracks that were deleted along with the playlist.
    ///
    /// # Errors
    /// Returns `sqlx::Error::RowNotFound` if the playlist doesn't exist.
    pub async fn delete(&self, playlist_id: Uuid) -> Result<u64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Delete playlist tracks first (in case cascade isn't set up)
        let tracks_deleted = sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = $1")
            .bind(playlist_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        // Delete the playlist and verify it existed
        let playlist_result = sqlx::query("DELETE FROM playlists WHERE id = $1")
            .bind(playlist_id)
            .execute(&mut *tx)
            .await?;

        if playlist_result.rows_affected() == 0 {
            // Rollback happens automatically when tx is dropped
            return Err(sqlx::Error::RowNotFound);
        }

        tx.commit().await?;
        Ok(tracks_deleted)
    }

    /// Add tracks to a playlist
    ///
    /// Adds tracks at the specified position (or at the end if position is None).
    /// Updates the playlist's track_count and total_duration_ms.
    pub async fn add_tracks(
        &self,
        playlist_id: Uuid,
        track_ids: &[Uuid],
        added_by: Uuid,
        position: Option<i32>,
    ) -> Result<(), sqlx::Error> {
        if track_ids.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // Get the current max position
        let max_position: Option<i32> = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), -1) FROM playlist_tracks WHERE playlist_id = $1",
        )
        .bind(playlist_id)
        .fetch_one(&mut *tx)
        .await?;

        // Clamp start_position to min 0 for defense-in-depth (mutation layer also validates)
        let start_position = position.unwrap_or(max_position.unwrap_or(-1) + 1).max(0);

        // Filter out tracks that already exist to prevent gaps when using position shifts
        // This is important because ON CONFLICT DO NOTHING would skip existing tracks,
        // but we'd have already shifted positions by the full track_ids count
        let new_track_ids: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT x.track_id
            FROM UNNEST($1::uuid[]) AS x(track_id)
            WHERE NOT EXISTS (
                SELECT 1
                FROM playlist_tracks pt
                WHERE pt.playlist_id = $2 AND pt.track_id = x.track_id
            )
            "#,
        )
        .bind(track_ids)
        .bind(playlist_id)
        .fetch_all(&mut *tx)
        .await?;

        // If inserting at a specific position, shift existing tracks by new track count only
        if position.is_some() && !new_track_ids.is_empty() {
            sqlx::query(
                r#"
                UPDATE playlist_tracks
                SET position = position + $1
                WHERE playlist_id = $2 AND position >= $3
                "#,
            )
            .bind(new_track_ids.len() as i32)
            .bind(playlist_id)
            .bind(start_position)
            .execute(&mut *tx)
            .await?;
        }

        // Insert new tracks (no conflicts expected due to pre-filter)
        for (i, track_id) in new_track_ids.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO playlist_tracks (id, playlist_id, track_id, added_by, position)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(playlist_id)
            .bind(track_id)
            .bind(added_by)
            .bind(start_position + i as i32)
            .execute(&mut *tx)
            .await?;
        }

        // Update playlist stats
        self.update_playlist_stats(&mut tx, playlist_id).await?;

        tx.commit().await?;
        Ok(())
    }

    /// Remove tracks from a playlist
    ///
    /// Removes the specified tracks and reorders remaining tracks.
    /// Updates the playlist's track_count and total_duration_ms.
    pub async fn remove_tracks(
        &self,
        playlist_id: Uuid,
        track_ids: &[Uuid],
    ) -> Result<(), sqlx::Error> {
        if track_ids.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // Delete the tracks
        sqlx::query(
            r#"
            DELETE FROM playlist_tracks
            WHERE playlist_id = $1 AND track_id = ANY($2)
            "#,
        )
        .bind(playlist_id)
        .bind(track_ids)
        .execute(&mut *tx)
        .await?;

        // Reorder remaining tracks to close gaps
        sqlx::query(
            r#"
            WITH ordered AS (
                SELECT id, ROW_NUMBER() OVER (ORDER BY position) - 1 as new_position
                FROM playlist_tracks
                WHERE playlist_id = $1
            )
            UPDATE playlist_tracks pt
            SET position = o.new_position
            FROM ordered o
            WHERE pt.id = o.id
            "#,
        )
        .bind(playlist_id)
        .execute(&mut *tx)
        .await?;

        // Update playlist stats
        self.update_playlist_stats(&mut tx, playlist_id).await?;

        tx.commit().await?;
        Ok(())
    }

    /// Set tracks for a smart playlist (replaces all existing tracks)
    ///
    /// Used when refreshing a smart playlist with newly evaluated rules.
    /// The `added_by` parameter records who triggered the refresh (usually the playlist owner
    /// or a system user for automated refreshes).
    #[allow(dead_code)] // Will be used by PlaylistService in Step 3
    pub async fn set_tracks(
        &self,
        playlist_id: Uuid,
        track_ids: &[Uuid],
        added_by: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Clear existing tracks
        sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = $1")
            .bind(playlist_id)
            .execute(&mut *tx)
            .await?;

        // Insert new tracks
        for (i, track_id) in track_ids.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO playlist_tracks (id, playlist_id, track_id, added_by, position)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(playlist_id)
            .bind(track_id)
            .bind(added_by)
            .bind(i as i32)
            .execute(&mut *tx)
            .await?;
        }

        // Update playlist stats
        self.update_playlist_stats(&mut tx, playlist_id).await?;

        tx.commit().await?;
        Ok(())
    }

    /// Update playlist statistics (track_count, total_duration_ms)
    async fn update_playlist_stats(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        playlist_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE playlists p
            SET
                track_count = (
                    SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = p.id
                ),
                total_duration_ms = (
                    SELECT COALESCE(SUM(t.duration_ms), 0)
                    FROM playlist_tracks pt
                    JOIN tracks t ON pt.track_id = t.id
                    WHERE pt.playlist_id = p.id
                ),
                updated_at = NOW()
            WHERE p.id = $1
            "#,
        )
        .bind(playlist_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
