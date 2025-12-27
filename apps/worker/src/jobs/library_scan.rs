//! Library scanning job
//!
//! Scans the music library directory for new, modified, or removed tracks.
//! Updates the database with track metadata and queues feature extraction jobs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::WorkerResult;
use crate::AppState;

/// Library scan job payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryScanJob {
    /// Optional: Scan only a specific subdirectory
    pub path: Option<PathBuf>,

    /// Whether to force rescan even if file hasn't changed
    pub force_rescan: bool,
}

impl Default for LibraryScanJob {
    fn default() -> Self {
        Self {
            path: None,
            force_rescan: false,
        }
    }
}

/// Execute the library scan job
pub async fn execute(state: &AppState, job: &LibraryScanJob) -> WorkerResult<()> {
    let scan_path = job
        .path
        .clone()
        .unwrap_or_else(|| state.config.music_library_path().clone());

    tracing::info!("Starting library scan: {:?}", scan_path);

    // TODO: Implement library scanning logic
    // 1. Walk the directory tree
    // 2. For each audio file (mp3, flac, ogg, etc.):
    //    a. Check if file exists in database
    //    b. Compare modification time
    //    c. If new/modified, extract metadata and update database
    //    d. Queue feature extraction job for new tracks
    // 3. Mark removed files as unavailable

    tracing::info!("Library scan completed");

    Ok(())
}

/// Supported audio file extensions
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "opus", "m4a", "aac", "wav", "aiff", "wma",
];

/// Check if a file path has a supported audio extension
pub fn is_audio_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
