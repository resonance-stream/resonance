//! Auto-Updater Integration
//!
//! Provides automatic update checking and installation for the desktop application.
//! Uses tauri-plugin-updater with a configured update endpoint.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Wry};
use tauri_plugin_updater::UpdaterExt;

/// Update status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
}

/// Update progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percentage: Option<f32>,
}

/// Checks for available updates
#[tauri::command]
pub async fn check_for_updates(app: AppHandle<Wry>) -> Result<UpdateStatus, String> {
    let updater = app
        .updater()
        .map_err(|e| format!("Updater not available: {}", e))?;
    let current_version = app.package_info().version.to_string();

    match updater.check().await {
        Ok(Some(update)) => {
            tracing::info!(
                "Update available: {} -> {}",
                current_version,
                update.version
            );
            Ok(UpdateStatus {
                available: true,
                current_version,
                latest_version: Some(update.version.clone()),
                release_notes: update.body.clone(),
                download_url: None,
            })
        }
        Ok(None) => {
            tracing::debug!("No update available, current version: {}", current_version);
            Ok(UpdateStatus {
                available: false,
                current_version,
                latest_version: None,
                release_notes: None,
                download_url: None,
            })
        }
        Err(e) => {
            tracing::error!("Failed to check for updates: {}", e);
            Err(format!("Failed to check for updates: {}", e))
        }
    }
}

/// Downloads and installs the available update
#[tauri::command]
pub async fn install_update(app: AppHandle<Wry>) -> Result<(), String> {
    let updater = app
        .updater()
        .map_err(|e| format!("Updater not available: {}", e))?;

    let update = updater
        .check()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))?
        .ok_or_else(|| "No update available".to_string())?;

    tracing::info!("Downloading update: {}", update.version);

    // Download the update
    let bytes = update
        .download(
            |chunk_length, content_length| {
                let percentage =
                    content_length.map(|total| (chunk_length as f32 / total as f32) * 100.0);
                tracing::debug!(
                    "Download progress: {} / {:?} bytes ({:?}%)",
                    chunk_length,
                    content_length,
                    percentage
                );
            },
            || {
                tracing::debug!("Download complete, preparing to install...");
            },
        )
        .await
        .map_err(|e| format!("Failed to download update: {}", e))?;

    tracing::info!("Installing update...");

    // Install and restart
    update
        .install(bytes)
        .map_err(|e| format!("Failed to install update: {}", e))?;

    // Restart the application to apply the update
    tracing::info!("Update installed, restarting application...");
    app.restart();
}

/// Gets the current application version
#[tauri::command]
pub fn get_current_version(app: AppHandle<Wry>) -> String {
    app.package_info().version.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_status_available() {
        let status = UpdateStatus {
            available: true,
            current_version: "0.1.0".to_string(),
            latest_version: Some("0.2.0".to_string()),
            release_notes: Some("Bug fixes and improvements".to_string()),
            download_url: None,
        };
        assert!(status.available);
        assert_eq!(status.latest_version, Some("0.2.0".to_string()));
    }

    #[test]
    fn test_update_status_not_available() {
        let status = UpdateStatus {
            available: false,
            current_version: "0.2.0".to_string(),
            latest_version: None,
            release_notes: None,
            download_url: None,
        };
        assert!(!status.available);
        assert!(status.latest_version.is_none());
    }

    #[test]
    fn test_update_progress() {
        let progress = UpdateProgress {
            downloaded: 5_000_000,
            total: Some(10_000_000),
            percentage: Some(50.0),
        };
        assert_eq!(progress.downloaded, 5_000_000);
        assert_eq!(progress.percentage, Some(50.0));
    }
}
