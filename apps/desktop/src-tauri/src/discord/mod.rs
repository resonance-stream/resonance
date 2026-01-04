//! Discord Rich Presence Integration
//!
//! Provides Discord Rich Presence functionality for displaying current playback
//! state in Discord. Includes connection management and presence updates.

mod connection;
mod presence;

pub use connection::DiscordConnection;
pub use presence::{PresencePayload, RichPresence};

use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Wry};

/// Discord Application ID for Resonance
/// This is a placeholder - in production, register at https://discord.com/developers/applications
pub const DISCORD_APP_ID: &str = "1234567890123456789";

/// Global Discord connection state
pub type DiscordState = Arc<Mutex<Option<DiscordConnection>>>;

/// Initialize Discord RPC state
pub fn init_discord_state() -> DiscordState {
    Arc::new(Mutex::new(None))
}

/// Set the Discord rich presence with track information
#[tauri::command]
pub fn set_presence(app: AppHandle<Wry>, payload: PresencePayload) -> Result<(), String> {
    let state = app.state::<DiscordState>();
    let mut guard = state.lock();

    // Initialize connection if not already connected
    if guard.is_none() {
        match DiscordConnection::new(DISCORD_APP_ID) {
            Ok(conn) => {
                *guard = Some(conn);
                tracing::info!("Discord RPC connected");
            }
            Err(e) => {
                tracing::warn!("Failed to connect to Discord: {}", e);
                return Err(format!("Failed to connect to Discord: {}", e));
            }
        }
    }

    if let Some(ref mut conn) = *guard {
        let presence = RichPresence::from_payload(&payload);
        conn.set_activity(presence).map_err(|e| {
            tracing::error!("Failed to set Discord presence: {}", e);
            e.to_string()
        })?;
    }

    Ok(())
}

/// Clear the Discord rich presence
#[tauri::command]
pub fn clear_presence(app: AppHandle<Wry>) -> Result<(), String> {
    let state = app.state::<DiscordState>();
    let mut guard = state.lock();

    if let Some(ref mut conn) = *guard {
        conn.clear_activity().map_err(|e| {
            tracing::error!("Failed to clear Discord presence: {}", e);
            e.to_string()
        })?;
        tracing::debug!("Discord presence cleared");
    }

    Ok(())
}

/// Disconnect from Discord RPC
#[tauri::command]
pub fn disconnect_discord(app: AppHandle<Wry>) -> Result<(), String> {
    let state = app.state::<DiscordState>();
    let mut guard = state.lock();

    if guard.take().is_some() {
        tracing::info!("Discord RPC disconnected");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_discord_state() {
        let state = init_discord_state();
        assert!(state.lock().is_none());
    }

    #[test]
    fn test_discord_app_id_format() {
        // Discord app IDs are numeric strings
        assert!(DISCORD_APP_ID.chars().all(|c| c.is_ascii_digit()));
    }
}
