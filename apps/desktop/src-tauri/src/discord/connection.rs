//! Discord IPC Connection Management
//!
//! Handles the connection to Discord's local IPC socket for Rich Presence updates.
//! Uses the discord-rich-presence crate for the underlying IPC communication.

use discord_rich_presence::{activity::Activity, DiscordIpc, DiscordIpcClient};
use std::fmt;

use super::presence::RichPresence;

/// Error type for Discord connection operations
#[derive(Debug)]
pub enum ConnectionError {
    /// Failed to create the IPC client
    ClientCreation(String),
    /// Failed to connect to Discord
    Connection(String),
    /// Failed to set activity
    SetActivity(String),
    /// Failed to clear activity
    ClearActivity(String),
    /// Discord is not running
    NotRunning,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClientCreation(msg) => write!(f, "Failed to create Discord client: {}", msg),
            Self::Connection(msg) => write!(f, "Failed to connect to Discord: {}", msg),
            Self::SetActivity(msg) => write!(f, "Failed to set activity: {}", msg),
            Self::ClearActivity(msg) => write!(f, "Failed to clear activity: {}", msg),
            Self::NotRunning => write!(f, "Discord is not running"),
        }
    }
}

impl std::error::Error for ConnectionError {}

/// Manages the connection to Discord's IPC socket
pub struct DiscordConnection {
    client: DiscordIpcClient,
    connected: bool,
}

impl DiscordConnection {
    /// Creates a new Discord connection with the given application ID
    ///
    /// # Arguments
    /// * `app_id` - The Discord application ID (registered at discord.com/developers)
    ///
    /// # Returns
    /// A connected `DiscordConnection` or an error if connection fails
    pub fn new(app_id: &str) -> Result<Self, ConnectionError> {
        let mut client = DiscordIpcClient::new(app_id)
            .map_err(|e| ConnectionError::ClientCreation(e.to_string()))?;

        client
            .connect()
            .map_err(|e| ConnectionError::Connection(e.to_string()))?;

        tracing::debug!("Discord IPC connection established");

        Ok(Self {
            client,
            connected: true,
        })
    }

    /// Checks if the connection is currently active
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Sets the Discord Rich Presence activity
    ///
    /// # Arguments
    /// * `presence` - The rich presence data to display
    pub fn set_activity(&mut self, presence: RichPresence) -> Result<(), ConnectionError> {
        if !self.connected {
            return Err(ConnectionError::NotRunning);
        }

        let activity = presence.to_activity();

        self.client
            .set_activity(activity)
            .map_err(|e| ConnectionError::SetActivity(e.to_string()))?;

        tracing::debug!("Discord activity set: {}", presence.details);

        Ok(())
    }

    /// Clears the current Discord Rich Presence activity
    pub fn clear_activity(&mut self) -> Result<(), ConnectionError> {
        if !self.connected {
            return Err(ConnectionError::NotRunning);
        }

        // Set an empty activity to clear presence
        let empty_activity = Activity::new();

        self.client
            .set_activity(empty_activity)
            .map_err(|e| ConnectionError::ClearActivity(e.to_string()))?;

        tracing::debug!("Discord activity cleared");

        Ok(())
    }

    /// Reconnects to Discord if disconnected
    #[allow(dead_code)]
    pub fn reconnect(&mut self) -> Result<(), ConnectionError> {
        if self.connected {
            // Already connected
            return Ok(());
        }

        self.client
            .reconnect()
            .map_err(|e| ConnectionError::Connection(e.to_string()))?;

        self.connected = true;
        tracing::info!("Discord IPC reconnected");

        Ok(())
    }

    /// Closes the Discord IPC connection
    pub fn close(&mut self) -> Result<(), ConnectionError> {
        if !self.connected {
            return Ok(());
        }

        self.client
            .close()
            .map_err(|e| ConnectionError::Connection(e.to_string()))?;

        self.connected = false;
        tracing::debug!("Discord IPC connection closed");

        Ok(())
    }
}

impl Drop for DiscordConnection {
    fn drop(&mut self) {
        if self.connected {
            // Best effort close on drop
            let _ = self.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_error_display() {
        let err = ConnectionError::NotRunning;
        assert_eq!(err.to_string(), "Discord is not running");

        let err = ConnectionError::ClientCreation("test".to_string());
        assert!(err.to_string().contains("Failed to create Discord client"));

        let err = ConnectionError::Connection("timeout".to_string());
        assert!(err.to_string().contains("Failed to connect to Discord"));

        let err = ConnectionError::SetActivity("invalid".to_string());
        assert!(err.to_string().contains("Failed to set activity"));

        let err = ConnectionError::ClearActivity("error".to_string());
        assert!(err.to_string().contains("Failed to clear activity"));
    }
}
