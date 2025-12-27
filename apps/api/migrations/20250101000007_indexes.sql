-- Resonance: Database Indexes
-- Migration: 20250101000007_indexes

-- B-tree indexes on foreign keys and common query fields
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
