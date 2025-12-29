-- Resonance: Add track availability tracking
-- Migration: 20250101000010_add_track_availability
-- This column tracks whether the audio file still exists on disk

ALTER TABLE tracks ADD COLUMN IF NOT EXISTS is_available BOOLEAN NOT NULL DEFAULT true;

-- Index for filtering available tracks
CREATE INDEX IF NOT EXISTS idx_tracks_is_available ON tracks(is_available) WHERE is_available = true;

COMMENT ON COLUMN tracks.is_available IS 'Whether the audio file exists on disk (false if deleted)';
