//! Lidarr integration sync job
//!
//! Syncs with Lidarr to monitor for new releases from followed artists
//! and automatically add them to the library.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::error::{WorkerError, WorkerResult};
use crate::jobs::{enqueue_job, library_scan::LibraryScanJob, Job};
use crate::AppState;

/// Lidarr sync job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidarrSyncJob {
    /// Whether to check for new releases from monitored artists
    pub check_new_releases: bool,

    /// Whether to sync artist metadata
    pub sync_metadata: bool,
}

impl Default for LidarrSyncJob {
    fn default() -> Self {
        Self {
            check_new_releases: true,
            sync_metadata: true,
        }
    }
}

/// Lidarr artist response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LidarrArtist {
    id: i64,
    artist_name: String,
    sort_name: Option<String>,
    overview: Option<String>,
    monitored: bool,
    path: Option<String>,
    foreign_artist_id: Option<String>,
    #[serde(default)]
    genres: Vec<String>,
    #[serde(default)]
    images: Vec<LidarrImage>,
}

/// Lidarr album response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct LidarrAlbum {
    id: i64,
    title: String,
    artist_id: i64,
    monitored: bool,
    foreign_album_id: Option<String>,
    release_date: Option<String>,
    #[serde(default)]
    genres: Vec<String>,
    album_type: Option<String>,
    /// Statistics about the album
    #[serde(default)]
    statistics: AlbumStatistics,
}

/// Album statistics from Lidarr
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct AlbumStatistics {
    /// Total tracks in the album
    total_track_count: i32,
    /// Number of tracks on disk
    track_file_count: i32,
    /// Size in bytes
    size_on_disk: i64,
    /// Percent of tracks on disk
    percent_of_tracks: f64,
}

/// Lidarr image reference
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LidarrImage {
    cover_type: String,
    url: String,
}

/// Execute the Lidarr sync job
pub async fn execute(state: &AppState, job: &LidarrSyncJob) -> WorkerResult<()> {
    // Check if Lidarr is configured
    let lidarr_config = match state.config.lidarr() {
        Some(config) => config,
        None => {
            tracing::debug!("Lidarr not configured, skipping sync");
            return Ok(());
        }
    };

    let lidarr_url = &lidarr_config.url;
    let api_key = &lidarr_config.api_key;

    tracing::info!("Starting Lidarr sync");

    // Fetch all artists once (used by both sync_artists and check_new_releases)
    let artists = fetch_all_artists(state, lidarr_url, api_key).await?;

    // Build artist path cache for efficient lookup
    let artist_paths: HashMap<i64, String> = artists
        .iter()
        .filter_map(|a| a.path.as_ref().map(|p| (a.id, p.clone())))
        .collect();

    if job.sync_metadata {
        sync_artists_from_data(state, &artists).await?;
    }

    if job.check_new_releases {
        check_new_releases(state, lidarr_url, api_key, &artist_paths).await?;
    }

    tracing::info!("Lidarr sync completed");

    Ok(())
}

/// Fetch all artists from Lidarr API with proper error handling
async fn fetch_all_artists(
    state: &AppState,
    lidarr_url: &str,
    api_key: &str,
) -> WorkerResult<Vec<LidarrArtist>> {
    let response = state
        .http_client
        .get(format!("{}/api/v1/artist", lidarr_url))
        .header("X-Api-Key", api_key)
        .send()
        .await?;

    // Check HTTP status before parsing JSON
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(WorkerError::lidarr_api(status, body));
    }

    let artists: Vec<LidarrArtist> = response.json().await?;
    Ok(artists)
}

/// Fetch all albums from Lidarr API with proper error handling
async fn fetch_all_albums(
    state: &AppState,
    lidarr_url: &str,
    api_key: &str,
) -> WorkerResult<Vec<LidarrAlbum>> {
    let response = state
        .http_client
        .get(format!("{}/api/v1/album", lidarr_url))
        .header("X-Api-Key", api_key)
        .send()
        .await?;

    // Check HTTP status before parsing JSON
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(WorkerError::lidarr_api(status, body));
    }

    let albums: Vec<LidarrAlbum> = response.json().await?;
    Ok(albums)
}

/// Sync artist metadata from pre-fetched Lidarr data
async fn sync_artists_from_data(state: &AppState, artists: &[LidarrArtist]) -> WorkerResult<()> {
    tracing::debug!("Syncing artists from Lidarr");

    let monitored_artists: Vec<&LidarrArtist> = artists.iter().filter(|a| a.monitored).collect();

    tracing::info!(
        "Found {} monitored artists in Lidarr",
        monitored_artists.len()
    );

    let mut created = 0;
    let mut updated = 0;

    for artist in monitored_artists {
        // Find poster image URL
        let image_url = artist
            .images
            .iter()
            .find(|img| img.cover_type == "poster" || img.cover_type == "fanart")
            .map(|img| img.url.clone());

        // Parse MusicBrainz ID if available
        let mbid = artist
            .foreign_artist_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok());

        // Upsert artist into database
        let result = upsert_artist(
            &state.db,
            artist.id,
            &artist.artist_name,
            artist.sort_name.as_deref(),
            artist.overview.as_deref(),
            image_url.as_deref(),
            &artist.genres,
            mbid,
        )
        .await?;

        if result {
            created += 1;
        } else {
            updated += 1;
        }
    }

    tracing::info!(
        "Artist sync complete: {} created, {} updated",
        created,
        updated
    );

    Ok(())
}

/// Upsert an artist into the database using ON CONFLICT for race-condition safety
/// Returns true if created, false if updated
#[allow(clippy::too_many_arguments)]
async fn upsert_artist(
    db: &sqlx::PgPool,
    lidarr_id: i64,
    name: &str,
    sort_name: Option<&str>,
    biography: Option<&str>,
    image_url: Option<&str>,
    genres: &[String],
    mbid: Option<Uuid>,
) -> WorkerResult<bool> {
    // Safely convert i64 to i32 to prevent silent truncation
    let lidarr_id_i32 = i32::try_from(lidarr_id).map_err(|_| {
        WorkerError::InvalidJobData(format!("Lidarr artist ID out of i32 range: {}", lidarr_id))
    })?;

    // Use ON CONFLICT to handle race conditions atomically
    // xmax = 0 indicates a fresh insert (no previous version existed)
    let row = sqlx::query(
        r#"
        INSERT INTO artists (name, sort_name, biography, image_url, genres, lidarr_id, mbid)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (lidarr_id) DO UPDATE SET
            name = EXCLUDED.name,
            sort_name = COALESCE(EXCLUDED.sort_name, artists.sort_name),
            biography = COALESCE(EXCLUDED.biography, artists.biography),
            image_url = COALESCE(EXCLUDED.image_url, artists.image_url),
            genres = EXCLUDED.genres,
            mbid = COALESCE(EXCLUDED.mbid, artists.mbid),
            updated_at = NOW()
        RETURNING (xmax = 0) AS inserted
        "#,
    )
    .bind(name)
    .bind(sort_name)
    .bind(biography)
    .bind(image_url)
    .bind(genres)
    .bind(lidarr_id_i32)
    .bind(mbid)
    .fetch_one(db)
    .await?;

    let inserted: bool = row.get("inserted");

    if inserted {
        tracing::debug!("Created artist: {} (lidarr_id: {})", name, lidarr_id);
    } else {
        tracing::debug!("Updated artist: {} (lidarr_id: {})", name, lidarr_id);
    }

    Ok(inserted)
}

/// Check for new releases from monitored artists
async fn check_new_releases(
    state: &AppState,
    lidarr_url: &str,
    api_key: &str,
    artist_paths: &HashMap<i64, String>,
) -> WorkerResult<()> {
    tracing::debug!("Checking for new releases");

    // Fetch all albums from Lidarr
    let albums = fetch_all_albums(state, lidarr_url, api_key).await?;

    // Filter to albums that have tracks on disk
    let albums_with_files: Vec<&LidarrAlbum> = albums
        .iter()
        .filter(|a| a.statistics.track_file_count > 0)
        .collect();

    tracing::info!(
        "Found {} albums with tracks on disk",
        albums_with_files.len()
    );

    // Get local albums with lidarr_id to see what's already imported
    let local_album_ids: HashSet<i64> = get_local_lidarr_album_ids(&state.db).await?;

    // Find albums that are in Lidarr but not yet in our database
    let mut new_album_count = 0;
    let mut scan_paths: HashSet<String> = HashSet::new();

    for album in &albums_with_files {
        if !local_album_ids.contains(&album.id) {
            new_album_count += 1;

            // Use cached artist path (no additional API call needed)
            if let Some(path) = artist_paths.get(&album.artist_id) {
                scan_paths.insert(path.clone());
            }
        }
    }

    tracing::info!(
        "Found {} new albums to import across {} artist paths",
        new_album_count,
        scan_paths.len()
    );

    // Validate and queue library scans for each artist directory
    let canonical_library = state
        .config
        .music_library_path()
        .canonicalize()
        .map_err(|e| {
            WorkerError::Configuration(format!("Failed to canonicalize library path: {}", e))
        })?;

    for path in scan_paths {
        // Validate path is within music library before queueing
        let candidate = PathBuf::from(&path);
        let canonical_candidate = match candidate.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    "Skipping Lidarr scan path (cannot canonicalize) {}: {}",
                    path,
                    e
                );
                continue;
            }
        };

        if !canonical_candidate.starts_with(&canonical_library) {
            tracing::warn!(
                "Skipping Lidarr scan path outside library: {} (library: {:?})",
                path,
                canonical_library
            );
            continue;
        }

        let scan_job = Job::LibraryScan(LibraryScanJob {
            path: Some(canonical_candidate),
            force_rescan: false,
        });

        if let Err(e) = enqueue_job(&state.redis, &scan_job).await {
            tracing::warn!("Failed to queue library scan for {}: {}", path, e);
        } else {
            tracing::info!("Queued library scan for: {}", path);
        }
    }

    // Also sync album metadata for existing albums
    sync_album_metadata(state, &albums).await?;

    Ok(())
}

/// Get all local album IDs that have a lidarr_id set
async fn get_local_lidarr_album_ids(db: &sqlx::PgPool) -> WorkerResult<HashSet<i64>> {
    let rows = sqlx::query("SELECT lidarr_id FROM albums WHERE lidarr_id IS NOT NULL")
        .fetch_all(db)
        .await?;

    let ids: HashSet<i64> = rows
        .iter()
        .map(|r| r.get::<i32, _>("lidarr_id") as i64)
        .collect();

    Ok(ids)
}

/// Sync album metadata from Lidarr to local database
async fn sync_album_metadata(state: &AppState, albums: &[LidarrAlbum]) -> WorkerResult<()> {
    let mut updated = 0;

    for album in albums {
        // Try to find existing album by lidarr_id or by matching title and artist
        let local_album: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM albums WHERE lidarr_id = $1")
                .bind(album.id as i32)
                .fetch_optional(&state.db)
                .await?;

        if let Some((album_uuid,)) = local_album {
            // Parse release date if available using safe string slicing
            let release_date = album.release_date.as_ref().and_then(|d| {
                d.get(..10).and_then(|date_str| {
                    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
                })
            });

            // Parse MusicBrainz ID if available
            let mbid = album
                .foreign_album_id
                .as_ref()
                .and_then(|id| Uuid::parse_str(id).ok());

            // Map album type
            let album_type = match album.album_type.as_deref() {
                Some("single") => "single",
                Some("ep") => "ep",
                Some("compilation") => "compilation",
                Some("live") => "live",
                _ => "album",
            };

            // Update album metadata
            sqlx::query(
                r#"
                UPDATE albums SET
                    release_date = COALESCE($1, release_date),
                    genres = CASE WHEN array_length($2::text[], 1) > 0 THEN $2 ELSE genres END,
                    album_type = $3::album_type,
                    mbid = COALESCE($4, mbid),
                    total_tracks = COALESCE($5, total_tracks),
                    updated_at = NOW()
                WHERE id = $6
                "#,
            )
            .bind(release_date)
            .bind(&album.genres)
            .bind(album_type)
            .bind(mbid)
            .bind(album.statistics.total_track_count)
            .bind(album_uuid)
            .execute(&state.db)
            .await?;

            updated += 1;
        } else {
            // Try to match by title and artist name
            // First, get artist name from Lidarr
            if let Some(artist) = get_artist_by_lidarr_id(&state.db, album.artist_id).await? {
                // Try to find album by title and artist
                let found: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM albums WHERE LOWER(title) = LOWER($1) AND artist_id = $2",
                )
                .bind(&album.title)
                .bind(artist)
                .fetch_optional(&state.db)
                .await?;

                if let Some((album_uuid,)) = found {
                    // Link album to Lidarr
                    sqlx::query("UPDATE albums SET lidarr_id = $1 WHERE id = $2")
                        .bind(album.id as i32)
                        .bind(album_uuid)
                        .execute(&state.db)
                        .await?;

                    tracing::debug!(
                        "Linked album '{}' to Lidarr (lidarr_id: {})",
                        album.title,
                        album.id
                    );
                    updated += 1;
                }
            }
        }
    }

    if updated > 0 {
        tracing::info!("Updated {} album records with Lidarr metadata", updated);
    }

    Ok(())
}

/// Get artist UUID by Lidarr ID
async fn get_artist_by_lidarr_id(db: &sqlx::PgPool, lidarr_id: i64) -> WorkerResult<Option<Uuid>> {
    let result: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM artists WHERE lidarr_id = $1")
        .bind(lidarr_id as i32)
        .fetch_optional(db)
        .await?;

    Ok(result.map(|(id,)| id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_lidarr_sync_job() {
        let job = LidarrSyncJob::default();
        assert!(job.check_new_releases);
        assert!(job.sync_metadata);
    }
}
