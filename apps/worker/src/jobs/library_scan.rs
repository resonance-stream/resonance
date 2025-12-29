//! Library scanning job
//!
//! Scans the music library directory for new, modified, or removed tracks.
//! Updates the database with track metadata and queues feature extraction jobs.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use lofty::{Accessor, AudioFile, ItemKey, Probe, TaggedFileExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::error::{WorkerError, WorkerResult};
use crate::jobs::{enqueue_job, feature_extraction::FeatureExtractionJob, Job};
use crate::AppState;

/// Library scan job payload
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryScanJob {
    /// Optional: Scan only a specific subdirectory
    pub path: Option<PathBuf>,

    /// Whether to force rescan even if file hasn't changed
    #[serde(default)]
    pub force_rescan: bool,
}

/// Supported audio file extensions
/// Includes common lossy and lossless formats
pub const AUDIO_EXTENSIONS: &[&str] = &[
    // Lossy formats
    "mp3", "ogg", "opus", "aac", "m4a", "wma", "mpc", "ape", // Lossless formats
    "flac", "wav", "aiff", "aif", "alac", "dsf", "dff", "wv", // Container formats
    "m4b", "m4p", "m4r", "mp4", "3gp", "webm",
];

/// Map file extension to audio_format enum value
/// Based on the PostgreSQL enum: flac, mp3, aac, opus, ogg, wav, alac, other
fn extension_to_audio_format(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "mp3" => "mp3",
        "flac" => "flac",
        "ogg" => "ogg",
        "opus" => "opus",
        "aac" | "m4a" | "m4b" | "m4p" | "m4r" | "mp4" | "3gp" => "aac",
        "wav" | "aiff" | "aif" => "wav",
        "alac" => "alac",
        "webm" => "opus", // WebM usually contains Opus
        _ => "other",
    }
}

/// Check if a file path has a supported audio extension
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Track record from database
#[derive(Debug, sqlx::FromRow)]
struct DbTrack {
    id: Uuid,
    file_path: String,
    file_hash: Option<String>,
}

/// Extracted metadata from an audio file
#[derive(Debug)]
struct AudioMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    track_number: Option<i16>,
    disc_number: Option<i16>,
    year: Option<i32>,
    genre: Option<String>,
    duration_ms: i32,
    file_size: i64,
    file_hash: String,
    bit_rate: Option<i32>,
    sample_rate: Option<i32>,
    channels: Option<i16>,
    format: String,
}

/// Execute the library scan job
pub async fn execute(state: &AppState, job: &LibraryScanJob) -> WorkerResult<()> {
    let library_path = state.config.music_library_path();
    let scan_path = job.path.clone().unwrap_or_else(|| library_path.clone());

    tracing::info!("Starting library scan: {:?}", scan_path);

    if !scan_path.exists() {
        return Err(WorkerError::Configuration(format!(
            "Music library path does not exist: {:?}",
            scan_path
        )));
    }

    // Security: Canonicalize paths and verify scan_path is within library
    let canonical_library = library_path.canonicalize().map_err(|e| {
        WorkerError::Configuration(format!("Failed to canonicalize library path: {}", e))
    })?;
    let canonical_scan = scan_path.canonicalize().map_err(|e| {
        WorkerError::Configuration(format!("Failed to canonicalize scan path: {}", e))
    })?;

    if !canonical_scan.starts_with(&canonical_library) {
        return Err(WorkerError::Configuration(format!(
            "Scan path {:?} is outside the music library {:?}",
            canonical_scan, canonical_library
        )));
    }

    // Get existing tracks from database for comparison
    let existing_tracks = get_existing_tracks(&state.db).await?;
    let existing_paths: HashSet<String> = existing_tracks
        .iter()
        .map(|t| t.file_path.clone())
        .collect();
    let mut found_paths: HashSet<String> = HashSet::new();

    let mut new_count = 0;
    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    // Walk the directory tree
    for entry in WalkDir::new(&scan_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories and non-audio files
        if !path.is_file() || !is_audio_file(path) {
            continue;
        }

        let path_str = path.to_string_lossy().to_string();
        found_paths.insert(path_str.clone());

        // Check if file exists in database
        let existing = existing_tracks.iter().find(|t| t.file_path == path_str);

        // Process the file
        match process_audio_file(state, path, existing, job.force_rescan).await {
            Ok(ProcessResult::New(track_id)) => {
                new_count += 1;
                // Queue feature extraction for new tracks
                let extraction_job = Job::FeatureExtraction(FeatureExtractionJob {
                    track_id: track_id.to_string(),
                });
                if let Err(e) = enqueue_job(&state.redis, &extraction_job).await {
                    tracing::warn!(
                        "Failed to queue feature extraction for track {}: {}",
                        track_id,
                        e
                    );
                }
            }
            Ok(ProcessResult::Updated(_)) => {
                updated_count += 1;
            }
            Ok(ProcessResult::Skipped) => {
                skipped_count += 1;
            }
            Err(e) => {
                tracing::warn!("Failed to process {:?}: {}", path, e);
                error_count += 1;
            }
        }
    }

    // Mark removed files as unavailable
    let removed_paths: Vec<&String> = existing_paths.difference(&found_paths).collect();
    let removed_count = removed_paths.len();

    if !removed_paths.is_empty() {
        mark_tracks_unavailable(&state.db, &removed_paths).await?;
    }

    tracing::info!(
        "Library scan completed: {} new, {} updated, {} skipped, {} removed, {} errors",
        new_count,
        updated_count,
        skipped_count,
        removed_count,
        error_count
    );

    Ok(())
}

/// Result of processing a single audio file
#[allow(dead_code)]
enum ProcessResult {
    New(Uuid),
    Updated(Uuid),
    Skipped,
}

/// Process a single audio file
async fn process_audio_file(
    state: &AppState,
    path: &Path,
    existing: Option<&DbTrack>,
    force_rescan: bool,
) -> WorkerResult<ProcessResult> {
    // Compute file hash
    let file_hash = compute_file_hash(path)?;

    // Check if we need to process this file
    if !force_rescan {
        if let Some(track) = existing {
            if track.file_hash.as_ref() == Some(&file_hash) {
                return Ok(ProcessResult::Skipped);
            }
        }
    }

    // Extract metadata
    let metadata = extract_metadata(path, &file_hash)?;

    // Get or create artist (always required)
    let artist_name = metadata.artist.as_deref().unwrap_or("Unknown Artist");
    let artist_id = get_or_create_artist(&state.db, artist_name).await?;

    // Get or create album (optional)
    let album_id = if let Some(ref album_name) = metadata.album {
        let album_artist = metadata.album_artist.as_ref().or(metadata.artist.as_ref());
        Some(
            get_or_create_album(
                &state.db,
                album_name,
                album_artist,
                artist_id,
                metadata.year,
            )
            .await?,
        )
    } else {
        None
    };

    // Insert or update track
    let path_str = path.to_string_lossy().to_string();

    if let Some(track) = existing {
        // Update existing track
        update_track(&state.db, track.id, &metadata, artist_id, album_id).await?;
        Ok(ProcessResult::Updated(track.id))
    } else {
        // Insert new track
        let track_id = insert_track(&state.db, &path_str, &metadata, artist_id, album_id).await?;
        Ok(ProcessResult::New(track_id))
    }
}

/// Compute SHA-256 hash of a file
fn compute_file_hash(path: &Path) -> WorkerResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Extract metadata from an audio file using lofty
fn extract_metadata(path: &Path, file_hash: &str) -> WorkerResult<AudioMetadata> {
    let file_size = fs::metadata(path)?.len() as i64;

    let tagged_file = Probe::open(path)
        .map_err(|e| WorkerError::AudioProcessing(format!("Failed to open audio file: {}", e)))?
        .read()
        .map_err(|e| WorkerError::AudioProcessing(format!("Failed to read audio file: {}", e)))?;

    let properties = tagged_file.properties();
    let duration_ms = properties.duration().as_millis() as i32;
    let bit_rate = properties.audio_bitrate().map(|b| b as i32);
    let sample_rate = properties.sample_rate().map(|s| s as i32);
    let channels = properties.channels().map(|c| c as i16);

    // Determine format from file extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| "other".to_string());
    let format = extension_to_audio_format(&ext).to_string();

    // Extract tags (try all tag types and use the first one with data)
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let (title, artist, album, album_artist, track_number, disc_number, year, genre) =
        if let Some(tag) = tag {
            // Extract string values from tags
            let title_val = tag.title().map(|s| s.into_owned());
            let artist_val = tag.artist().map(|s| s.into_owned());
            let album_val = tag.album().map(|s| s.into_owned());
            let album_artist_val = tag.get_string(&ItemKey::AlbumArtist).map(|s| s.to_string());
            let genre_val = tag.genre().map(|g| g.into_owned());

            (
                title_val,
                artist_val,
                album_val,
                album_artist_val,
                tag.track().map(|t| t as i16),
                tag.disk().map(|d| d as i16),
                tag.year().map(|y| y as i32),
                genre_val,
            )
        } else {
            // Fallback: use filename as title
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            (title, None, None, None, None, None, None, None)
        };

    Ok(AudioMetadata {
        title,
        artist,
        album,
        album_artist,
        track_number,
        disc_number,
        year,
        genre,
        duration_ms,
        file_size,
        file_hash: file_hash.to_string(),
        bit_rate,
        sample_rate,
        channels,
        format,
    })
}

/// Get all existing tracks from database
async fn get_existing_tracks(db: &sqlx::PgPool) -> WorkerResult<Vec<DbTrack>> {
    let tracks = sqlx::query_as::<_, DbTrack>(
        "SELECT id, file_path, file_hash FROM tracks WHERE is_available = true",
    )
    .fetch_all(db)
    .await?;

    Ok(tracks)
}

/// Get or create an artist by name using upsert pattern to avoid race conditions
async fn get_or_create_artist(db: &sqlx::PgPool, name: &str) -> WorkerResult<Uuid> {
    // Use INSERT ... ON CONFLICT to handle race conditions atomically
    // This requires a unique index on LOWER(name) - see migration 20250101000011
    let row = sqlx::query(
        r#"
        INSERT INTO artists (name)
        VALUES ($1)
        ON CONFLICT ((LOWER(name))) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
    )
    .bind(name)
    .fetch_one(db)
    .await?;

    let id: Uuid = row.get("id");
    tracing::debug!("Got or created artist: {} (ID: {})", name, id);

    Ok(id)
}

/// Get or create an album by name using upsert pattern to avoid race conditions
async fn get_or_create_album(
    db: &sqlx::PgPool,
    name: &str,
    artist_name: Option<&String>,
    artist_id: Uuid,
    year: Option<i32>,
) -> WorkerResult<Uuid> {
    // Create release_date from year if available
    let release_date = year.and_then(|y| chrono::NaiveDate::from_ymd_opt(y, 1, 1));

    // Use INSERT ... ON CONFLICT to handle race conditions atomically
    // This requires a unique index on (LOWER(title), artist_id) - see migration 20250101000011
    let row = sqlx::query(
        r#"
        INSERT INTO albums (title, artist_id, release_date)
        VALUES ($1, $2, $3)
        ON CONFLICT ((LOWER(title)), artist_id) DO UPDATE SET
            release_date = COALESCE(albums.release_date, EXCLUDED.release_date)
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(artist_id)
    .bind(release_date)
    .fetch_one(db)
    .await?;

    let id: Uuid = row.get("id");
    tracing::debug!(
        "Got or created album: {} by {} (ID: {})",
        name,
        artist_name.unwrap_or(&"Unknown".to_string()),
        id
    );

    Ok(id)
}

/// Insert a new track into the database
async fn insert_track(
    db: &sqlx::PgPool,
    file_path: &str,
    metadata: &AudioMetadata,
    artist_id: Uuid,
    album_id: Option<Uuid>,
) -> WorkerResult<Uuid> {
    // Use filename as title if no title in metadata
    let title = metadata.title.clone().unwrap_or_else(|| {
        Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    });

    let genres: Vec<String> = metadata.genre.iter().cloned().collect();

    let row = sqlx::query(
        r#"
        INSERT INTO tracks (
            title, artist_id, album_id, track_number, disc_number,
            duration_ms, file_path, file_size, file_hash, file_format,
            bit_rate, sample_rate, channels, genres, is_available
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::audio_format, $11, $12, $13, $14, true)
        RETURNING id
        "#,
    )
    .bind(&title)
    .bind(artist_id)
    .bind(album_id)
    .bind(metadata.track_number)
    .bind(metadata.disc_number.unwrap_or(1))
    .bind(metadata.duration_ms)
    .bind(file_path)
    .bind(metadata.file_size)
    .bind(&metadata.file_hash)
    .bind(&metadata.format)
    .bind(metadata.bit_rate)
    .bind(metadata.sample_rate)
    .bind(metadata.channels)
    .bind(&genres)
    .fetch_one(db)
    .await?;

    let track_id: Uuid = row.get("id");
    tracing::debug!("Inserted new track: {} (ID: {})", title, track_id);

    Ok(track_id)
}

/// Update an existing track in the database
async fn update_track(
    db: &sqlx::PgPool,
    track_id: Uuid,
    metadata: &AudioMetadata,
    artist_id: Uuid,
    album_id: Option<Uuid>,
) -> WorkerResult<()> {
    let title = metadata
        .title
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());
    let genres: Vec<String> = metadata.genre.iter().cloned().collect();

    sqlx::query(
        r#"
        UPDATE tracks SET
            title = $1,
            artist_id = $2,
            album_id = $3,
            track_number = $4,
            disc_number = $5,
            duration_ms = $6,
            file_size = $7,
            file_hash = $8,
            file_format = $9::audio_format,
            bit_rate = $10,
            sample_rate = $11,
            channels = $12,
            genres = $13,
            is_available = true,
            updated_at = NOW()
        WHERE id = $14
        "#,
    )
    .bind(&title)
    .bind(artist_id)
    .bind(album_id)
    .bind(metadata.track_number)
    .bind(metadata.disc_number.unwrap_or(1))
    .bind(metadata.duration_ms)
    .bind(metadata.file_size)
    .bind(&metadata.file_hash)
    .bind(&metadata.format)
    .bind(metadata.bit_rate)
    .bind(metadata.sample_rate)
    .bind(metadata.channels)
    .bind(&genres)
    .bind(track_id)
    .execute(db)
    .await?;

    tracing::debug!("Updated track ID: {}", track_id);

    Ok(())
}

/// Mark tracks as unavailable (file no longer exists) using batch update
async fn mark_tracks_unavailable(db: &sqlx::PgPool, paths: &[&String]) -> WorkerResult<()> {
    if paths.is_empty() {
        return Ok(());
    }

    // Convert to owned strings for the query
    let path_vec: Vec<String> = paths.iter().map(|s| (*s).clone()).collect();

    // Use batch update with ANY for efficiency
    let result = sqlx::query(
        "UPDATE tracks SET is_available = false, updated_at = NOW() WHERE file_path = ANY($1)",
    )
    .bind(&path_vec)
    .execute(db)
    .await?;

    tracing::info!(
        "Marked {} tracks as unavailable (batch update)",
        result.rows_affected()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_audio_file() {
        assert!(is_audio_file(Path::new("/music/song.mp3")));
        assert!(is_audio_file(Path::new("/music/song.FLAC")));
        assert!(is_audio_file(Path::new("/music/song.ogg")));
        assert!(is_audio_file(Path::new("/music/song.m4a")));
        assert!(!is_audio_file(Path::new("/music/image.jpg")));
        assert!(!is_audio_file(Path::new("/music/document.pdf")));
        assert!(!is_audio_file(Path::new("/music/folder")));
    }

    #[test]
    fn test_audio_extensions() {
        // Should have a reasonable number of supported formats
        assert!(AUDIO_EXTENSIONS.len() >= 15);
        assert!(AUDIO_EXTENSIONS.contains(&"mp3"));
        assert!(AUDIO_EXTENSIONS.contains(&"flac"));
        assert!(AUDIO_EXTENSIONS.contains(&"opus"));
        assert!(AUDIO_EXTENSIONS.contains(&"alac"));
        assert!(AUDIO_EXTENSIONS.contains(&"wav"));
    }

    #[test]
    fn test_extension_to_audio_format() {
        assert_eq!(extension_to_audio_format("mp3"), "mp3");
        assert_eq!(extension_to_audio_format("MP3"), "mp3");
        assert_eq!(extension_to_audio_format("flac"), "flac");
        assert_eq!(extension_to_audio_format("m4a"), "aac");
        assert_eq!(extension_to_audio_format("wma"), "other");
        assert_eq!(extension_to_audio_format("unknown"), "other");
    }
}
