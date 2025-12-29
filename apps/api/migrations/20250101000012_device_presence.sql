-- Device presence table for tracking connected devices and their state
-- This enables cross-device sync features and persists device info for offline state

CREATE TABLE IF NOT EXISTS device_presence (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- User who owns this device
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Device identification
    device_id VARCHAR(128) NOT NULL,
    device_name VARCHAR(255) NOT NULL DEFAULT 'Unknown Device',
    device_type VARCHAR(32) NOT NULL DEFAULT 'unknown',

    -- User agent string for debugging
    user_agent TEXT,

    -- Connection state
    is_connected BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,  -- Is this the device controlling playback?

    -- Last known playback state (for reconnection sync)
    last_track_id VARCHAR(255),
    last_position_ms BIGINT DEFAULT 0,
    last_is_playing BOOLEAN DEFAULT FALSE,

    -- Timestamps
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    connected_at TIMESTAMPTZ,
    disconnected_at TIMESTAMPTZ,

    -- Ensure unique device per user
    CONSTRAINT unique_user_device UNIQUE (user_id, device_id),

    -- Ensure position is non-negative (playback position cannot be negative)
    CONSTRAINT check_position_non_negative CHECK (last_position_ms IS NULL OR last_position_ms >= 0)
);

-- Indexes for common queries
CREATE INDEX idx_device_presence_user_id ON device_presence(user_id);
CREATE INDEX idx_device_presence_user_connected ON device_presence(user_id, is_connected) WHERE is_connected = TRUE;
CREATE INDEX idx_device_presence_user_active ON device_presence(user_id, is_active) WHERE is_active = TRUE;
CREATE INDEX idx_device_presence_last_seen ON device_presence(last_seen_at);

-- Enable RLS
ALTER TABLE device_presence ENABLE ROW LEVEL SECURITY;

-- RLS policies: users can only see their own devices
CREATE POLICY device_presence_select ON device_presence
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', TRUE)::UUID);

CREATE POLICY device_presence_insert ON device_presence
    FOR INSERT
    WITH CHECK (user_id = current_setting('app.current_user_id', TRUE)::UUID);

CREATE POLICY device_presence_update ON device_presence
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', TRUE)::UUID);

CREATE POLICY device_presence_delete ON device_presence
    FOR DELETE
    USING (user_id = current_setting('app.current_user_id', TRUE)::UUID);

-- Function to update last_seen_at automatically
CREATE OR REPLACE FUNCTION update_device_last_seen()
RETURNS TRIGGER AS $$
BEGIN
    NEW.last_seen_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to update last_seen_at on any update
CREATE TRIGGER device_presence_update_last_seen
    BEFORE UPDATE ON device_presence
    FOR EACH ROW
    EXECUTE FUNCTION update_device_last_seen();

-- Comment on table
COMMENT ON TABLE device_presence IS 'Tracks connected devices for cross-device playback synchronization';
COMMENT ON COLUMN device_presence.device_id IS 'Client-provided persistent device identifier';
COMMENT ON COLUMN device_presence.is_active IS 'Whether this device is currently controlling playback (Spotify Connect style)';
