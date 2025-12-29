-- Resonance: Add unique indexes for artist and album upsert operations
-- Migration: 20250101000011_add_artist_album_unique_indexes
-- These indexes enable race-condition-free upsert patterns in library_scan.rs

-- Unique index on lowercase artist name for case-insensitive deduplication
-- This allows ON CONFLICT ((LOWER(name))) in the upsert query
CREATE UNIQUE INDEX IF NOT EXISTS idx_artists_name_lower
    ON artists ((LOWER(name)));

-- Unique index on lowercase album title + artist_id for case-insensitive deduplication
-- This allows ON CONFLICT ((LOWER(title)), artist_id) in the upsert query
CREATE UNIQUE INDEX IF NOT EXISTS idx_albums_title_artist_lower
    ON albums ((LOWER(title)), artist_id);

COMMENT ON INDEX idx_artists_name_lower IS 'Case-insensitive unique constraint for artist names';
COMMENT ON INDEX idx_albums_title_artist_lower IS 'Case-insensitive unique constraint for album titles per artist';
