//! Deep Link Handler
//!
//! Handles the resonance:// protocol for deep linking into the application.
//! Supports URLs like:
//! - resonance://play/track/<track_id>
//! - resonance://play/album/<album_id>
//! - resonance://play/playlist/<playlist_id>
//! - resonance://play/artist/<artist_id>
//! - resonance://search?q=<query>
//! - resonance://settings
//! - resonance://library

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};
use url::Url;

/// Deep link event payload sent to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DeepLinkAction {
    /// Play a specific track
    PlayTrack { track_id: String },
    /// Play an album
    PlayAlbum { album_id: String },
    /// Play a playlist
    PlayPlaylist { playlist_id: String },
    /// Play an artist's top tracks
    PlayArtist { artist_id: String },
    /// Perform a search
    Search { query: String },
    /// Navigate to settings
    OpenSettings,
    /// Navigate to library
    OpenLibrary,
    /// Navigate to a specific path
    Navigate { path: String },
}

/// Parses and handles a deep link URL
pub fn handle_deep_link<R: Runtime>(app: &AppHandle<R>, urls: Vec<String>) {
    for url_str in urls {
        tracing::info!("Handling deep link: {}", url_str);

        match parse_deep_link(&url_str) {
            Ok(action) => {
                if let Err(e) = app.emit("deep-link", &action) {
                    tracing::error!("Failed to emit deep link event: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Invalid deep link '{}': {}", url_str, e);
            }
        }
    }
}

/// Parses a deep link URL into a DeepLinkAction
fn parse_deep_link(url_str: &str) -> Result<DeepLinkAction, String> {
    let url = Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;

    // Ensure the scheme is resonance://
    if url.scheme() != "resonance" {
        return Err(format!("Unsupported scheme: {}", url.scheme()));
    }

    let host = url.host_str().unwrap_or("");
    let path_segments: Vec<&str> = url.path_segments().map_or(vec![], |s| s.collect());

    match host {
        "play" => parse_play_action(&path_segments),
        "search" => {
            let query = url
                .query_pairs()
                .find(|(k, _)| k == "q")
                .map(|(_, v)| v.to_string())
                .ok_or_else(|| "Missing search query parameter 'q'".to_string())?;
            Ok(DeepLinkAction::Search { query })
        }
        "settings" => Ok(DeepLinkAction::OpenSettings),
        "library" => Ok(DeepLinkAction::OpenLibrary),
        "" => {
            // Handle resonance:///path style URLs
            let path = url.path();
            if !path.is_empty() && path != "/" {
                Ok(DeepLinkAction::Navigate {
                    path: path.to_string(),
                })
            } else {
                Err("Empty deep link path".to_string())
            }
        }
        _ => {
            // Treat unknown hosts as navigation paths
            let full_path = if path_segments.is_empty() {
                format!("/{}", host)
            } else {
                format!("/{}/{}", host, path_segments.join("/"))
            };
            Ok(DeepLinkAction::Navigate { path: full_path })
        }
    }
}

/// Parses play-related deep link actions
fn parse_play_action(segments: &[&str]) -> Result<DeepLinkAction, String> {
    if segments.len() < 2 {
        return Err("Play action requires type and ID".to_string());
    }

    let content_type = segments[0];
    let id = segments[1].to_string();

    match content_type {
        "track" => Ok(DeepLinkAction::PlayTrack { track_id: id }),
        "album" => Ok(DeepLinkAction::PlayAlbum { album_id: id }),
        "playlist" => Ok(DeepLinkAction::PlayPlaylist { playlist_id: id }),
        "artist" => Ok(DeepLinkAction::PlayArtist { artist_id: id }),
        _ => Err(format!("Unknown play content type: {}", content_type)),
    }
}

/// Gets the registered deep link schemes
#[tauri::command]
pub fn get_deep_link_scheme() -> String {
    "resonance".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_play_track() {
        let action = parse_deep_link("resonance://play/track/123").unwrap();
        match action {
            DeepLinkAction::PlayTrack { track_id } => assert_eq!(track_id, "123"),
            _ => panic!("Expected PlayTrack action"),
        }
    }

    #[test]
    fn test_parse_play_album() {
        let action = parse_deep_link("resonance://play/album/456").unwrap();
        match action {
            DeepLinkAction::PlayAlbum { album_id } => assert_eq!(album_id, "456"),
            _ => panic!("Expected PlayAlbum action"),
        }
    }

    #[test]
    fn test_parse_play_playlist() {
        let action = parse_deep_link("resonance://play/playlist/789").unwrap();
        match action {
            DeepLinkAction::PlayPlaylist { playlist_id } => assert_eq!(playlist_id, "789"),
            _ => panic!("Expected PlayPlaylist action"),
        }
    }

    #[test]
    fn test_parse_play_artist() {
        let action = parse_deep_link("resonance://play/artist/abc").unwrap();
        match action {
            DeepLinkAction::PlayArtist { artist_id } => assert_eq!(artist_id, "abc"),
            _ => panic!("Expected PlayArtist action"),
        }
    }

    #[test]
    fn test_parse_search() {
        let action = parse_deep_link("resonance://search?q=test%20query").unwrap();
        match action {
            DeepLinkAction::Search { query } => assert_eq!(query, "test query"),
            _ => panic!("Expected Search action"),
        }
    }

    #[test]
    fn test_parse_settings() {
        let action = parse_deep_link("resonance://settings").unwrap();
        match action {
            DeepLinkAction::OpenSettings => {}
            _ => panic!("Expected OpenSettings action"),
        }
    }

    #[test]
    fn test_parse_library() {
        let action = parse_deep_link("resonance://library").unwrap();
        match action {
            DeepLinkAction::OpenLibrary => {}
            _ => panic!("Expected OpenLibrary action"),
        }
    }

    #[test]
    fn test_parse_navigate() {
        let action = parse_deep_link("resonance://queue").unwrap();
        match action {
            DeepLinkAction::Navigate { path } => assert_eq!(path, "/queue"),
            _ => panic!("Expected Navigate action"),
        }
    }

    #[test]
    fn test_invalid_scheme() {
        let result = parse_deep_link("http://example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_search_query() {
        let result = parse_deep_link("resonance://search");
        assert!(result.is_err());
    }

    #[test]
    fn test_incomplete_play_action() {
        let result = parse_deep_link("resonance://play/track");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_deep_link_scheme() {
        assert_eq!(get_deep_link_scheme(), "resonance");
    }
}
