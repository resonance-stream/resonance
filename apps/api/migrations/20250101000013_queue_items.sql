-- Resonance: Persistent Queue Items
-- Migration: 20250101000013_queue_items
--
-- This table stores the user's explicit play queue for:
-- 1. Cross-session persistence (resume queue after app restart)
-- 2. Queue-based prefetch by the worker (vs autoplay prediction)
-- 3. Cross-device sync (WebSocket sync keeps Redis warm, this is cold storage)
--
-- Note: This is the persistence layer for queue state. The hot path uses
-- Redis pub/sub via WebSocket sync. listening_activity.queue stores
-- ephemeral real-time state, while queue_items is the durable source of truth.
--
-- Lifecycle: queue_state rows are created via UPSERT on first queue operation.

-- Queue items: tracks explicitly added to user's play queue
CREATE TABLE queue_items (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- User who owns this queue
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Track in the queue
    track_id UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,

    -- Position in queue (0-indexed, lower = earlier)
    -- Users can queue the same track multiple times, so position is unique per user
    position INTEGER NOT NULL,
    CONSTRAINT check_position_non_negative CHECK (position >= 0),

    -- Source context: where did this track come from?
    -- Helps with analytics and smart prefetch weighting
    source_type context_type,
    source_id UUID,

    -- When was this added to queue
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Metadata for prefetch optimization
    -- e.g., { "prefetched": true, "prefetch_priority": 0.85 }
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Position must be unique per user (but same track can appear multiple times)
    UNIQUE(user_id, position)
);

-- Current playback position within the queue
-- Separate table to avoid row-level contention on queue_items during frequent index updates
-- Rows are created via UPSERT on first queue operation, deleted when user account is deleted
CREATE TABLE queue_state (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,

    -- Current position in queue (index into queue_items by position)
    current_index INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT check_current_index_non_negative CHECK (current_index >= 0),

    -- Last time queue state was modified
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient queue operations
CREATE INDEX idx_queue_items_user_position ON queue_items(user_id, position);
CREATE INDEX idx_queue_items_track ON queue_items(track_id);
CREATE INDEX idx_queue_items_added ON queue_items(user_id, added_at DESC);

-- Row-Level Security for multi-user isolation
ALTER TABLE queue_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE queue_state ENABLE ROW LEVEL SECURITY;

-- Users can only access their own queue items
CREATE POLICY queue_items_user_policy ON queue_items
    FOR ALL
    USING (user_id = current_setting('app.current_user_id', true)::uuid)
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::uuid);

CREATE POLICY queue_state_user_policy ON queue_state
    FOR ALL
    USING (user_id = current_setting('app.current_user_id', true)::uuid)
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::uuid);

-- Trigger for automatic updated_at on queue_state
CREATE TRIGGER update_queue_state_updated_at
    BEFORE UPDATE ON queue_state
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Comments for documentation
COMMENT ON TABLE queue_items IS 'Persistent user play queue for cross-session and cross-device sync';
COMMENT ON TABLE queue_state IS 'Current playback position within each user queue';
COMMENT ON COLUMN queue_items.source_type IS 'Context where track was added from (album, playlist, search, etc.)';
COMMENT ON COLUMN queue_items.metadata IS 'Prefetch metadata: prefetched flag, priority score, etc.';
COMMENT ON COLUMN queue_items.position IS '0-indexed position in queue; lower values play first; same track can appear at multiple positions';
