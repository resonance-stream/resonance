-- Resonance: User Data Tables (Playlists, History, Library)
-- Migration: 20250101000005_user_data_tables

CREATE TABLE playlists (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    image_url VARCHAR(500),
    is_public BOOLEAN NOT NULL DEFAULT false,
    is_collaborative BOOLEAN NOT NULL DEFAULT false,
    playlist_type playlist_type NOT NULL DEFAULT 'manual',
    smart_rules JSONB,
    track_count INTEGER NOT NULL DEFAULT 0,
    total_duration_ms BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE playlist_tracks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    playlist_id UUID NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    added_by UUID REFERENCES users(id) ON DELETE SET NULL,
    position INTEGER NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(playlist_id, track_id)
);

CREATE TABLE playlist_collaborators (
    playlist_id UUID NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    can_edit BOOLEAN NOT NULL DEFAULT true,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (playlist_id, user_id)
);

CREATE TABLE listening_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    track_id UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    played_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    duration_played_ms INTEGER NOT NULL,
    completed BOOLEAN NOT NULL DEFAULT false,
    context_type context_type,
    context_id UUID,
    device_id VARCHAR(255),
    scrobbled BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE user_library (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_type item_type NOT NULL,
    item_id UUID NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, item_type, item_id)
);

COMMENT ON TABLE playlists IS 'User playlists including smart playlists with rules';
COMMENT ON TABLE playlist_tracks IS 'Playlist track relationships with ordering';
COMMENT ON TABLE playlist_collaborators IS 'Collaborative playlist permissions';
COMMENT ON TABLE listening_history IS 'Complete listening history for recommendations';
COMMENT ON TABLE user_library IS 'User library (liked tracks, albums, artists)';
COMMENT ON COLUMN playlists.smart_rules IS 'Rule definitions for smart playlists';
