//! Autostart Management
//!
//! Manages application autostart settings to launch Resonance on system boot.
//! Uses tauri-plugin-autostart for cross-platform autostart support.

use tauri::{AppHandle, Wry};
use tauri_plugin_autostart::ManagerExt;

/// Enables autostart - application will launch on system boot
#[tauri::command]
pub fn enable_autostart(app: AppHandle<Wry>) -> Result<(), String> {
    let autostart_manager = app.autolaunch();

    autostart_manager
        .enable()
        .map_err(|e| format!("Failed to enable autostart: {}", e))?;

    tracing::info!("Autostart enabled");
    Ok(())
}

/// Disables autostart - application will not launch on system boot
#[tauri::command]
pub fn disable_autostart(app: AppHandle<Wry>) -> Result<(), String> {
    let autostart_manager = app.autolaunch();

    autostart_manager
        .disable()
        .map_err(|e| format!("Failed to disable autostart: {}", e))?;

    tracing::info!("Autostart disabled");
    Ok(())
}

/// Checks if autostart is currently enabled
#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle<Wry>) -> Result<bool, String> {
    let autostart_manager = app.autolaunch();

    autostart_manager
        .is_enabled()
        .map_err(|e| format!("Failed to check autostart status: {}", e))
}

/// Toggles autostart state
#[tauri::command]
pub fn toggle_autostart(app: AppHandle<Wry>) -> Result<bool, String> {
    let autostart_manager = app.autolaunch();

    let is_enabled = autostart_manager
        .is_enabled()
        .map_err(|e| format!("Failed to check autostart status: {}", e))?;

    if is_enabled {
        autostart_manager
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {}", e))?;
        tracing::info!("Autostart toggled off");
        Ok(false)
    } else {
        autostart_manager
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
        tracing::info!("Autostart toggled on");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    // Autostart tests require a running Tauri app context,
    // so unit tests are limited to basic compile checks.
    // Integration tests should be added in a separate test harness.

    #[test]
    fn test_module_compiles() {
        // Ensures the module compiles correctly
        assert!(true);
    }
}
