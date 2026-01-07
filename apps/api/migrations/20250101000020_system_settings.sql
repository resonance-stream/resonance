-- Resonance: System Settings and Setup Tables
-- Migration: 20250101000020_system_settings
-- Description: Admin-configurable external service settings with encrypted secrets,
--              user library paths, and first-run setup status tracking

-- Service type enum for external integrations
CREATE TYPE service_type AS ENUM (
    'ollama',
    'lidarr',
    'lastfm',
    'meilisearch',
    'music_library'
);

-- System settings table for admin-configurable external services
-- One row per service type, managed by admins only
CREATE TABLE system_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service service_type NOT NULL UNIQUE,
    enabled BOOLEAN NOT NULL DEFAULT false,

    -- Configuration stored as JSON (URLs, ports, non-sensitive options)
    config JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Encrypted secrets (API keys, tokens) using AES-256-GCM
    -- Format: nonce (12 bytes) || ciphertext || auth_tag (16 bytes)
    encrypted_secrets BYTEA,

    -- Connection health tracking
    last_connection_test TIMESTAMPTZ,
    connection_healthy BOOLEAN,
    connection_error TEXT,

    -- Audit fields
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- User library paths - allows users to configure their own music library paths
-- Users can have multiple paths (e.g., local + NAS)
CREATE TABLE user_library_paths (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    label TEXT,  -- User-friendly label (e.g., "NAS Music", "Local Collection")
    is_primary BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Each user can only add a path once
    CONSTRAINT unique_user_path UNIQUE (user_id, path)
);

-- Setup status singleton - tracks whether first-run setup has been completed
-- Uses CHECK constraint to ensure only one row exists
CREATE TABLE setup_status (
    id INTEGER PRIMARY KEY DEFAULT 1,
    completed BOOLEAN NOT NULL DEFAULT false,
    completed_at TIMESTAMPTZ,
    completed_by UUID REFERENCES users(id) ON DELETE SET NULL,

    -- Ensure singleton: only id = 1 is allowed
    CONSTRAINT setup_status_singleton CHECK (id = 1)
);

-- Insert initial setup status row
INSERT INTO setup_status (id, completed) VALUES (1, false);

-- Indexes for system_settings
CREATE INDEX idx_system_settings_service ON system_settings(service);
CREATE INDEX idx_system_settings_enabled ON system_settings(service) WHERE enabled = true;

-- Indexes for user_library_paths
CREATE INDEX idx_user_library_paths_user_id ON user_library_paths(user_id);
CREATE INDEX idx_user_library_paths_user_primary ON user_library_paths(user_id, is_primary)
    WHERE is_primary = true;

-- Trigger for system_settings updated_at
CREATE TRIGGER update_system_settings_updated_at
    BEFORE UPDATE ON system_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- RLS for system_settings: admin-only access
ALTER TABLE system_settings ENABLE ROW LEVEL SECURITY;

-- Admin can do everything
CREATE POLICY system_settings_admin_all ON system_settings
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM users
            WHERE id = current_setting('app.current_user_id', true)::UUID
            AND role = 'admin'
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM users
            WHERE id = current_setting('app.current_user_id', true)::UUID
            AND role = 'admin'
        )
    );

-- Non-admin users can only read enabled settings (without secrets)
-- Note: The application layer should filter out encrypted_secrets for non-admins
CREATE POLICY system_settings_user_select ON system_settings
    FOR SELECT
    USING (
        NOT EXISTS (
            SELECT 1 FROM users
            WHERE id = current_setting('app.current_user_id', true)::UUID
            AND role = 'admin'
        )
        AND enabled = true
    );

-- RLS for user_library_paths: users manage their own paths
ALTER TABLE user_library_paths ENABLE ROW LEVEL SECURITY;

CREATE POLICY user_library_paths_select ON user_library_paths
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_library_paths_insert ON user_library_paths
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_library_paths_update ON user_library_paths
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY user_library_paths_delete ON user_library_paths
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- RLS for setup_status: anyone can read, only admin can update
ALTER TABLE setup_status ENABLE ROW LEVEL SECURITY;

CREATE POLICY setup_status_select ON setup_status
    FOR SELECT
    USING (true);  -- Anyone can read setup status

CREATE POLICY setup_status_update ON setup_status
    FOR UPDATE
    USING (
        EXISTS (
            SELECT 1 FROM users
            WHERE id = current_setting('app.current_user_id', true)::UUID
            AND role = 'admin'
        )
    );

-- Comments
COMMENT ON TABLE system_settings IS 'Admin-configurable external service settings with encrypted secrets';
COMMENT ON COLUMN system_settings.config IS 'Non-sensitive configuration as JSON (URLs, ports, model names)';
COMMENT ON COLUMN system_settings.encrypted_secrets IS 'AES-256-GCM encrypted API keys/tokens: nonce(12) || ciphertext || tag(16)';
COMMENT ON COLUMN system_settings.connection_healthy IS 'Result of last connection test (NULL if never tested)';

COMMENT ON TABLE user_library_paths IS 'User-configured music library paths (can have multiple per user)';
COMMENT ON COLUMN user_library_paths.is_primary IS 'Primary path used for default scans and imports';
COMMENT ON COLUMN user_library_paths.label IS 'User-friendly label for this path (e.g., "NAS Music")';

COMMENT ON TABLE setup_status IS 'Singleton tracking first-run setup wizard completion';
COMMENT ON COLUMN setup_status.id IS 'Always 1 (singleton constraint)';
