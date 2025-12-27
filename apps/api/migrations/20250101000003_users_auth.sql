-- Resonance: Users and Authentication Tables
-- Migration: 20250101000003_users_auth

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

COMMENT ON TABLE users IS 'User accounts with authentication and preferences';
COMMENT ON TABLE sessions IS 'Active user sessions with device information';
