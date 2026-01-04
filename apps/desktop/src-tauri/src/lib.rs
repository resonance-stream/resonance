//! Resonance Desktop Application
//!
//! This is the Tauri 2.x desktop shell for the Resonance music streaming platform.
//! It wraps the web application and provides native desktop integration including:
//! - System tray with dynamic playback controls
//! - Global media key shortcuts
//! - Minimize-to-tray functionality

mod media_keys;
mod tray;

use tauri::{Manager, WindowEvent};

/// Runs the Tauri application.
///
/// This function sets up the Tauri app with all necessary plugins and configurations.
/// The web frontend is loaded from the bundled dist directory.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![tray::update_playback_state])
        .setup(|app| {
            // Create system tray
            if let Err(e) = tray::create_tray(app.handle()) {
                tracing::error!("Failed to create system tray: {}", e);
            }

            // Register global media key shortcuts
            if let Err(e) = media_keys::register_media_keys(app.handle()) {
                tracing::error!("Failed to register media keys: {}", e);
            }

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Handle window close to minimize to tray instead
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide window to tray instead of closing
                if let Err(e) = tray::minimize_to_tray(window) {
                    tracing::error!("Failed to minimize to tray: {}", e);
                } else {
                    // Prevent the window from closing
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
