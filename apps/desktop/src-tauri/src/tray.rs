//! System Tray Integration
//!
//! Provides system tray functionality with dynamic menu showing current playback state,
//! playback controls, and minimize-to-tray behavior.

use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, Window, Wry};

/// Playback state for updating tray menu
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub track_title: Option<String>,
    pub artist_name: Option<String>,
}

/// Creates the system tray icon and menu
pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> Result<TrayIcon<R>, tauri::Error> {
    let menu = build_tray_menu(app, &PlaybackState::default())?;

    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Resonance")
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_icon_event)
        .build(app)
}

/// Builds the tray menu with current playback state
fn build_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    state: &PlaybackState,
) -> Result<Menu<R>, tauri::Error> {
    let now_playing = if let (Some(title), Some(artist)) = (&state.track_title, &state.artist_name)
    {
        format!("{} - {}", title, artist)
    } else {
        "Not Playing".to_string()
    };

    let play_pause_label = if state.is_playing { "Pause" } else { "Play" };

    // Create menu items
    let now_playing_item = MenuItem::with_id(app, "now-playing", &now_playing, false, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;
    let play_pause = MenuItem::with_id(app, "play-pause", play_pause_label, true, None::<&str>)?;
    let previous = MenuItem::with_id(app, "previous", "Previous Track", true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next", "Next Track", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let show_window = MenuItem::with_id(app, "show", "Show Resonance", true, None::<&str>)?;
    let separator3 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &now_playing_item,
            &separator1,
            &play_pause,
            &previous,
            &next,
            &separator2,
            &show_window,
            &separator3,
            &quit,
        ],
    )
}

/// Updates the tray menu with new playback state
pub fn update_tray_menu(app: &AppHandle<Wry>, state: &PlaybackState) -> Result<(), tauri::Error> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let menu = build_tray_menu(app, state)?;
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

/// Handles menu item clicks
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        "play-pause" => {
            emit_playback_command(app, "toggle-playback");
        }
        "previous" => {
            emit_playback_command(app, "previous-track");
        }
        "next" => {
            emit_playback_command(app, "next-track");
        }
        "show" => {
            show_main_window(app);
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

/// Handles tray icon events (left-click to show window)
fn handle_tray_icon_event<R: Runtime>(tray: &TrayIcon<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        show_main_window(tray.app_handle());
    }
}

/// Emits a playback command to the frontend
fn emit_playback_command<R: Runtime>(app: &AppHandle<R>, command: &str) {
    if let Err(e) = app.emit("playback-command", command) {
        tracing::error!("Failed to emit playback command: {}", e);
    }
}

/// Shows and focuses the main window
fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Hides the main window to tray instead of closing
pub fn minimize_to_tray(window: &Window) -> Result<(), tauri::Error> {
    window.hide()?;
    Ok(())
}

/// Tauri command to update playback state from frontend
#[tauri::command]
pub fn update_playback_state(
    app: AppHandle<Wry>,
    is_playing: bool,
    track_title: Option<String>,
    artist_name: Option<String>,
) -> Result<(), String> {
    let state = PlaybackState {
        is_playing,
        track_title,
        artist_name,
    };

    update_tray_menu(&app, &state).map_err(|e| e.to_string())?;

    // Update tooltip with current track
    if let Some(tray) = app.tray_by_id("main-tray") {
        let tooltip = if let (Some(title), Some(artist)) = (&state.track_title, &state.artist_name)
        {
            format!("Resonance\n{} - {}", title, artist)
        } else {
            "Resonance".to_string()
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_state_default() {
        let state = PlaybackState::default();
        assert!(!state.is_playing);
        assert!(state.track_title.is_none());
        assert!(state.artist_name.is_none());
    }

    #[test]
    fn test_playback_state_with_track() {
        let state = PlaybackState {
            is_playing: true,
            track_title: Some("Test Track".to_string()),
            artist_name: Some("Test Artist".to_string()),
        };
        assert!(state.is_playing);
        assert_eq!(state.track_title.as_deref(), Some("Test Track"));
        assert_eq!(state.artist_name.as_deref(), Some("Test Artist"));
    }
}
