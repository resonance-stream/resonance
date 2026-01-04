//! Native Notifications
//!
//! Provides native desktop notifications for track changes and other events.
//! Uses tauri-plugin-notification for cross-platform notification support.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Wry};
use tauri_plugin_notification::NotificationExt;

/// Track information for notification display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackNotification {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub artwork_url: Option<String>,
}

/// Shows a notification when the track changes
#[tauri::command]
pub fn show_track_notification(
    app: AppHandle<Wry>,
    track: TrackNotification,
) -> Result<(), String> {
    let notification = app.notification();

    let body = if let Some(album) = &track.album {
        format!("{} \u{2022} {}", track.artist, album)
    } else {
        track.artist.clone()
    };

    notification
        .builder()
        .title(&track.title)
        .body(&body)
        .show()
        .map_err(|e| format!("Failed to show notification: {}", e))?;

    tracing::debug!(
        "Showed track notification: {} - {}",
        track.title,
        track.artist
    );

    Ok(())
}

/// Shows a generic notification
#[tauri::command]
pub fn show_notification(
    app: AppHandle<Wry>,
    title: String,
    body: Option<String>,
) -> Result<(), String> {
    let notification = app.notification();

    let mut builder = notification.builder().title(&title);

    if let Some(body_text) = body {
        builder = builder.body(&body_text);
    }

    builder
        .show()
        .map_err(|e| format!("Failed to show notification: {}", e))?;

    tracing::debug!("Showed notification: {}", title);

    Ok(())
}

/// Checks if notifications are permitted
#[tauri::command]
pub fn check_notification_permission(app: AppHandle<Wry>) -> Result<bool, String> {
    let notification = app.notification();

    notification
        .permission_state()
        .map(|state| state == tauri_plugin_notification::PermissionState::Granted)
        .map_err(|e| format!("Failed to check permission: {}", e))
}

/// Requests notification permission from the user
#[tauri::command]
pub async fn request_notification_permission(app: AppHandle<Wry>) -> Result<bool, String> {
    let notification = app.notification();

    notification
        .request_permission()
        .map(|state| state == tauri_plugin_notification::PermissionState::Granted)
        .map_err(|e| format!("Failed to request permission: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_notification_with_album() {
        let track = TrackNotification {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: Some("Test Album".to_string()),
            artwork_url: None,
        };
        assert_eq!(track.title, "Test Song");
        assert_eq!(track.artist, "Test Artist");
        assert_eq!(track.album, Some("Test Album".to_string()));
    }

    #[test]
    fn test_track_notification_without_album() {
        let track = TrackNotification {
            title: "Single Track".to_string(),
            artist: "Solo Artist".to_string(),
            album: None,
            artwork_url: None,
        };
        assert_eq!(track.title, "Single Track");
        assert!(track.album.is_none());
    }
}
