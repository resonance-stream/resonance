-- Resonance: Add pre-computed audio features vector for fast acoustic similarity search
-- Migration: 20250101000017_add_audio_features_vector
--
-- This migration addresses the P1 acoustic similarity bottleneck by:
-- 1. Adding a pre-computed 5-dimensional vector for audio features
-- 2. Creating an HNSW index for O(log n) similarity search
-- 3. Backfilling the vector from existing audio_features JSONB data
--
-- Vector dimensions (all normalized to 0-1 range):
-- [0] energy (already 0-1)
-- [1] loudness_norm ((loudness + 60) / 60, maps -60..0 dB to 0..1)
-- [2] valence (already 0-1)
-- [3] danceability (already 0-1)
-- [4] bpm_norm (bpm / 200, normalizes typical 60-180 BPM range)

-- Add the audio features vector column
ALTER TABLE track_embeddings
ADD COLUMN audio_features_vector vector(5);

COMMENT ON COLUMN track_embeddings.audio_features_vector IS 'Pre-computed normalized audio features vector [energy, loudness_norm, valence, danceability, bpm_norm] for fast similarity search';

-- Create HNSW index for fast Euclidean distance similarity search
-- Using vector_l2_ops (Euclidean distance) to match existing similarity calculations
-- Parameters match existing HNSW indexes: m=16, ef_construction=64
CREATE INDEX idx_track_embeddings_audio_features ON track_embeddings
    USING hnsw(audio_features_vector vector_l2_ops)
    WITH (m = 16, ef_construction = 64);

-- Backfill the audio_features_vector from existing tracks.audio_features JSONB
-- Only populate for tracks that have the required audio features extracted
UPDATE track_embeddings te
SET audio_features_vector = (
    SELECT CASE
        WHEN t.audio_features->>'energy' IS NOT NULL
         AND t.audio_features->>'loudness' IS NOT NULL
         AND t.audio_features->>'valence' IS NOT NULL
         AND t.audio_features->>'danceability' IS NOT NULL
         AND t.audio_features->>'bpm' IS NOT NULL
        THEN (
            '[' ||
            COALESCE((t.audio_features->>'energy')::float, 0.5)::text || ',' ||
            COALESCE(((t.audio_features->>'loudness')::float + 60) / 60, 0.5)::text || ',' ||
            COALESCE((t.audio_features->>'valence')::float, 0.5)::text || ',' ||
            COALESCE((t.audio_features->>'danceability')::float, 0.5)::text || ',' ||
            COALESCE((t.audio_features->>'bpm')::float / 200, 0.5)::text ||
            ']'
        )::vector
        ELSE NULL
    END
    FROM tracks t
    WHERE t.id = te.track_id
)
WHERE EXISTS (
    SELECT 1 FROM tracks t
    WHERE t.id = te.track_id
      AND t.audio_features->>'energy' IS NOT NULL
);
