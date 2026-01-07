//! Meilisearch full-text search service
//!
//! Provides full-text search capabilities for tracks, albums, and artists using Meilisearch.
//! This complements the semantic search (pgvector embeddings) with traditional keyword search.
//!
//! ## Index Structure
//! - `tracks` - Full-text search for track title, artist, album, genres, moods
//! - `albums` - Full-text search for album title, artist, genres
//! - `artists` - Full-text search for artist name, genres, biography

use meilisearch_sdk::client::Client;
use meilisearch_sdk::search::SearchResults;
use meilisearch_sdk::settings::Settings;
use meilisearch_sdk::task_info::TaskInfo;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// Index name constants
pub mod indexes {
    pub const TRACKS: &str = "tracks";
    pub const ALBUMS: &str = "albums";
    pub const ARTISTS: &str = "artists";
}

/// Maximum number of search results per page
const MAX_SEARCH_LIMIT: usize = 100;

/// Default number of search results
const DEFAULT_SEARCH_LIMIT: usize = 20;

/// Validate and clamp the limit parameter
fn validate_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT)
}

// ==================== Document Types ====================

/// Track document for Meilisearch indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TrackDocument {
    /// Unique identifier (primary key)
    pub id: String,
    /// Track UUID
    pub track_id: Uuid,
    /// Track title
    pub title: String,
    /// Artist name for search
    pub artist_name: String,
    /// Artist ID for filtering
    pub artist_id: Uuid,
    /// Album title (if available)
    pub album_title: Option<String>,
    /// Album ID for filtering
    pub album_id: Option<Uuid>,
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

impl TrackDocument {
    /// Create a new track document from database fields
    #[allow(dead_code)]
    pub fn new(
        track_id: Uuid,
        title: String,
        artist_id: Uuid,
        artist_name: String,
        album_id: Option<Uuid>,
        album_title: Option<String>,
        genres: Vec<String>,
        moods: Vec<String>,
        tags: Vec<String>,
        duration_ms: i32,
        play_count: i32,
        explicit: bool,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            id: track_id.to_string(),
            track_id,
            title,
            artist_name,
            artist_id,
            album_title,
            album_id,
            genres,
            moods,
            tags,
            duration_ms,
            play_count,
            explicit,
            created_at: created_at.timestamp(),
            updated_at: updated_at.timestamp(),
        }
    }
}

/// Album document for Meilisearch indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AlbumDocument {
    /// Unique identifier (primary key)
    pub id: String,
    /// Album UUID
    pub album_id: Uuid,
    /// Album title
    pub title: String,
    /// Artist name for search
    pub artist_name: String,
    /// Artist ID for filtering
    pub artist_id: Uuid,
    /// Genre tags
    pub genres: Vec<String>,
    /// Album type (album, single, EP, etc.)
    pub album_type: String,
    /// Release year (for filtering)
    pub release_year: Option<i32>,
    /// Total tracks count
    pub total_tracks: Option<i32>,
    /// Unix timestamp for sorting
    pub created_at: i64,
}

impl AlbumDocument {
    /// Create a new album document from database fields
    #[allow(dead_code)]
    pub fn new(
        album_id: Uuid,
        title: String,
        artist_id: Uuid,
        artist_name: String,
        genres: Vec<String>,
        album_type: String,
        release_date: Option<chrono::NaiveDate>,
        total_tracks: Option<i32>,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            id: album_id.to_string(),
            album_id,
            title,
            artist_name,
            artist_id,
            genres,
            album_type,
            release_year: release_date.map(|d| d.year()),
            total_tracks,
            created_at: created_at.timestamp(),
        }
    }
}

/// Artist document for Meilisearch indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ArtistDocument {
    /// Unique identifier (primary key)
    pub id: String,
    /// Artist UUID
    pub artist_id: Uuid,
    /// Artist name
    pub name: String,
    /// Sort name for search
    pub sort_name: Option<String>,
    /// Genre tags
    pub genres: Vec<String>,
    /// Biography text (for full-text search)
    pub biography: Option<String>,
    /// Unix timestamp for sorting
    pub created_at: i64,
}

impl ArtistDocument {
    /// Create a new artist document from database fields
    #[allow(dead_code)]
    pub fn new(
        artist_id: Uuid,
        name: String,
        sort_name: Option<String>,
        genres: Vec<String>,
        biography: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            id: artist_id.to_string(),
            artist_id,
            name,
            sort_name,
            genres,
            biography,
            created_at: created_at.timestamp(),
        }
    }
}

// ==================== Search Results ====================

/// Track search result with hit highlights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackSearchHit {
    /// Track UUID
    pub track_id: Uuid,
    /// Track title
    pub title: String,
    /// Artist name
    pub artist_name: String,
    /// Artist ID
    pub artist_id: Uuid,
    /// Album title
    pub album_title: Option<String>,
    /// Album ID
    pub album_id: Option<Uuid>,
    /// Genres
    pub genres: Vec<String>,
    /// Moods
    pub moods: Vec<String>,
    /// Duration in ms
    pub duration_ms: i32,
}

impl From<TrackDocument> for TrackSearchHit {
    fn from(doc: TrackDocument) -> Self {
        Self {
            track_id: doc.track_id,
            title: doc.title,
            artist_name: doc.artist_name,
            artist_id: doc.artist_id,
            album_title: doc.album_title,
            album_id: doc.album_id,
            genres: doc.genres,
            moods: doc.moods,
            duration_ms: doc.duration_ms,
        }
    }
}

/// Album search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumSearchHit {
    /// Album UUID
    pub album_id: Uuid,
    /// Album title
    pub title: String,
    /// Artist name
    pub artist_name: String,
    /// Artist ID
    pub artist_id: Uuid,
    /// Genres
    pub genres: Vec<String>,
    /// Album type
    pub album_type: String,
    /// Release year
    pub release_year: Option<i32>,
}

impl From<AlbumDocument> for AlbumSearchHit {
    fn from(doc: AlbumDocument) -> Self {
        Self {
            album_id: doc.album_id,
            title: doc.title,
            artist_name: doc.artist_name,
            artist_id: doc.artist_id,
            genres: doc.genres,
            album_type: doc.album_type,
            release_year: doc.release_year,
        }
    }
}

/// Artist search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistSearchHit {
    /// Artist UUID
    pub artist_id: Uuid,
    /// Artist name
    pub name: String,
    /// Genres
    pub genres: Vec<String>,
}

impl From<ArtistDocument> for ArtistSearchHit {
    fn from(doc: ArtistDocument) -> Self {
        Self {
            artist_id: doc.artist_id,
            name: doc.name,
            genres: doc.genres,
        }
    }
}

/// Combined search results across all entity types
#[derive(Debug, Clone, Default)]
pub struct UnifiedSearchResults {
    /// Matching tracks
    pub tracks: Vec<TrackSearchHit>,
    /// Matching albums
    pub albums: Vec<AlbumSearchHit>,
    /// Matching artists
    pub artists: Vec<ArtistSearchHit>,
    /// Total estimated hits (sum of all entity types)
    pub total_hits: usize,
    /// Query processing time in milliseconds
    pub processing_time_ms: u128,
}

// ==================== Meilisearch Service ====================

/// Meilisearch full-text search service
#[derive(Clone)]
#[allow(dead_code)]
pub struct MeilisearchService {
    client: Client,
}

#[allow(dead_code)]
impl MeilisearchService {
    /// Create a new Meilisearch service
    ///
    /// # Arguments
    /// * `url` - Meilisearch server URL (e.g., "http://localhost:7700")
    /// * `api_key` - Meilisearch API key (master key or search-only key)
    pub fn new(url: &str, api_key: &str) -> Self {
        let client = Client::new(url, Some(api_key));
        Self { client }
    }

    /// Initialize indexes with proper settings
    ///
    /// Creates indexes and configures searchable/filterable attributes.
    /// Should be called during application startup.
    #[instrument(skip(self))]
    pub async fn initialize_indexes(&self) -> ApiResult<()> {
        info!("Initializing Meilisearch indexes");

        // Create and configure tracks index
        self.ensure_index_with_settings(
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

        // Create and configure albums index
        self.ensure_index_with_settings(
            indexes::ALBUMS,
            Settings::new()
                .with_searchable_attributes(["title", "artist_name", "genres"])
                .with_filterable_attributes(["artist_id", "genres", "album_type", "release_year"])
                .with_sortable_attributes(["created_at", "title", "release_year"]),
        )
        .await?;

        // Create and configure artists index
        self.ensure_index_with_settings(
            indexes::ARTISTS,
            Settings::new()
                .with_searchable_attributes(["name", "sort_name", "genres", "biography"])
                .with_filterable_attributes(["genres"])
                .with_sortable_attributes(["created_at", "name"]),
        )
        .await?;

        info!("Meilisearch indexes initialized successfully");
        Ok(())
    }

    /// Ensure an index exists with the given settings
    async fn ensure_index_with_settings(
        &self,
        index_name: &str,
        settings: Settings,
    ) -> ApiResult<()> {
        // Create index if it doesn't exist
        let task = self
            .client
            .create_index(index_name, Some("id"))
            .await
            .map_err(|e| {
                ApiError::Search(format!("Failed to create index {}: {}", index_name, e))
            })?;

        // Wait for index creation
        self.wait_for_task(task).await?;

        // Apply settings
        let index = self.client.index(index_name);
        let task = index.set_settings(&settings).await.map_err(|e| {
            ApiError::Search(format!("Failed to set settings for {}: {}", index_name, e))
        })?;

        self.wait_for_task(task).await?;

        debug!(index = index_name, "Index configured");
        Ok(())
    }

    /// Wait for a Meilisearch task to complete
    async fn wait_for_task(&self, task: TaskInfo) -> ApiResult<()> {
        task.wait_for_completion(&self.client, None, Some(Duration::from_secs(30)))
            .await
            .map_err(|e| ApiError::Search(format!("Task failed: {}", e)))?;
        Ok(())
    }

    // ==================== Index Operations ====================

    /// Index multiple track documents
    #[instrument(skip(self, documents), fields(count = documents.len()))]
    pub async fn index_tracks(&self, documents: &[TrackDocument]) -> ApiResult<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let index = self.client.index(indexes::TRACKS);
        let task = index
            .add_documents(documents, Some("id"))
            .await
            .map_err(|e| ApiError::Search(format!("Failed to index tracks: {}", e)))?;

        self.wait_for_task(task).await?;
        debug!(count = documents.len(), "Indexed tracks");
        Ok(())
    }

    /// Index multiple album documents
    #[instrument(skip(self, documents), fields(count = documents.len()))]
    pub async fn index_albums(&self, documents: &[AlbumDocument]) -> ApiResult<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let index = self.client.index(indexes::ALBUMS);
        let task = index
            .add_documents(documents, Some("id"))
            .await
            .map_err(|e| ApiError::Search(format!("Failed to index albums: {}", e)))?;

        self.wait_for_task(task).await?;
        debug!(count = documents.len(), "Indexed albums");
        Ok(())
    }

    /// Index multiple artist documents
    #[instrument(skip(self, documents), fields(count = documents.len()))]
    pub async fn index_artists(&self, documents: &[ArtistDocument]) -> ApiResult<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let index = self.client.index(indexes::ARTISTS);
        let task = index
            .add_documents(documents, Some("id"))
            .await
            .map_err(|e| ApiError::Search(format!("Failed to index artists: {}", e)))?;

        self.wait_for_task(task).await?;
        debug!(count = documents.len(), "Indexed artists");
        Ok(())
    }

    /// Delete a track document by ID
    #[instrument(skip(self))]
    pub async fn delete_track(&self, track_id: Uuid) -> ApiResult<()> {
        let index = self.client.index(indexes::TRACKS);
        let task = index
            .delete_document(track_id.to_string())
            .await
            .map_err(|e| ApiError::Search(format!("Failed to delete track: {}", e)))?;

        self.wait_for_task(task).await?;
        debug!(track_id = %track_id, "Deleted track from index");
        Ok(())
    }

    /// Delete an album document by ID
    #[instrument(skip(self))]
    pub async fn delete_album(&self, album_id: Uuid) -> ApiResult<()> {
        let index = self.client.index(indexes::ALBUMS);
        let task = index
            .delete_document(album_id.to_string())
            .await
            .map_err(|e| ApiError::Search(format!("Failed to delete album: {}", e)))?;

        self.wait_for_task(task).await?;
        debug!(album_id = %album_id, "Deleted album from index");
        Ok(())
    }

    /// Delete an artist document by ID
    #[instrument(skip(self))]
    pub async fn delete_artist(&self, artist_id: Uuid) -> ApiResult<()> {
        let index = self.client.index(indexes::ARTISTS);
        let task = index
            .delete_document(artist_id.to_string())
            .await
            .map_err(|e| ApiError::Search(format!("Failed to delete artist: {}", e)))?;

        self.wait_for_task(task).await?;
        debug!(artist_id = %artist_id, "Deleted artist from index");
        Ok(())
    }

    /// Delete all documents from all indexes
    #[instrument(skip(self))]
    pub async fn clear_all_indexes(&self) -> ApiResult<()> {
        for index_name in [indexes::TRACKS, indexes::ALBUMS, indexes::ARTISTS] {
            let index = self.client.index(index_name);
            let task = index
                .delete_all_documents()
                .await
                .map_err(|e| ApiError::Search(format!("Failed to clear {}: {}", index_name, e)))?;
            self.wait_for_task(task).await?;
        }
        info!("Cleared all Meilisearch indexes");
        Ok(())
    }

    // ==================== Search Operations ====================

    /// Search tracks by query
    #[instrument(skip(self))]
    pub async fn search_tracks(
        &self,
        query: &str,
        limit: Option<usize>,
        filter: Option<&str>,
    ) -> ApiResult<Vec<TrackSearchHit>> {
        let limit = validate_limit(limit);
        let index = self.client.index(indexes::TRACKS);

        let results: SearchResults<TrackDocument> = if let Some(f) = filter {
            index
                .search()
                .with_query(query)
                .with_limit(limit)
                .with_filter(f)
                .execute()
                .await
                .map_err(|e| ApiError::Search(format!("Track search failed: {}", e)))?
        } else {
            index
                .search()
                .with_query(query)
                .with_limit(limit)
                .execute()
                .await
                .map_err(|e| ApiError::Search(format!("Track search failed: {}", e)))?
        };

        Ok(results
            .hits
            .into_iter()
            .map(|hit| TrackSearchHit::from(hit.result))
            .collect())
    }

    /// Search albums by query
    #[instrument(skip(self))]
    pub async fn search_albums(
        &self,
        query: &str,
        limit: Option<usize>,
        filter: Option<&str>,
    ) -> ApiResult<Vec<AlbumSearchHit>> {
        let limit = validate_limit(limit);
        let index = self.client.index(indexes::ALBUMS);

        let results: SearchResults<AlbumDocument> = if let Some(f) = filter {
            index
                .search()
                .with_query(query)
                .with_limit(limit)
                .with_filter(f)
                .execute()
                .await
                .map_err(|e| ApiError::Search(format!("Album search failed: {}", e)))?
        } else {
            index
                .search()
                .with_query(query)
                .with_limit(limit)
                .execute()
                .await
                .map_err(|e| ApiError::Search(format!("Album search failed: {}", e)))?
        };

        Ok(results
            .hits
            .into_iter()
            .map(|hit| AlbumSearchHit::from(hit.result))
            .collect())
    }

    /// Search artists by query
    #[instrument(skip(self))]
    pub async fn search_artists(
        &self,
        query: &str,
        limit: Option<usize>,
        filter: Option<&str>,
    ) -> ApiResult<Vec<ArtistSearchHit>> {
        let limit = validate_limit(limit);
        let index = self.client.index(indexes::ARTISTS);

        let results: SearchResults<ArtistDocument> = if let Some(f) = filter {
            index
                .search()
                .with_query(query)
                .with_limit(limit)
                .with_filter(f)
                .execute()
                .await
                .map_err(|e| ApiError::Search(format!("Artist search failed: {}", e)))?
        } else {
            index
                .search()
                .with_query(query)
                .with_limit(limit)
                .execute()
                .await
                .map_err(|e| ApiError::Search(format!("Artist search failed: {}", e)))?
        };

        Ok(results
            .hits
            .into_iter()
            .map(|hit| ArtistSearchHit::from(hit.result))
            .collect())
    }

    /// Unified search across all entity types
    ///
    /// Performs parallel searches on tracks, albums, and artists indexes.
    #[instrument(skip(self))]
    pub async fn search_all(
        &self,
        query: &str,
        limit_per_type: Option<usize>,
    ) -> ApiResult<UnifiedSearchResults> {
        let start = std::time::Instant::now();
        let limit = validate_limit(limit_per_type);

        // Run searches in parallel
        let (tracks_result, albums_result, artists_result) = tokio::join!(
            self.search_tracks(query, Some(limit), None),
            self.search_albums(query, Some(limit), None),
            self.search_artists(query, Some(limit), None),
        );

        let tracks = tracks_result?;
        let albums = albums_result?;
        let artists = artists_result?;

        let total_hits = tracks.len() + albums.len() + artists.len();

        Ok(UnifiedSearchResults {
            tracks,
            albums,
            artists,
            total_hits,
            processing_time_ms: start.elapsed().as_millis(),
        })
    }

    // ==================== Health Check ====================

    /// Check if Meilisearch is healthy and responding
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> ApiResult<MeilisearchHealthStatus> {
        let start = std::time::Instant::now();

        // Check if we can get stats (requires connection)
        match self.client.get_stats().await {
            Ok(stats) => {
                let elapsed = start.elapsed();

                // Get version info
                let version = self.client.get_version().await.map(|v| v.pkg_version).ok();

                // Get index stats
                let indexes = stats.indexes;
                let track_count = indexes.get(indexes::TRACKS).map(|s| s.number_of_documents);
                let album_count = indexes.get(indexes::ALBUMS).map(|s| s.number_of_documents);
                let artist_count = indexes.get(indexes::ARTISTS).map(|s| s.number_of_documents);

                Ok(MeilisearchHealthStatus {
                    healthy: true,
                    response_time_ms: elapsed.as_millis() as u64,
                    version,
                    track_count,
                    album_count,
                    artist_count,
                    error: None,
                })
            }
            Err(e) => {
                warn!(error = %e, "Meilisearch health check failed");
                Ok(MeilisearchHealthStatus {
                    healthy: false,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    version: None,
                    track_count: None,
                    album_count: None,
                    artist_count: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get the count of documents in each index
    #[instrument(skip(self))]
    pub async fn get_index_stats(&self) -> ApiResult<IndexStats> {
        let stats = self
            .client
            .get_stats()
            .await
            .map_err(|e| ApiError::Search(format!("Failed to get stats: {}", e)))?;

        let indexes = stats.indexes;
        Ok(IndexStats {
            tracks: indexes
                .get(indexes::TRACKS)
                .map(|s| s.number_of_documents)
                .unwrap_or(0),
            albums: indexes
                .get(indexes::ALBUMS)
                .map(|s| s.number_of_documents)
                .unwrap_or(0),
            artists: indexes
                .get(indexes::ARTISTS)
                .map(|s| s.number_of_documents)
                .unwrap_or(0),
        })
    }
}

/// Meilisearch health check status
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct MeilisearchHealthStatus {
    /// Whether Meilisearch is healthy
    pub healthy: bool,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Meilisearch version
    pub version: Option<String>,
    /// Number of indexed tracks
    pub track_count: Option<usize>,
    /// Number of indexed albums
    pub album_count: Option<usize>,
    /// Number of indexed artists
    pub artist_count: Option<usize>,
    /// Error message if unhealthy
    pub error: Option<String>,
}

/// Index document counts
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct IndexStats {
    /// Number of indexed tracks
    pub tracks: usize,
    /// Number of indexed albums
    pub albums: usize,
    /// Number of indexed artists
    pub artists: usize,
}

// Use chrono's Datelike trait for year extraction
use chrono::Datelike;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_track_document_new() {
        let track_id = Uuid::new_v4();
        let artist_id = Uuid::new_v4();
        let album_id = Uuid::new_v4();
        let now = Utc::now();

        let doc = TrackDocument::new(
            track_id,
            "Test Track".to_string(),
            artist_id,
            "Test Artist".to_string(),
            Some(album_id),
            Some("Test Album".to_string()),
            vec!["Rock".to_string()],
            vec!["happy".to_string()],
            vec!["energetic".to_string()],
            180000,
            42,
            false,
            now,
            now,
        );

        assert_eq!(doc.id, track_id.to_string());
        assert_eq!(doc.track_id, track_id);
        assert_eq!(doc.title, "Test Track");
        assert_eq!(doc.artist_name, "Test Artist");
        assert_eq!(doc.artist_id, artist_id);
        assert_eq!(doc.album_id, Some(album_id));
        assert_eq!(doc.album_title, Some("Test Album".to_string()));
        assert_eq!(doc.genres, vec!["Rock".to_string()]);
        assert_eq!(doc.moods, vec!["happy".to_string()]);
        assert_eq!(doc.duration_ms, 180000);
        assert_eq!(doc.play_count, 42);
        assert!(!doc.explicit);
    }

    #[test]
    fn test_album_document_new() {
        let album_id = Uuid::new_v4();
        let artist_id = Uuid::new_v4();
        let now = Utc::now();

        let doc = AlbumDocument::new(
            album_id,
            "Test Album".to_string(),
            artist_id,
            "Test Artist".to_string(),
            vec!["Rock".to_string()],
            "album".to_string(),
            Some(chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()),
            Some(12),
            now,
        );

        assert_eq!(doc.id, album_id.to_string());
        assert_eq!(doc.album_id, album_id);
        assert_eq!(doc.title, "Test Album");
        assert_eq!(doc.release_year, Some(2024));
    }

    #[test]
    fn test_artist_document_new() {
        let artist_id = Uuid::new_v4();
        let now = Utc::now();

        let doc = ArtistDocument::new(
            artist_id,
            "Test Artist".to_string(),
            Some("Artist, Test".to_string()),
            vec!["Rock".to_string()],
            Some("A great artist".to_string()),
            now,
        );

        assert_eq!(doc.id, artist_id.to_string());
        assert_eq!(doc.artist_id, artist_id);
        assert_eq!(doc.name, "Test Artist");
        assert_eq!(doc.sort_name, Some("Artist, Test".to_string()));
    }

    #[test]
    fn test_validate_limit() {
        assert_eq!(validate_limit(None), DEFAULT_SEARCH_LIMIT);
        assert_eq!(validate_limit(Some(10)), 10);
        assert_eq!(validate_limit(Some(0)), 1);
        assert_eq!(validate_limit(Some(200)), MAX_SEARCH_LIMIT);
    }

    #[test]
    fn test_track_search_hit_from_document() {
        let doc = TrackDocument {
            id: "123".to_string(),
            track_id: Uuid::new_v4(),
            title: "Test".to_string(),
            artist_name: "Artist".to_string(),
            artist_id: Uuid::new_v4(),
            album_title: Some("Album".to_string()),
            album_id: Some(Uuid::new_v4()),
            genres: vec!["Rock".to_string()],
            moods: vec!["happy".to_string()],
            tags: vec![],
            duration_ms: 180000,
            play_count: 0,
            explicit: false,
            created_at: 0,
            updated_at: 0,
        };

        let hit: TrackSearchHit = doc.clone().into();
        assert_eq!(hit.track_id, doc.track_id);
        assert_eq!(hit.title, doc.title);
        assert_eq!(hit.artist_name, doc.artist_name);
    }
}
