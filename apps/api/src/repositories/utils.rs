//! Shared utility functions for repositories
//!
//! This module provides common functions used across repositories.

/// Escape special characters in ILIKE patterns to prevent pattern injection.
///
/// ILIKE uses `%` for any sequence and `_` for single character wildcards.
/// If user input contains these characters, they must be escaped to match literally.
///
/// # Example
/// ```
/// use resonance_api::repositories::utils::escape_ilike;
///
/// let input = "100%";
/// let escaped = escape_ilike(input);
/// assert_eq!(escaped, r"100\%");
/// ```
pub fn escape_ilike(pattern: &str) -> String {
    pattern
        .replace('\\', r"\\")
        .replace('%', r"\%")
        .replace('_', r"\_")
}

// ============================================================================
// SQL Column Constants
//
// These constants define the SELECT column lists for each entity type,
// reducing duplication and ensuring consistency across queries.
// ============================================================================

/// SQL columns for artist queries
pub const ARTIST_COLUMNS: &str = r#"
    id, name, sort_name, mbid, lidarr_id,
    biography, image_url, genres,
    external_urls, metadata,
    created_at, updated_at
"#;

/// SQL columns for album queries
pub const ALBUM_COLUMNS: &str = r#"
    id, title, artist_id, mbid, lidarr_id,
    release_date, album_type, genres,
    total_tracks, total_duration_ms,
    cover_art_path, cover_art_colors,
    external_urls, metadata,
    created_at, updated_at
"#;

/// SQL columns for track queries
pub const TRACK_COLUMNS: &str = r#"
    id, title, album_id, artist_id, mbid,
    file_path, file_size, file_format, file_hash,
    duration_ms, bit_rate, sample_rate, channels, bit_depth,
    track_number, disc_number, genres, explicit,
    lyrics, synced_lyrics, audio_features,
    ai_mood, ai_tags, ai_description,
    play_count, skip_count, last_played_at,
    created_at, updated_at
"#;

/// SQL columns for playlist queries
pub const PLAYLIST_COLUMNS: &str = r#"
    id, user_id, name, description, image_url,
    is_public, is_collaborative, playlist_type,
    smart_rules, track_count, total_duration_ms,
    created_at, updated_at
"#;

/// SQL columns for user queries
#[allow(dead_code)]
pub const USER_COLUMNS: &str = r#"
    id, email, password_hash, display_name, avatar_url,
    role, preferences, listenbrainz_token, discord_user_id,
    email_verified, last_seen_at, created_at, updated_at
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_ilike_no_special_chars() {
        assert_eq!(escape_ilike("hello world"), "hello world");
    }

    #[test]
    fn test_escape_ilike_percent() {
        assert_eq!(escape_ilike("100% complete"), r"100\% complete");
    }

    #[test]
    fn test_escape_ilike_underscore() {
        assert_eq!(escape_ilike("test_case"), r"test\_case");
    }

    #[test]
    fn test_escape_ilike_backslash() {
        assert_eq!(escape_ilike(r"path\to\file"), r"path\\to\\file");
    }

    #[test]
    fn test_escape_ilike_all_special() {
        assert_eq!(escape_ilike(r"100%_\test"), r"100\%\_\\test");
    }

    #[test]
    fn test_escape_ilike_empty() {
        assert_eq!(escape_ilike(""), "");
    }
}
