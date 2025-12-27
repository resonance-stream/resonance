//! Lidarr integration sync job
//!
//! Syncs with Lidarr to monitor for new releases from followed artists
//! and automatically add them to the library.

use anyhow::Result;
use serde::{Deserialize, Serialize};

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
#[allow(dead_code)]
struct LidarrArtist {
    id: i64,
    #[serde(rename = "artistName")]
    artist_name: String,
    monitored: bool,
}

/// Lidarr album response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LidarrAlbum {
    id: i64,
    title: String,
    #[serde(rename = "artistId")]
    artist_id: i64,
    monitored: bool,
}

/// Execute the Lidarr sync job
pub async fn execute(state: &AppState, job: &LidarrSyncJob) -> Result<()> {
    // Check if Lidarr is configured
    if !state.config.has_lidarr() {
        tracing::debug!("Lidarr not configured, skipping sync");
        return Ok(());
    }

    let lidarr_url = state.config.lidarr_url.as_ref().unwrap();
    let api_key = state.config.lidarr_api_key.as_ref().unwrap();

    tracing::info!("Starting Lidarr sync");

    if job.sync_metadata {
        sync_artists(state, lidarr_url, api_key).await?;
    }

    if job.check_new_releases {
        check_new_releases(state, lidarr_url, api_key).await?;
    }

    tracing::info!("Lidarr sync completed");

    Ok(())
}

/// Sync artist metadata from Lidarr
async fn sync_artists(state: &AppState, lidarr_url: &str, api_key: &str) -> Result<()> {
    tracing::debug!("Syncing artists from Lidarr");

    // TODO: Implement artist sync
    // 1. Fetch all monitored artists from Lidarr
    // 2. Update local artist metadata

    let _artists: Vec<LidarrArtist> = state
        .http_client
        .get(format!("{}/api/v1/artist", lidarr_url))
        .header("X-Api-Key", api_key)
        .send()
        .await?
        .json()
        .await?;

    // TODO: Update database with artist info

    Ok(())
}

/// Check for new releases from monitored artists
async fn check_new_releases(state: &AppState, lidarr_url: &str, api_key: &str) -> Result<()> {
    tracing::debug!("Checking for new releases");

    // TODO: Implement new release checking
    // 1. Fetch albums from Lidarr
    // 2. Compare with local database
    // 3. Queue library scan for new albums

    let _albums: Vec<LidarrAlbum> = state
        .http_client
        .get(format!("{}/api/v1/album", lidarr_url))
        .header("X-Api-Key", api_key)
        .send()
        .await?
        .json()
        .await?;

    // TODO: Compare with local database and queue library scans

    Ok(())
}
