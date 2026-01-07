-- Resonance: Row Level Security Policies for User-Scoped Tables
-- Migration: 20250101000021_rls_policies
--
-- This migration adds Row Level Security (RLS) policies to 11 user-scoped tables
-- that were missing RLS protection. Each table uses current_setting('app.current_user_id')
-- to scope access to the authenticated user's data.
--
-- Tables covered:
-- 1. listening_history - User's listening history
-- 2. sessions - User auth sessions
-- 3. playlists - User playlists (with public visibility support)
-- 4. user_library - Liked tracks/albums/artists
-- 5. playlist_tracks - Tracks within playlists (via playlist ownership/collaboration)
-- 6. playlist_collaborators - Playlist collaboration permissions
-- 7. equalizer_presets - User EQ presets (includes system defaults)
-- 8. recommendation_cache - Cached recommendations
-- 9. pending_downloads - User download requests
-- 10. user_taste_clusters - K-means taste clusters
-- 11. listening_activity - Real-time playback state
--
-- Note: The `true` parameter in current_setting() returns NULL if the setting
-- doesn't exist, preventing errors during migrations or system operations.

-- ============================================================================
-- 1. LISTENING_HISTORY
-- ============================================================================
-- User can only access their own listening history
ALTER TABLE listening_history ENABLE ROW LEVEL SECURITY;

CREATE POLICY listening_history_select_policy ON listening_history
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY listening_history_insert_policy ON listening_history
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY listening_history_update_policy ON listening_history
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY listening_history_delete_policy ON listening_history
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================================
-- 2. SESSIONS
-- ============================================================================
-- User can only access their own sessions
ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;

CREATE POLICY sessions_select_policy ON sessions
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY sessions_insert_policy ON sessions
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY sessions_update_policy ON sessions
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY sessions_delete_policy ON sessions
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================================
-- 3. PLAYLISTS
-- ============================================================================
-- User can see their own playlists, public playlists, and playlists they collaborate on
ALTER TABLE playlists ENABLE ROW LEVEL SECURITY;

CREATE POLICY playlists_select_policy ON playlists
    FOR SELECT
    USING (
        user_id = current_setting('app.current_user_id', true)::UUID
        OR is_public = true
        OR EXISTS (
            SELECT 1 FROM playlist_collaborators pc
            WHERE pc.playlist_id = playlists.id
            AND pc.user_id = current_setting('app.current_user_id', true)::UUID
        )
    );

CREATE POLICY playlists_insert_policy ON playlists
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY playlists_update_policy ON playlists
    FOR UPDATE
    USING (
        user_id = current_setting('app.current_user_id', true)::UUID
        OR EXISTS (
            SELECT 1 FROM playlist_collaborators pc
            WHERE pc.playlist_id = playlists.id
            AND pc.user_id = current_setting('app.current_user_id', true)::UUID
            AND pc.can_edit = true
        )
    );

CREATE POLICY playlists_delete_policy ON playlists
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================================
-- 4. USER_LIBRARY
-- ============================================================================
-- User can only access their own library
ALTER TABLE user_library ENABLE ROW LEVEL SECURITY;

CREATE POLICY user_library_select_policy ON user_library
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_library_insert_policy ON user_library
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_library_update_policy ON user_library
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_library_delete_policy ON user_library
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================================
-- 5. PLAYLIST_TRACKS
-- ============================================================================
-- Access controlled via playlist ownership/collaboration
ALTER TABLE playlist_tracks ENABLE ROW LEVEL SECURITY;

-- Can see tracks if user owns the playlist, playlist is public, or user is collaborator
CREATE POLICY playlist_tracks_select_policy ON playlist_tracks
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM playlists p
            WHERE p.id = playlist_tracks.playlist_id
            AND (
                p.user_id = current_setting('app.current_user_id', true)::UUID
                OR p.is_public = true
                OR EXISTS (
                    SELECT 1 FROM playlist_collaborators pc
                    WHERE pc.playlist_id = p.id
                    AND pc.user_id = current_setting('app.current_user_id', true)::UUID
                )
            )
        )
    );

-- Can insert tracks if user owns playlist or is collaborator with edit permission
CREATE POLICY playlist_tracks_insert_policy ON playlist_tracks
    FOR INSERT
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM playlists p
            WHERE p.id = playlist_tracks.playlist_id
            AND (
                p.user_id = current_setting('app.current_user_id', true)::UUID
                OR EXISTS (
                    SELECT 1 FROM playlist_collaborators pc
                    WHERE pc.playlist_id = p.id
                    AND pc.user_id = current_setting('app.current_user_id', true)::UUID
                    AND pc.can_edit = true
                )
            )
        )
    );

-- Can update tracks if user owns playlist or is collaborator with edit permission
CREATE POLICY playlist_tracks_update_policy ON playlist_tracks
    FOR UPDATE
    USING (
        EXISTS (
            SELECT 1 FROM playlists p
            WHERE p.id = playlist_tracks.playlist_id
            AND (
                p.user_id = current_setting('app.current_user_id', true)::UUID
                OR EXISTS (
                    SELECT 1 FROM playlist_collaborators pc
                    WHERE pc.playlist_id = p.id
                    AND pc.user_id = current_setting('app.current_user_id', true)::UUID
                    AND pc.can_edit = true
                )
            )
        )
    );

-- Can delete tracks if user owns playlist or is collaborator with edit permission
CREATE POLICY playlist_tracks_delete_policy ON playlist_tracks
    FOR DELETE
    USING (
        EXISTS (
            SELECT 1 FROM playlists p
            WHERE p.id = playlist_tracks.playlist_id
            AND (
                p.user_id = current_setting('app.current_user_id', true)::UUID
                OR EXISTS (
                    SELECT 1 FROM playlist_collaborators pc
                    WHERE pc.playlist_id = p.id
                    AND pc.user_id = current_setting('app.current_user_id', true)::UUID
                    AND pc.can_edit = true
                )
            )
        )
    );

-- ============================================================================
-- 6. PLAYLIST_COLLABORATORS
-- ============================================================================
-- Owner can manage collaborators; collaborators can see each other
ALTER TABLE playlist_collaborators ENABLE ROW LEVEL SECURITY;

-- Can see collaborators if user owns the playlist or is a collaborator
CREATE POLICY playlist_collaborators_select_policy ON playlist_collaborators
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM playlists p
            WHERE p.id = playlist_collaborators.playlist_id
            AND (
                p.user_id = current_setting('app.current_user_id', true)::UUID
                OR EXISTS (
                    SELECT 1 FROM playlist_collaborators pc2
                    WHERE pc2.playlist_id = p.id
                    AND pc2.user_id = current_setting('app.current_user_id', true)::UUID
                )
            )
        )
    );

-- Only playlist owner can add collaborators
CREATE POLICY playlist_collaborators_insert_policy ON playlist_collaborators
    FOR INSERT
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM playlists p
            WHERE p.id = playlist_collaborators.playlist_id
            AND p.user_id = current_setting('app.current_user_id', true)::UUID
        )
    );

-- Only playlist owner can update collaborator permissions
CREATE POLICY playlist_collaborators_update_policy ON playlist_collaborators
    FOR UPDATE
    USING (
        EXISTS (
            SELECT 1 FROM playlists p
            WHERE p.id = playlist_collaborators.playlist_id
            AND p.user_id = current_setting('app.current_user_id', true)::UUID
        )
    );

-- Owner can remove any collaborator; collaborators can remove themselves
CREATE POLICY playlist_collaborators_delete_policy ON playlist_collaborators
    FOR DELETE
    USING (
        playlist_collaborators.user_id = current_setting('app.current_user_id', true)::UUID
        OR EXISTS (
            SELECT 1 FROM playlists p
            WHERE p.id = playlist_collaborators.playlist_id
            AND p.user_id = current_setting('app.current_user_id', true)::UUID
        )
    );

-- ============================================================================
-- 7. EQUALIZER_PRESETS
-- ============================================================================
-- User can access their own presets and system defaults (user_id IS NULL)
ALTER TABLE equalizer_presets ENABLE ROW LEVEL SECURITY;

CREATE POLICY equalizer_presets_select_policy ON equalizer_presets
    FOR SELECT
    USING (
        user_id IS NULL  -- System presets are visible to all
        OR user_id = current_setting('app.current_user_id', true)::UUID
    );

CREATE POLICY equalizer_presets_insert_policy ON equalizer_presets
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY equalizer_presets_update_policy ON equalizer_presets
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY equalizer_presets_delete_policy ON equalizer_presets
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================================
-- 8. RECOMMENDATION_CACHE
-- ============================================================================
-- User can only access their own recommendation cache
ALTER TABLE recommendation_cache ENABLE ROW LEVEL SECURITY;

CREATE POLICY recommendation_cache_select_policy ON recommendation_cache
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY recommendation_cache_insert_policy ON recommendation_cache
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY recommendation_cache_update_policy ON recommendation_cache
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY recommendation_cache_delete_policy ON recommendation_cache
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================================
-- 9. PENDING_DOWNLOADS
-- ============================================================================
-- User can access their own download requests; system downloads (user_id IS NULL) are admin-only
ALTER TABLE pending_downloads ENABLE ROW LEVEL SECURITY;

CREATE POLICY pending_downloads_select_policy ON pending_downloads
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY pending_downloads_insert_policy ON pending_downloads
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY pending_downloads_update_policy ON pending_downloads
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY pending_downloads_delete_policy ON pending_downloads
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================================
-- 10. USER_TASTE_CLUSTERS
-- ============================================================================
-- User can only access their own taste clusters
ALTER TABLE user_taste_clusters ENABLE ROW LEVEL SECURITY;

CREATE POLICY user_taste_clusters_select_policy ON user_taste_clusters
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_taste_clusters_insert_policy ON user_taste_clusters
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_taste_clusters_update_policy ON user_taste_clusters
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_taste_clusters_delete_policy ON user_taste_clusters
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================================
-- 11. LISTENING_ACTIVITY
-- ============================================================================
-- User can only access their own listening activity
ALTER TABLE listening_activity ENABLE ROW LEVEL SECURITY;

CREATE POLICY listening_activity_select_policy ON listening_activity
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY listening_activity_insert_policy ON listening_activity
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY listening_activity_update_policy ON listening_activity
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY listening_activity_delete_policy ON listening_activity
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================================
-- COMMENTS
-- ============================================================================
COMMENT ON POLICY listening_history_select_policy ON listening_history IS 'Users can only view their own listening history';
COMMENT ON POLICY sessions_select_policy ON sessions IS 'Users can only view their own sessions';
COMMENT ON POLICY playlists_select_policy ON playlists IS 'Users can view own playlists, public playlists, and collaborative playlists';
COMMENT ON POLICY user_library_select_policy ON user_library IS 'Users can only view their own library';
COMMENT ON POLICY playlist_tracks_select_policy ON playlist_tracks IS 'Access controlled via playlist ownership, publicity, or collaboration';
COMMENT ON POLICY playlist_collaborators_select_policy ON playlist_collaborators IS 'Owners and collaborators can view collaboration info';
COMMENT ON POLICY equalizer_presets_select_policy ON equalizer_presets IS 'Users see own presets plus system defaults';
COMMENT ON POLICY recommendation_cache_select_policy ON recommendation_cache IS 'Users can only view their own recommendation cache';
COMMENT ON POLICY pending_downloads_select_policy ON pending_downloads IS 'Users can only view their own download requests';
COMMENT ON POLICY user_taste_clusters_select_policy ON user_taste_clusters IS 'Users can only view their own taste clusters';
COMMENT ON POLICY listening_activity_select_policy ON listening_activity IS 'Users can only view their own listening activity';
