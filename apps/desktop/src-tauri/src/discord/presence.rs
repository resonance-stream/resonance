//! Discord Rich Presence Data Types
//!
//! Defines the data structures for Discord Rich Presence, including track information
//! and playback state that gets displayed in the user's Discord profile.

use discord_rich_presence::activity::{Activity, Assets, Timestamps};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Payload received from the frontend for presence updates
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresencePayload {
    /// Current track title
    pub track_title: String,
    /// Artist name
    pub artist_name: String,
    /// Album name (optional)
    pub album_name: Option<String>,
    /// Track duration in seconds
    pub duration_secs: Option<u64>,
    /// Current playback position in seconds
    pub elapsed_secs: Option<u64>,
    /// Whether playback is active
    pub is_playing: bool,
    /// URL to album artwork (optional)
    pub album_art_url: Option<String>,
}

/// Rich presence data ready to be sent to Discord
#[derive(Debug, Clone)]
pub struct RichPresence {
    /// Main text line (track title)
    pub details: String,
    /// Secondary text line (artist - album)
    pub state: String,
    /// Large image key or URL
    pub large_image: Option<String>,
    /// Large image tooltip text
    pub large_image_text: Option<String>,
    /// Small image key (play/pause indicator)
    pub small_image: Option<String>,
    /// Small image tooltip text
    pub small_image_text: Option<String>,
    /// Start timestamp for elapsed time
    pub start_timestamp: Option<i64>,
    /// End timestamp for remaining time
    pub end_timestamp: Option<i64>,
}

impl RichPresence {
    /// Creates a RichPresence from a PresencePayload
    pub fn from_payload(payload: &PresencePayload) -> Self {
        let details = payload.track_title.clone();

        // Format state as "Artist • Album" or just "Artist"
        let state = match &payload.album_name {
            Some(album) if !album.is_empty() => {
                format!("{} • {}", payload.artist_name, album)
            }
            _ => payload.artist_name.clone(),
        };

        // Calculate timestamps for playback progress
        let (start_timestamp, end_timestamp) = if payload.is_playing {
            Self::calculate_timestamps(payload.elapsed_secs, payload.duration_secs)
        } else {
            (None, None)
        };

        // Determine small image based on playback state
        let (small_image, small_image_text) = if payload.is_playing {
            (Some("play".to_string()), Some("Playing".to_string()))
        } else {
            (Some("pause".to_string()), Some("Paused".to_string()))
        };

        // Use album art URL if provided, otherwise use app icon
        let large_image = payload
            .album_art_url
            .clone()
            .or_else(|| Some("resonance_icon".to_string()));

        let large_image_text = payload
            .album_name
            .clone()
            .or_else(|| Some("Resonance".to_string()));

        Self {
            details,
            state,
            large_image,
            large_image_text,
            small_image,
            small_image_text,
            start_timestamp,
            end_timestamp,
        }
    }

    /// Calculates start and end timestamps for Discord's time display
    fn calculate_timestamps(
        elapsed_secs: Option<u64>,
        duration_secs: Option<u64>,
    ) -> (Option<i64>, Option<i64>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        match (elapsed_secs, duration_secs) {
            (Some(elapsed), Some(duration)) => {
                let start = now - elapsed as i64;
                let end = start + duration as i64;
                (Some(start), Some(end))
            }
            (Some(elapsed), None) => {
                let start = now - elapsed as i64;
                (Some(start), None)
            }
            _ => (None, None),
        }
    }

    /// Converts RichPresence to a discord_rich_presence Activity
    pub fn to_activity(&self) -> Activity<'_> {
        let mut activity = Activity::new()
            .details(&self.details)
            .state(&self.state);

        // Add assets (images)
        let mut assets = Assets::new();

        if let Some(ref large_img) = self.large_image {
            assets = assets.large_image(large_img);
        }
        if let Some(ref large_text) = self.large_image_text {
            assets = assets.large_text(large_text);
        }
        if let Some(ref small_img) = self.small_image {
            assets = assets.small_image(small_img);
        }
        if let Some(ref small_text) = self.small_image_text {
            assets = assets.small_text(small_text);
        }

        activity = activity.assets(assets);

        // Add timestamps if available
        if self.start_timestamp.is_some() || self.end_timestamp.is_some() {
            let mut timestamps = Timestamps::new();

            if let Some(start) = self.start_timestamp {
                timestamps = timestamps.start(start);
            }
            if let Some(end) = self.end_timestamp {
                timestamps = timestamps.end(end);
            }

            activity = activity.timestamps(timestamps);
        }

        activity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presence_payload_default() {
        let payload = PresencePayload::default();
        assert!(payload.track_title.is_empty());
        assert!(payload.artist_name.is_empty());
        assert!(payload.album_name.is_none());
        assert!(!payload.is_playing);
    }

    #[test]
    fn test_rich_presence_from_payload_basic() {
        let payload = PresencePayload {
            track_title: "Test Track".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: None,
            duration_secs: None,
            elapsed_secs: None,
            is_playing: false,
            album_art_url: None,
        };

        let presence = RichPresence::from_payload(&payload);
        assert_eq!(presence.details, "Test Track");
        assert_eq!(presence.state, "Test Artist");
        assert_eq!(presence.small_image, Some("pause".to_string()));
        assert_eq!(presence.small_image_text, Some("Paused".to_string()));
    }

    #[test]
    fn test_rich_presence_from_payload_with_album() {
        let payload = PresencePayload {
            track_title: "Test Track".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: Some("Test Album".to_string()),
            duration_secs: None,
            elapsed_secs: None,
            is_playing: true,
            album_art_url: None,
        };

        let presence = RichPresence::from_payload(&payload);
        assert_eq!(presence.state, "Test Artist • Test Album");
        assert_eq!(presence.small_image, Some("play".to_string()));
        assert_eq!(presence.small_image_text, Some("Playing".to_string()));
    }

    #[test]
    fn test_rich_presence_with_timestamps() {
        let payload = PresencePayload {
            track_title: "Test Track".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: None,
            duration_secs: Some(180),
            elapsed_secs: Some(60),
            is_playing: true,
            album_art_url: None,
        };

        let presence = RichPresence::from_payload(&payload);
        assert!(presence.start_timestamp.is_some());
        assert!(presence.end_timestamp.is_some());

        // End should be 120 seconds (duration - elapsed) after start
        let start = presence.start_timestamp.unwrap();
        let end = presence.end_timestamp.unwrap();
        assert_eq!(end - start, 180); // Duration
    }

    #[test]
    fn test_rich_presence_paused_no_timestamps() {
        let payload = PresencePayload {
            track_title: "Test Track".to_string(),
            artist_name: "Test Artist".to_string(),
            album_name: None,
            duration_secs: Some(180),
            elapsed_secs: Some(60),
            is_playing: false,
            album_art_url: None,
        };

        let presence = RichPresence::from_payload(&payload);
        // When paused, no timestamps should be set
        assert!(presence.start_timestamp.is_none());
        assert!(presence.end_timestamp.is_none());
    }

    #[test]
    fn test_calculate_timestamps_both_present() {
        let (start, end) = RichPresence::calculate_timestamps(Some(30), Some(120));
        assert!(start.is_some());
        assert!(end.is_some());

        let start_ts = start.unwrap();
        let end_ts = end.unwrap();
        assert_eq!(end_ts - start_ts, 120);
    }

    #[test]
    fn test_calculate_timestamps_elapsed_only() {
        let (start, end) = RichPresence::calculate_timestamps(Some(30), None);
        assert!(start.is_some());
        assert!(end.is_none());
    }

    #[test]
    fn test_calculate_timestamps_none() {
        let (start, end) = RichPresence::calculate_timestamps(None, None);
        assert!(start.is_none());
        assert!(end.is_none());
    }
}
