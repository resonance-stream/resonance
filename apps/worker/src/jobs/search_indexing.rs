//! Search indexing job for Meilisearch
//!
//! This job handles indexing of tracks, albums, and artists into Meilisearch
//! for full-text search capabilities.
//!
//! ## Job Types
//! - `IndexAll` - Full reindex of all entities
//! - `IndexTracks` - Index specific tracks by ID
//! - `IndexAlbums` - Index specific albums by ID
//! - `IndexArtists` - Index specific artists by ID
//! - `DeleteTrack` - Remove a track from the index
//! - `DeleteAlbum` - Remove an album from the index
//! - `DeleteArtist` - Remove an artist from the index

use chrono::{DateTime, NaiveDate, Utc};
use meilisearch_sdk::client::Client;
use meilisearch_sdk::settings::Settings;
use meilisearch_sdk::task_info::TaskInfo;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::error::{WorkerError, WorkerResult};
use crate::AppState;

// ==================== Index Names ====================

/// Index name constants
pub mod indexes {
    pub const TRACKS: &str = "tracks";
    pub const ALBUMS: &str = "albums";
    pub const ARTISTS: &str = "artists";
}

// ==================== Document Types ====================

/// Track document for Meilisearch indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackDocument {
    /// Unique identifier (primary key)
    pub id: String,
    /// Track UUID
    pub track_id: String,
    /// Track title
    pub title: String,
    /// Artist name for search
    pub artist_name: String,
    /// Artist ID for filtering
    pub artist_id: String,
    /// Album title (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_title: Option<String>,
    /// Album ID for filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_id: Option<String>,
    /// Genre tags
    pub genres: Vec<String>,
    /// AI-detected mood tags
    pub moods: Vec<String>,
    /// AI-generated tags
    pub tags: Vec<String>,
    /// Duration in milliseconds (for filtering)
    pub duration_ms: i32,
    /// Play count (for ranking)
    pub play_count: i32,
    /// Whether track is explicit
    pub explicit: bool,
    /// Unix timestamp for recency sorting
    pub created_at: i64,
    /// Unix timestamp of last update
    pub updated_at: i64,
}

/// Album document for Meilisearch indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumDocument {
    /// Unique identifier (primary key)
    pub id: String,
    /// Album UUID
    pub album_id: String,
    /// Album title
    pub title: String,
    /// Artist name for search
    pub artist_name: String,
    /// Artist ID for filtering
    pub artist_id: String,
    /// Genre tags
    pub genres: Vec<String>,
    /// Album type (album, single, EP, etc.)
    pub album_type: String,
    /// Release year (for filtering)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_year: Option<i32>,
    /// Total tracks count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tracks: Option<i32>,
    /// Unix timestamp for sorting
    pub created_at: i64,
}

/// Artist document for Meilisearch indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistDocument {
    /// Unique identifier (primary key)
    pub id: String,
    /// Artist UUID
    pub artist_id: String,
    /// Artist name
    pub name: String,
    /// Sort name for search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_name: Option<String>,
    /// Genre tags
    pub genres: Vec<String>,
    /// Biography text (for full-text search)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biography: Option<String>,
    /// Unix timestamp for sorting
    pub created_at: i64,
}

// ==================== Job Payload ====================

/// Search indexing job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SearchIndexingJob {
    /// Full reindex of all entities
    IndexAll,
    /// Initialize indexes with settings (run on startup)
    InitializeIndexes,
    /// Index specific tracks by ID
    IndexTracks { track_ids: Vec<Uuid> },
    /// Index specific albums by ID
    IndexAlbums { album_ids: Vec<Uuid> },
    /// Index specific artists by ID
    IndexArtists { artist_ids: Vec<Uuid> },
    /// Remove a track from the index
    DeleteTrack { track_id: Uuid },
    /// Remove an album from the index
    DeleteAlbum { album_id: Uuid },
    /// Remove an artist from the index
    DeleteArtist { artist_id: Uuid },
}

// ==================== Database Query Types ====================

/// Track row from database query
#[derive(Debug, sqlx::FromRow)]
struct TrackRow {
    id: Uuid,
    title: String,
    artist_id: Uuid,
    artist_name: String,
    album_id: Option<Uuid>,
    album_title: Option<String>,
    genres: Vec<String>,
    ai_mood: Vec<String>,
    ai_tags: Vec<String>,
    duration_ms: i32,
    play_count: i32,
    explicit: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Album row from database query
#[derive(Debug, sqlx::FromRow)]
struct AlbumRow {
    id: Uuid,
    title: String,
    artist_id: Uuid,
    artist_name: String,
    genres: Vec<String>,
    album_type: String,
    release_date: Option<NaiveDate>,
    total_tracks: Option<i32>,
    created_at: DateTime<Utc>,
}

/// Artist row from database query
#[derive(Debug, sqlx::FromRow)]
struct ArtistRow {
    id: Uuid,
    name: String,
    sort_name: Option<String>,
    genres: Vec<String>,
    biography: Option<String>,
    created_at: DateTime<Utc>,
}

// ==================== Job Execution ====================

/// Execute a search indexing job
#[instrument(skip(state, job), fields(action = ?std::mem::discriminant(job)))]
pub async fn execute(state: &Arc<AppState>, job: &SearchIndexingJob) -> WorkerResult<()> {
    let client = Client::new(
        state.config.meilisearch_url(),
        Some(state.config.meilisearch_key()),
    );

    match job {
        SearchIndexingJob::IndexAll => {
            info!("Starting full search index rebuild");
            initialize_indexes(&client).await?;
            index_all_tracks(&client, &state.db).await?;
            index_all_albums(&client, &state.db).await?;
            index_all_artists(&client, &state.db).await?;
            info!("Full search index rebuild complete");
        }
        SearchIndexingJob::InitializeIndexes => {
            info!("Initializing Meilisearch indexes");
            initialize_indexes(&client).await?;
        }
        SearchIndexingJob::IndexTracks { track_ids } => {
            debug!(count = track_ids.len(), "Indexing tracks");
            index_tracks_by_id(&client, &state.db, track_ids).await?;
        }
        SearchIndexingJob::IndexAlbums { album_ids } => {
            debug!(count = album_ids.len(), "Indexing albums");
            index_albums_by_id(&client, &state.db, album_ids).await?;
        }
        SearchIndexingJob::IndexArtists { artist_ids } => {
            debug!(count = artist_ids.len(), "Indexing artists");
            index_artists_by_id(&client, &state.db, artist_ids).await?;
        }
        SearchIndexingJob::DeleteTrack { track_id } => {
            debug!(track_id = %track_id, "Deleting track from index");
            delete_document(&client, indexes::TRACKS, track_id).await?;
        }
        SearchIndexingJob::DeleteAlbum { album_id } => {
            debug!(album_id = %album_id, "Deleting album from index");
            delete_document(&client, indexes::ALBUMS, album_id).await?;
        }
        SearchIndexingJob::DeleteArtist { artist_id } => {
            debug!(artist_id = %artist_id, "Deleting artist from index");
            delete_document(&client, indexes::ARTISTS, artist_id).await?;
        }
    }

    Ok(())
}

// ==================== Index Initialization ====================

/// Initialize all indexes with proper settings
async fn initialize_indexes(client: &Client) -> WorkerResult<()> {
    // Tracks index
    ensure_index_with_settings(
        client,
        indexes::TRACKS,
        Settings::new()
            .with_searchable_attributes([
                "title",
                "artist_name",
                "album_title",
                "genres",
                "moods",
                "tags",
            ])
            .with_filterable_attributes([
                "artist_id",
                "album_id",
                "genres",
                "moods",
                "explicit",
                "duration_ms",
            ])
            .with_sortable_attributes(["play_count", "created_at", "updated_at", "title"])
            .with_ranking_rules([
                "words",
                "typo",
                "proximity",
                "attribute",
                "sort",
                "exactness",
                "play_count:desc",
            ]),
    )
    .await?;

    // Albums index
    ensure_index_with_settings(
        client,
        indexes::ALBUMS,
        Settings::new()
            .with_searchable_attributes(["title", "artist_name", "genres"])
            .with_filterable_attributes(["artist_id", "genres", "album_type", "release_year"])
            .with_sortable_attributes(["created_at", "title", "release_year"]),
    )
    .await?;

    // Artists index
    ensure_index_with_settings(
        client,
        indexes::ARTISTS,
        Settings::new()
            .with_searchable_attributes(["name", "sort_name", "genres", "biography"])
            .with_filterable_attributes(["genres"])
            .with_sortable_attributes(["created_at", "name"]),
    )
    .await?;

    info!("Meilisearch indexes initialized");
    Ok(())
}

/// Ensure an index exists with the given settings
async fn ensure_index_with_settings(
    client: &Client,
    index_name: &str,
    settings: Settings,
) -> WorkerResult<()> {
    // Create index if it doesn't exist
    let task = client
        .create_index(index_name, Some("id"))
        .await
        .map_err(|e| {
            WorkerError::service_error(
                "meilisearch",
                format!("Failed to create index {}: {}", index_name, e),
            )
        })?;

    wait_for_task(client, task).await?;

    // Apply settings
    let index = client.index(index_name);
    let task = index.set_settings(&settings).await.map_err(|e| {
        WorkerError::service_error(
            "meilisearch",
            format!("Failed to set settings for {}: {}", index_name, e),
        )
    })?;

    wait_for_task(client, task).await?;

    debug!(index = index_name, "Index configured");
    Ok(())
}

// ==================== Track Indexing ====================

/// Index all tracks from the database
async fn index_all_tracks(client: &Client, db: &PgPool) -> WorkerResult<()> {
    let rows: Vec<TrackRow> = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.title,
            t.artist_id,
            a.name as artist_name,
            t.album_id,
            al.title as album_title,
            t.genres,
            t.ai_mood,
            t.ai_tags,
            t.duration_ms,
            t.play_count,
            t.explicit,
            t.created_at,
            t.updated_at
        FROM tracks t
        JOIN artists a ON t.artist_id = a.id
        LEFT JOIN albums al ON t.album_id = al.id
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(|e| WorkerError::Database(e))?;

    let documents: Vec<TrackDocument> = rows.into_iter().map(track_row_to_document).collect();

    if documents.is_empty() {
        info!("No tracks to index");
        return Ok(());
    }

    index_documents(client, indexes::TRACKS, &documents).await?;
    info!(count = documents.len(), "Indexed all tracks");
    Ok(())
}

/// Index specific tracks by ID
async fn index_tracks_by_id(client: &Client, db: &PgPool, track_ids: &[Uuid]) -> WorkerResult<()> {
    if track_ids.is_empty() {
        return Ok(());
    }

    let rows: Vec<TrackRow> = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.title,
            t.artist_id,
            a.name as artist_name,
            t.album_id,
            al.title as album_title,
            t.genres,
            t.ai_mood,
            t.ai_tags,
            t.duration_ms,
            t.play_count,
            t.explicit,
            t.created_at,
            t.updated_at
        FROM tracks t
        JOIN artists a ON t.artist_id = a.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.id = ANY($1)
        "#,
    )
    .bind(track_ids)
    .fetch_all(db)
    .await
    .map_err(|e| WorkerError::Database(e))?;

    let documents: Vec<TrackDocument> = rows.into_iter().map(track_row_to_document).collect();

    if documents.is_empty() {
        warn!("No tracks found for given IDs");
        return Ok(());
    }

    index_documents(client, indexes::TRACKS, &documents).await?;
    debug!(count = documents.len(), "Indexed tracks");
    Ok(())
}

/// Convert a track row to a document
fn track_row_to_document(row: TrackRow) -> TrackDocument {
    TrackDocument {
        id: row.id.to_string(),
        track_id: row.id.to_string(),
        title: row.title,
        artist_name: row.artist_name,
        artist_id: row.artist_id.to_string(),
        album_title: row.album_title,
        album_id: row.album_id.map(|id| id.to_string()),
        genres: row.genres,
        moods: row.ai_mood,
        tags: row.ai_tags,
        duration_ms: row.duration_ms,
        play_count: row.play_count,
        explicit: row.explicit,
        created_at: row.created_at.timestamp(),
        updated_at: row.updated_at.timestamp(),
    }
}

// ==================== Album Indexing ====================

/// Index all albums from the database
async fn index_all_albums(client: &Client, db: &PgPool) -> WorkerResult<()> {
    let rows: Vec<AlbumRow> = sqlx::query_as(
        r#"
        SELECT
            al.id,
            al.title,
            al.artist_id,
            a.name as artist_name,
            al.genres,
            al.album_type::text as album_type,
            al.release_date,
            al.total_tracks,
            al.created_at
        FROM albums al
        JOIN artists a ON al.artist_id = a.id
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(|e| WorkerError::Database(e))?;

    let documents: Vec<AlbumDocument> = rows.into_iter().map(album_row_to_document).collect();

    if documents.is_empty() {
        info!("No albums to index");
        return Ok(());
    }

    index_documents(client, indexes::ALBUMS, &documents).await?;
    info!(count = documents.len(), "Indexed all albums");
    Ok(())
}

/// Index specific albums by ID
async fn index_albums_by_id(client: &Client, db: &PgPool, album_ids: &[Uuid]) -> WorkerResult<()> {
    if album_ids.is_empty() {
        return Ok(());
    }

    let rows: Vec<AlbumRow> = sqlx::query_as(
        r#"
        SELECT
            al.id,
            al.title,
            al.artist_id,
            a.name as artist_name,
            al.genres,
            al.album_type::text as album_type,
            al.release_date,
            al.total_tracks,
            al.created_at
        FROM albums al
        JOIN artists a ON al.artist_id = a.id
        WHERE al.id = ANY($1)
        "#,
    )
    .bind(album_ids)
    .fetch_all(db)
    .await
    .map_err(|e| WorkerError::Database(e))?;

    let documents: Vec<AlbumDocument> = rows.into_iter().map(album_row_to_document).collect();

    if documents.is_empty() {
        warn!("No albums found for given IDs");
        return Ok(());
    }

    index_documents(client, indexes::ALBUMS, &documents).await?;
    debug!(count = documents.len(), "Indexed albums");
    Ok(())
}

/// Convert an album row to a document
fn album_row_to_document(row: AlbumRow) -> AlbumDocument {
    use chrono::Datelike;

    AlbumDocument {
        id: row.id.to_string(),
        album_id: row.id.to_string(),
        title: row.title,
        artist_name: row.artist_name,
        artist_id: row.artist_id.to_string(),
        genres: row.genres,
        album_type: row.album_type,
        release_year: row.release_date.map(|d| d.year()),
        total_tracks: row.total_tracks,
        created_at: row.created_at.timestamp(),
    }
}

// ==================== Artist Indexing ====================

/// Index all artists from the database
async fn index_all_artists(client: &Client, db: &PgPool) -> WorkerResult<()> {
    let rows: Vec<ArtistRow> = sqlx::query_as(
        r#"
        SELECT
            id,
            name,
            sort_name,
            genres,
            biography,
            created_at
        FROM artists
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(|e| WorkerError::Database(e))?;

    let documents: Vec<ArtistDocument> = rows.into_iter().map(artist_row_to_document).collect();

    if documents.is_empty() {
        info!("No artists to index");
        return Ok(());
    }

    index_documents(client, indexes::ARTISTS, &documents).await?;
    info!(count = documents.len(), "Indexed all artists");
    Ok(())
}

/// Index specific artists by ID
async fn index_artists_by_id(
    client: &Client,
    db: &PgPool,
    artist_ids: &[Uuid],
) -> WorkerResult<()> {
    if artist_ids.is_empty() {
        return Ok(());
    }

    let rows: Vec<ArtistRow> = sqlx::query_as(
        r#"
        SELECT
            id,
            name,
            sort_name,
            genres,
            biography,
            created_at
        FROM artists
        WHERE id = ANY($1)
        "#,
    )
    .bind(artist_ids)
    .fetch_all(db)
    .await
    .map_err(|e| WorkerError::Database(e))?;

    let documents: Vec<ArtistDocument> = rows.into_iter().map(artist_row_to_document).collect();

    if documents.is_empty() {
        warn!("No artists found for given IDs");
        return Ok(());
    }

    index_documents(client, indexes::ARTISTS, &documents).await?;
    debug!(count = documents.len(), "Indexed artists");
    Ok(())
}

/// Convert an artist row to a document
fn artist_row_to_document(row: ArtistRow) -> ArtistDocument {
    ArtistDocument {
        id: row.id.to_string(),
        artist_id: row.id.to_string(),
        name: row.name,
        sort_name: row.sort_name,
        genres: row.genres,
        biography: row.biography,
        created_at: row.created_at.timestamp(),
    }
}

// ==================== Helper Functions ====================

/// Index documents into a Meilisearch index
async fn index_documents<T: Serialize>(
    client: &Client,
    index_name: &str,
    documents: &[T],
) -> WorkerResult<()> {
    let index = client.index(index_name);
    let task = index
        .add_documents(documents, Some("id"))
        .await
        .map_err(|e| {
            WorkerError::service_error("meilisearch", format!("Failed to index documents: {}", e))
        })?;

    wait_for_task(client, task).await?;
    Ok(())
}

/// Delete a document from a Meilisearch index
async fn delete_document(client: &Client, index_name: &str, id: &Uuid) -> WorkerResult<()> {
    let index = client.index(index_name);
    let task = index.delete_document(id.to_string()).await.map_err(|e| {
        WorkerError::service_error("meilisearch", format!("Failed to delete document: {}", e))
    })?;

    wait_for_task(client, task).await?;
    Ok(())
}

/// Wait for a Meilisearch task to complete
async fn wait_for_task(client: &Client, task: TaskInfo) -> WorkerResult<()> {
    task.wait_for_completion(client, None, Some(Duration::from_secs(60)))
        .await
        .map_err(|e| WorkerError::service_error("meilisearch", format!("Task failed: {}", e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_document_serialization() {
        let doc = TrackDocument {
            id: "test-id".to_string(),
            track_id: "test-id".to_string(),
            title: "Test Track".to_string(),
            artist_name: "Test Artist".to_string(),
            artist_id: "artist-id".to_string(),
            album_title: Some("Test Album".to_string()),
            album_id: Some("album-id".to_string()),
            genres: vec!["Rock".to_string()],
            moods: vec!["happy".to_string()],
            tags: vec!["energetic".to_string()],
            duration_ms: 180000,
            play_count: 42,
            explicit: false,
            created_at: 1704067200,
            updated_at: 1704067200,
        };

        let json = serde_json::to_string(&doc).expect("serialization should work");
        assert!(json.contains("Test Track"));
        assert!(json.contains("Test Artist"));
    }

    #[test]
    fn test_search_indexing_job_serialization() {
        let job = SearchIndexingJob::IndexAll;
        let json = serde_json::to_string(&job).expect("serialization should work");
        assert!(json.contains("IndexAll"));

        let job = SearchIndexingJob::IndexTracks {
            track_ids: vec![Uuid::new_v4()],
        };
        let json = serde_json::to_string(&job).expect("serialization should work");
        assert!(json.contains("IndexTracks"));
    }
}
