-- Resonance: Custom PostgreSQL Types
-- Migration: 20250101000002_custom_types

CREATE TYPE user_role AS ENUM ('admin', 'user', 'guest');
CREATE TYPE album_type AS ENUM ('album', 'single', 'ep', 'compilation', 'live', 'remix', 'soundtrack', 'other');
CREATE TYPE audio_format AS ENUM ('flac', 'mp3', 'aac', 'opus', 'ogg', 'wav', 'alac', 'other');
CREATE TYPE playlist_type AS ENUM ('manual', 'smart', 'discover', 'radio');
CREATE TYPE context_type AS ENUM ('album', 'artist', 'playlist', 'search', 'recommendation', 'radio', 'queue');
CREATE TYPE item_type AS ENUM ('track', 'album', 'artist', 'playlist');
CREATE TYPE download_status AS ENUM ('pending', 'downloading', 'completed', 'failed');
CREATE TYPE sync_status AS ENUM ('idle', 'syncing', 'error');
