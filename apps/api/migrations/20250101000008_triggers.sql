-- Resonance: Triggers and Functions
-- Migration: 20250101000008_triggers

-- Function to automatically update updated_at column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply updated_at triggers to all tables with updated_at column
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

-- Function to update playlist statistics when tracks are added/removed
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

-- Function to update track play count when listening history is recorded
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
