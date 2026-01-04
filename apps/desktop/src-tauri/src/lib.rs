//! Resonance Desktop Application
//!
//! This is the Tauri 2.x desktop shell for the Resonance music streaming platform.
//! It wraps the web application and provides native desktop integration including:
//! - System tray with dynamic playback controls
//! - Global media key shortcuts
//! - Minimize-to-tray functionality
//! - Discord Rich Presence integration
//! - Native notifications for track changes
//! - Autostart on system boot
//! - Deep linking (resonance:// protocol)
//! - Automatic updates

mod autostart;
mod deep_link;
mod discord;
mod media_keys;
mod notifications;
mod tray;
mod updater;

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_deep_link::DeepLinkExt;

/// Runs the Tauri application.
///
/// This function sets up the Tauri app with all necessary plugins and configurations.
/// The web frontend is loaded from the bundled dist directory.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(discord::init_discord_state())
        .invoke_handler(tauri::generate_handler![
            // Tray commands
            tray::update_playback_state,
            // Discord commands
            discord::set_presence,
            discord::clear_presence,
            discord::disconnect_discord,
            // Notification commands
            notifications::show_track_notification,
            notifications::show_notification,
            notifications::check_notification_permission,
            notifications::request_notification_permission,
            // Autostart commands
            autostart::enable_autostart,
            autostart::disable_autostart,
            autostart::is_autostart_enabled,
            autostart::toggle_autostart,
            // Deep link commands
            deep_link::get_deep_link_scheme,
            // Updater commands
            updater::check_for_updates,
            updater::install_update,
            updater::get_current_version
        ])
        .setup(|app| {
            // Create system tray
            if let Err(e) = tray::create_tray(app.handle()) {
                tracing::error!("Failed to create system tray: {}", e);
            }

            // Register global media key shortcuts
            if let Err(e) = media_keys::register_media_keys(app.handle()) {
                tracing::error!("Failed to register media keys: {}", e);
            }

            // Register deep link handler
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                deep_link::handle_deep_link(&handle, event.urls().iter().map(|u| u.to_string()).collect());
            });

            // Check for updates on startup (non-blocking)
            #[cfg(not(debug_assertions))]
            {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // Delay update check by 5 seconds to let the app initialize
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    if let Err(e) = check_startup_update(&app_handle).await {
                        tracing::debug!("Startup update check skipped: {}", e);
                    }
                });
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

/// Checks for updates on startup and logs the result
#[cfg(not(debug_assertions))]
async fn check_startup_update(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => {
            tracing::info!(
                "Update available: {} (current: {})",
                update.version,
                app.package_info().version
            );
            // Emit event to frontend to show update notification
            if let Err(e) = app.emit("update-available", &update.version) {
                tracing::error!("Failed to emit update event: {}", e);
            }
            Ok(())
        }
        Ok(None) => {
            tracing::debug!("No update available");
            Ok(())
        }
        Err(e) => Err(format!("Update check failed: {}", e)),
    }
}
