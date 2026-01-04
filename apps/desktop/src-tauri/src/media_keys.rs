//! Global Media Key Shortcuts
//!
//! Registers global shortcuts for media keys (MediaPlayPause, MediaNextTrack, MediaPreviousTrack)
//! to control playback from anywhere on the system.

use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Media key shortcut identifiers
pub const MEDIA_PLAY_PAUSE: &str = "MediaPlayPause";
pub const MEDIA_NEXT_TRACK: &str = "MediaNextTrack";
pub const MEDIA_PREVIOUS_TRACK: &str = "MediaPreviousTrack";

/// Registers all media key shortcuts
pub fn register_media_keys<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let shortcuts = [MEDIA_PLAY_PAUSE, MEDIA_NEXT_TRACK, MEDIA_PREVIOUS_TRACK];

    for key in shortcuts {
        if let Err(e) = register_media_key(app, key) {
            tracing::warn!("Failed to register {}: {}", key, e);
            // Continue with other shortcuts even if one fails
        }
    }

    Ok(())
}

/// Registers a single media key shortcut
fn register_media_key<R: Runtime>(app: &AppHandle<R>, key: &str) -> Result<(), String> {
    let shortcut: Shortcut = key.parse().map_err(|e| format!("Failed to parse {}: {}", key, e))?;

    let app_handle = app.clone();
    let key_owned = key.to_string();

    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                handle_media_key(&app_handle, &key_owned);
            }
        })
        .map_err(|e| format!("Failed to register shortcut {}: {}", key, e))?;

    tracing::info!("Registered global shortcut: {}", key);
    Ok(())
}

/// Handles media key press events
fn handle_media_key<R: Runtime>(app: &AppHandle<R>, key: &str) {
    let command = match key {
        MEDIA_PLAY_PAUSE => "toggle-playback",
        MEDIA_NEXT_TRACK => "next-track",
        MEDIA_PREVIOUS_TRACK => "previous-track",
        _ => return,
    };

    tracing::debug!("Media key pressed: {} -> {}", key, command);

    if let Err(e) = app.emit("playback-command", command) {
        tracing::error!("Failed to emit playback command: {}", e);
    }
}

/// Unregisters all media key shortcuts
#[allow(dead_code)]
pub fn unregister_media_keys<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let shortcuts = [MEDIA_PLAY_PAUSE, MEDIA_NEXT_TRACK, MEDIA_PREVIOUS_TRACK];

    for key in shortcuts {
        if let Ok(shortcut) = key.parse::<Shortcut>() {
            if let Err(e) = app.global_shortcut().unregister(shortcut) {
                tracing::warn!("Failed to unregister {}: {}", key, e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_key_constants() {
        assert_eq!(MEDIA_PLAY_PAUSE, "MediaPlayPause");
        assert_eq!(MEDIA_NEXT_TRACK, "MediaNextTrack");
        assert_eq!(MEDIA_PREVIOUS_TRACK, "MediaPreviousTrack");
    }
}
