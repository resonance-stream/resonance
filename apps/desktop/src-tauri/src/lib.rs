//! Resonance Desktop Application
//!
//! This is the Tauri 2.x desktop shell for the Resonance music streaming platform.
//! It wraps the web application and provides native desktop integration.

use tauri::Manager;

/// Runs the Tauri application.
///
/// This function sets up the Tauri app with all necessary plugins and configurations.
/// The web frontend is loaded from the bundled dist directory.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
