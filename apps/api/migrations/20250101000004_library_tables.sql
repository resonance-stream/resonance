-- Resonance: Library Tables (Artists, Albums, Tracks)
-- Migration: 20250101000004_library_tables

CREATE TABLE artists (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(500) NOT NULL,
    sort_name VARCHAR(500),
    mbid UUID UNIQUE,
    lidarr_id INTEGER UNIQUE,
    biography TEXT,
    image_url VARCHAR(500),
    genres TEXT[] NOT NULL DEFAULT '{}',
    external_urls JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE albums (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(500) NOT NULL,
    artist_id UUID NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    mbid UUID UNIQUE,
    lidarr_id INTEGER UNIQUE,
    release_date DATE,
    album_type album_type NOT NULL DEFAULT 'album',
    genres TEXT[] NOT NULL DEFAULT '{}',
    total_tracks INTEGER,
    total_duration_ms BIGINT,
    cover_art_path VARCHAR(500),
    cover_art_colors JSONB NOT NULL DEFAULT '{
        "primary": null,
        "secondary": null,
        "accent": null,
        "vibrant": null,
        "muted": null
    }'::jsonb,
    external_urls JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tracks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(500) NOT NULL,
    album_id UUID REFERENCES albums(id) ON DELETE SET NULL,
    artist_id UUID NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    mbid UUID UNIQUE,

    -- File information
    file_path VARCHAR(1000) UNIQUE NOT NULL,
    file_size BIGINT NOT NULL,
    file_format audio_format NOT NULL,
    file_hash VARCHAR(64),

    -- Audio properties
    duration_ms INTEGER NOT NULL,
    bit_rate INTEGER,
    sample_rate INTEGER,
    channels SMALLINT,
    bit_depth SMALLINT,

    -- Track metadata
    track_number SMALLINT,
    disc_number SMALLINT DEFAULT 1,
    genres TEXT[] NOT NULL DEFAULT '{}',
    explicit BOOLEAN NOT NULL DEFAULT false,
    lyrics TEXT,
    synced_lyrics JSONB,

    -- Audio features (extracted by analysis)
    audio_features JSONB NOT NULL DEFAULT '{
        "bpm": null,
        "key": null,
        "mode": null,
        "loudness": null,
        "energy": null,
        "danceability": null,
        "valence": null,
        "acousticness": null,
        "instrumentalness": null,
        "speechiness": null
    }'::jsonb,

    -- AI-generated data
    ai_mood TEXT[] NOT NULL DEFAULT '{}',
    ai_tags TEXT[] NOT NULL DEFAULT '{}',
    ai_description TEXT,

    -- Playback statistics
    play_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    last_played_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE track_embeddings (
    track_id UUID PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
    title_embedding vector(768),
    description_embedding vector(768),
    audio_embedding vector(128),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE artists IS 'Artist metadata with MusicBrainz and Lidarr integration';
COMMENT ON TABLE albums IS 'Album metadata with cover art color extraction';
COMMENT ON TABLE tracks IS 'Track metadata, file info, audio features, and AI tags';
COMMENT ON TABLE track_embeddings IS 'Vector embeddings for semantic search and recommendations';
COMMENT ON COLUMN tracks.audio_features IS 'Audio features extracted by analysis (BPM, key, energy, etc.)';
COMMENT ON COLUMN tracks.ai_mood IS 'AI-detected mood tags (e.g., happy, melancholic, energetic)';
COMMENT ON COLUMN tracks.ai_tags IS 'AI-generated descriptive tags';
COMMENT ON COLUMN albums.cover_art_colors IS 'Extracted color palette for visualizer';
COMMENT ON COLUMN track_embeddings.title_embedding IS 'Text embedding of track title (768-dim)';
COMMENT ON COLUMN track_embeddings.description_embedding IS 'Text embedding of AI description (768-dim)';
COMMENT ON COLUMN track_embeddings.audio_embedding IS 'Audio feature embedding (128-dim)';
