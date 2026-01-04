-- Resonance: Add Unique Index for Discover Weekly Playlists
-- Migration: 20250101000018_add_discover_playlist_unique_index
--
-- This migration adds a partial unique index to prevent TOCTOU race conditions
-- when creating Discover Weekly playlists. The SELECT-then-INSERT pattern in
-- weekly_playlist.rs could theoretically create duplicate playlists under
-- concurrent execution. This index enables safe INSERT ON CONFLICT upserts.

-- Create partial unique index for discover playlists
-- This ensures only one "Discover Weekly" playlist per user
CREATE UNIQUE INDEX idx_playlists_user_discover_weekly
ON playlists (user_id, name)
WHERE playlist_type = 'discover';

COMMENT ON INDEX idx_playlists_user_discover_weekly IS
    'Ensures uniqueness of discover playlists per user/name combination, enabling atomic upserts';
