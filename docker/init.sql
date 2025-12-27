-- Resonance: PostgreSQL Schema Initialization
-- This file is executed on first database start

-- ============================================================================
-- EXTENSIONS
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "vector";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- ============================================================================
-- CUSTOM TYPES
-- ============================================================================

CREATE TYPE user_role AS ENUM ('admin', 'user', 'guest');
CREATE TYPE album_type AS ENUM ('album', 'single', 'ep', 'compilation', 'live', 'remix', 'soundtrack', 'other');
CREATE TYPE audio_format AS ENUM ('flac', 'mp3', 'aac', 'opus', 'ogg', 'wav', 'alac', 'other');
CREATE TYPE playlist_type AS ENUM ('manual', 'smart', 'discover', 'radio');
CREATE TYPE context_type AS ENUM ('album', 'artist', 'playlist', 'search', 'recommendation', 'radio', 'queue');
CREATE TYPE item_type AS ENUM ('track', 'album', 'artist', 'playlist');
CREATE TYPE download_status AS ENUM ('pending', 'downloading', 'completed', 'failed');
CREATE TYPE sync_status AS ENUM ('idle', 'syncing', 'error');

-- ============================================================================
-- USERS & AUTH TABLES
-- ============================================================================

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    avatar_url VARCHAR(500),
    role user_role NOT NULL DEFAULT 'user',
    preferences JSONB NOT NULL DEFAULT '{
        "theme": "dark",
        "quality": "high",
        "crossfade_duration_ms": 0,
        "gapless_playback": true,
        "normalize_volume": false,
        "show_explicit": true,
        "private_session": false,
        "discord_rpc": true,
        "listenbrainz_scrobble": false
    }'::jsonb,
    listenbrainz_token VARCHAR(255),
    discord_user_id VARCHAR(50),
    email_verified BOOLEAN NOT NULL DEFAULT false,
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    refresh_token_hash VARCHAR(255),
    device_name VARCHAR(100),
    device_type VARCHAR(50),
    device_id VARCHAR(255),
    ip_address INET,
    user_agent TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_active_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- LIBRARY TABLES
-- ============================================================================

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

-- ============================================================================
-- USER DATA TABLES
-- ============================================================================

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

-- ============================================================================
-- ADDITIONAL TABLES
-- ============================================================================

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

-- ============================================================================
-- INDEXES
-- ============================================================================

-- B-tree indexes on foreign keys
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX idx_sessions_is_active ON sessions(is_active) WHERE is_active = true;

CREATE INDEX idx_albums_artist_id ON albums(artist_id);
CREATE INDEX idx_albums_release_date ON albums(release_date);

CREATE INDEX idx_tracks_album_id ON tracks(album_id);
CREATE INDEX idx_tracks_artist_id ON tracks(artist_id);
CREATE INDEX idx_tracks_file_path ON tracks(file_path);
CREATE INDEX idx_tracks_duration ON tracks(duration_ms);
CREATE INDEX idx_tracks_play_count ON tracks(play_count DESC);

CREATE INDEX idx_playlists_user_id ON playlists(user_id);
CREATE INDEX idx_playlists_is_public ON playlists(is_public) WHERE is_public = true;
CREATE INDEX idx_playlists_playlist_type ON playlists(playlist_type);

CREATE INDEX idx_playlist_tracks_playlist_id ON playlist_tracks(playlist_id);
CREATE INDEX idx_playlist_tracks_track_id ON playlist_tracks(track_id);
CREATE INDEX idx_playlist_tracks_position ON playlist_tracks(playlist_id, position);

CREATE INDEX idx_playlist_collaborators_user_id ON playlist_collaborators(user_id);

CREATE INDEX idx_listening_history_user_id ON listening_history(user_id);
CREATE INDEX idx_listening_history_track_id ON listening_history(track_id);
CREATE INDEX idx_listening_history_played_at ON listening_history(user_id, played_at DESC);
CREATE INDEX idx_listening_history_scrobbled ON listening_history(scrobbled) WHERE scrobbled = false;

CREATE INDEX idx_user_library_user_id ON user_library(user_id);
CREATE INDEX idx_user_library_item ON user_library(item_type, item_id);

CREATE INDEX idx_listening_activity_user_id ON listening_activity(user_id);
CREATE INDEX idx_listening_activity_is_playing ON listening_activity(is_playing) WHERE is_playing = true;

CREATE INDEX idx_equalizer_presets_user_id ON equalizer_presets(user_id);

CREATE INDEX idx_pending_downloads_status ON pending_downloads(status);
CREATE INDEX idx_pending_downloads_user_id ON pending_downloads(user_id);

CREATE INDEX idx_recommendation_cache_user_id ON recommendation_cache(user_id);
CREATE INDEX idx_recommendation_cache_expires ON recommendation_cache(expires_at);

-- GIN indexes on array columns
CREATE INDEX idx_artists_genres ON artists USING GIN(genres);
CREATE INDEX idx_albums_genres ON albums USING GIN(genres);
CREATE INDEX idx_tracks_genres ON tracks USING GIN(genres);
CREATE INDEX idx_tracks_ai_mood ON tracks USING GIN(ai_mood);
CREATE INDEX idx_tracks_ai_tags ON tracks USING GIN(ai_tags);

-- Full-text search indexes
CREATE INDEX idx_artists_name_fts ON artists USING GIN(to_tsvector('english', name));
CREATE INDEX idx_albums_title_fts ON albums USING GIN(to_tsvector('english', title));
CREATE INDEX idx_tracks_title_fts ON tracks USING GIN(to_tsvector('english', title));
CREATE INDEX idx_playlists_name_fts ON playlists USING GIN(to_tsvector('english', name));

-- Trigram indexes for fuzzy search
CREATE INDEX idx_artists_name_trgm ON artists USING GIN(name gin_trgm_ops);
CREATE INDEX idx_albums_title_trgm ON albums USING GIN(title gin_trgm_ops);
CREATE INDEX idx_tracks_title_trgm ON tracks USING GIN(title gin_trgm_ops);
CREATE INDEX idx_artists_sort_name_trgm ON artists USING GIN(sort_name gin_trgm_ops);

-- HNSW indexes on vector columns for fast similarity search
CREATE INDEX idx_track_embeddings_title ON track_embeddings
    USING hnsw(title_embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX idx_track_embeddings_description ON track_embeddings
    USING hnsw(description_embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX idx_track_embeddings_audio ON track_embeddings
    USING hnsw(audio_embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- ============================================================================
-- TRIGGERS FOR UPDATED_AT
-- ============================================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_artists_updated_at
    BEFORE UPDATE ON artists
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_albums_updated_at
    BEFORE UPDATE ON albums
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_tracks_updated_at
    BEFORE UPDATE ON tracks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_track_embeddings_updated_at
    BEFORE UPDATE ON track_embeddings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_playlists_updated_at
    BEFORE UPDATE ON playlists
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_equalizer_presets_updated_at
    BEFORE UPDATE ON equalizer_presets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_lidarr_sync_state_updated_at
    BEFORE UPDATE ON lidarr_sync_state
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_pending_downloads_updated_at
    BEFORE UPDATE ON pending_downloads
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- TRIGGER FOR PLAYLIST TRACK COUNT
-- ============================================================================

CREATE OR REPLACE FUNCTION update_playlist_stats()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE playlists SET
            track_count = track_count + 1,
            total_duration_ms = total_duration_ms + COALESCE(
                (SELECT duration_ms FROM tracks WHERE id = NEW.track_id), 0
            ),
            updated_at = NOW()
        WHERE id = NEW.playlist_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE playlists SET
            track_count = GREATEST(track_count - 1, 0),
            total_duration_ms = GREATEST(total_duration_ms - COALESCE(
                (SELECT duration_ms FROM tracks WHERE id = OLD.track_id), 0
            ), 0),
            updated_at = NOW()
        WHERE id = OLD.playlist_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_playlist_stats_trigger
    AFTER INSERT OR DELETE ON playlist_tracks
    FOR EACH ROW EXECUTE FUNCTION update_playlist_stats();

-- ============================================================================
-- TRIGGER FOR TRACK PLAY COUNT
-- ============================================================================

CREATE OR REPLACE FUNCTION update_track_play_count()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.completed = true THEN
        UPDATE tracks SET
            play_count = play_count + 1,
            last_played_at = NEW.played_at
        WHERE id = NEW.track_id;
    END IF;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_track_play_count_trigger
    AFTER INSERT ON listening_history
    FOR EACH ROW EXECUTE FUNCTION update_track_play_count();

-- ============================================================================
-- DEFAULT EQUALIZER PRESETS
-- ============================================================================

INSERT INTO equalizer_presets (name, is_default, bands, preamp) VALUES
('Flat', true, '{"32": 0, "64": 0, "125": 0, "250": 0, "500": 0, "1000": 0, "2000": 0, "4000": 0, "8000": 0, "16000": 0}', 0),
('Bass Boost', true, '{"32": 5, "64": 4, "125": 3, "250": 2, "500": 0, "1000": 0, "2000": 0, "4000": 0, "8000": 0, "16000": 0}', 0),
('Treble Boost', true, '{"32": 0, "64": 0, "125": 0, "250": 0, "500": 0, "1000": 1, "2000": 2, "4000": 3, "8000": 4, "16000": 5}', 0),
('Vocal', true, '{"32": -2, "64": -1, "125": 0, "250": 1, "500": 3, "1000": 4, "2000": 3, "4000": 1, "8000": 0, "16000": -1}', 0),
('Rock', true, '{"32": 4, "64": 3, "125": 1, "250": 0, "500": -1, "1000": 0, "2000": 1, "4000": 3, "8000": 4, "16000": 4}', 0),
('Electronic', true, '{"32": 5, "64": 4, "125": 1, "250": 0, "500": -2, "1000": 1, "2000": 0, "4000": 2, "8000": 4, "16000": 5}', 0),
('Classical', true, '{"32": 0, "64": 0, "125": 0, "250": 0, "500": 0, "1000": 0, "2000": -1, "4000": -2, "8000": -2, "16000": -3}', 3),
('Jazz', true, '{"32": 3, "64": 2, "125": 1, "250": 1, "500": -1, "1000": -1, "2000": 0, "4000": 1, "8000": 2, "16000": 3}', 0),
('Hip-Hop', true, '{"32": 5, "64": 4, "125": 2, "250": 1, "500": -1, "1000": -1, "2000": 1, "4000": 0, "8000": 1, "16000": 3}', 0),
('Acoustic', true, '{"32": 2, "64": 2, "125": 1, "250": 1, "500": 0, "1000": 1, "2000": 2, "4000": 2, "8000": 1, "16000": 0}', 0);

-- ============================================================================
-- INITIAL LIDARR SYNC STATE
-- ============================================================================

INSERT INTO lidarr_sync_state (status) VALUES ('idle');

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE users IS 'User accounts with authentication and preferences';
COMMENT ON TABLE sessions IS 'Active user sessions with device information';
COMMENT ON TABLE artists IS 'Artist metadata with MusicBrainz and Lidarr integration';
COMMENT ON TABLE albums IS 'Album metadata with cover art color extraction';
COMMENT ON TABLE tracks IS 'Track metadata, file info, audio features, and AI tags';
COMMENT ON TABLE track_embeddings IS 'Vector embeddings for semantic search and recommendations';
COMMENT ON TABLE playlists IS 'User playlists including smart playlists with rules';
COMMENT ON TABLE playlist_tracks IS 'Playlist track relationships with ordering';
COMMENT ON TABLE playlist_collaborators IS 'Collaborative playlist permissions';
COMMENT ON TABLE listening_history IS 'Complete listening history for recommendations';
COMMENT ON TABLE user_library IS 'User library (liked tracks, albums, artists)';
COMMENT ON TABLE listening_activity IS 'Real-time playback state for cross-device sync';
COMMENT ON TABLE equalizer_presets IS 'User and system equalizer presets';
COMMENT ON TABLE lidarr_sync_state IS 'Lidarr synchronization status';
COMMENT ON TABLE pending_downloads IS 'Lidarr download queue';
COMMENT ON TABLE recommendation_cache IS 'Cached recommendations per user';

COMMENT ON COLUMN tracks.audio_features IS 'Audio features extracted by analysis (BPM, key, energy, etc.)';
COMMENT ON COLUMN tracks.ai_mood IS 'AI-detected mood tags (e.g., happy, melancholic, energetic)';
COMMENT ON COLUMN tracks.ai_tags IS 'AI-generated descriptive tags';
COMMENT ON COLUMN albums.cover_art_colors IS 'Extracted color palette for visualizer';
COMMENT ON COLUMN playlists.smart_rules IS 'Rule definitions for smart playlists';
COMMENT ON COLUMN track_embeddings.title_embedding IS 'Text embedding of track title (768-dim)';
COMMENT ON COLUMN track_embeddings.description_embedding IS 'Text embedding of AI description (768-dim)';
COMMENT ON COLUMN track_embeddings.audio_embedding IS 'Audio feature embedding (128-dim)';
