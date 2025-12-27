-- Resonance: Additional Tables (Activity, Presets, Sync, Downloads, Cache)
-- Migration: 20250101000006_additional_tables

CREATE TABLE listening_activity (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id UUID REFERENCES sessions(id) ON DELETE SET NULL,
    track_id UUID REFERENCES tracks(id) ON DELETE SET NULL,
    is_playing BOOLEAN NOT NULL DEFAULT false,
    progress_ms INTEGER NOT NULL DEFAULT 0,
    volume_percent SMALLINT DEFAULT 100,
    shuffle_enabled BOOLEAN NOT NULL DEFAULT false,
    repeat_mode VARCHAR(20) NOT NULL DEFAULT 'off',
    queue JSONB NOT NULL DEFAULT '[]'::jsonb,
    context_type context_type,
    context_id UUID,
    device_name VARCHAR(100),
    device_type VARCHAR(50),
    started_at TIMESTAMPTZ,
    last_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id, session_id)
);

CREATE TABLE equalizer_presets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT false,
    bands JSONB NOT NULL DEFAULT '{
        "32": 0,
        "64": 0,
        "125": 0,
        "250": 0,
        "500": 0,
        "1000": 0,
        "2000": 0,
        "4000": 0,
        "8000": 0,
        "16000": 0
    }'::jsonb,
    preamp SMALLINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE lidarr_sync_state (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    status sync_status NOT NULL DEFAULT 'idle',
    last_sync_at TIMESTAMPTZ,
    last_sync_duration_ms INTEGER,
    last_error TEXT,
    total_artists INTEGER NOT NULL DEFAULT 0,
    total_albums INTEGER NOT NULL DEFAULT 0,
    total_tracks INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE pending_downloads (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    lidarr_album_id INTEGER NOT NULL,
    artist_name VARCHAR(500) NOT NULL,
    album_name VARCHAR(500) NOT NULL,
    status download_status NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE recommendation_cache (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recommendation_type VARCHAR(50) NOT NULL,
    track_ids UUID[] NOT NULL,
    seed_track_ids UUID[],
    seed_artist_ids UUID[],
    score FLOAT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id, recommendation_type)
);

COMMENT ON TABLE listening_activity IS 'Real-time playback state for cross-device sync';
COMMENT ON TABLE equalizer_presets IS 'User and system equalizer presets';
COMMENT ON TABLE lidarr_sync_state IS 'Lidarr synchronization status';
COMMENT ON TABLE pending_downloads IS 'Lidarr download queue';
COMMENT ON TABLE recommendation_cache IS 'Cached recommendations per user';
