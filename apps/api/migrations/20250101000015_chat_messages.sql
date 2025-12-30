-- Resonance: AI Chat Messages
-- Migration: 20250101000015_chat_messages
-- Description: Chat conversations and messages for AI assistant

-- Chat conversations (sessions)
CREATE TABLE chat_conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL DEFAULT 'New Conversation',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

-- Chat messages within conversations
CREATE TABLE chat_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES chat_conversations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    -- Content can be NULL for assistant messages with tool_calls
    content TEXT,
    -- Sequence for guaranteed message ordering within conversation
    sequence_number INTEGER NOT NULL,
    -- Tool call tracking for function calling
    tool_calls JSONB,
    tool_call_id VARCHAR(100),
    -- Metadata
    context_snapshot JSONB,
    model_used VARCHAR(100),
    token_count INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Ensure tool_calls is an array when present
    CONSTRAINT check_tool_calls_is_array
        CHECK (tool_calls IS NULL OR jsonb_typeof(tool_calls) = 'array')
);

-- Indexes for efficient querying
CREATE INDEX idx_chat_conversations_user_id ON chat_conversations(user_id);
CREATE INDEX idx_chat_conversations_updated_at ON chat_conversations(updated_at DESC);
-- Partial index for finding active conversations by user
CREATE INDEX idx_chat_conversations_active_by_user
    ON chat_conversations(user_id, updated_at DESC)
    WHERE deleted_at IS NULL;
-- Composite index for message ordering within conversation
CREATE INDEX idx_chat_messages_conversation_sequence
    ON chat_messages(conversation_id, sequence_number);
-- Unique constraint on sequence within conversation
CREATE UNIQUE INDEX idx_chat_messages_unique_sequence
    ON chat_messages(conversation_id, sequence_number);
-- Index for tool call result lookups
CREATE INDEX idx_chat_messages_tool_call_id
    ON chat_messages(tool_call_id)
    WHERE tool_call_id IS NOT NULL;
-- GIN index for querying tool calls by function name
CREATE INDEX idx_chat_messages_tool_calls
    ON chat_messages USING GIN(tool_calls jsonb_path_ops)
    WHERE tool_calls IS NOT NULL;

-- Enable RLS
ALTER TABLE chat_conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE chat_messages ENABLE ROW LEVEL SECURITY;

-- RLS policies - users can only see their own conversations
CREATE POLICY chat_conversations_select_policy ON chat_conversations
    FOR SELECT USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY chat_conversations_insert_policy ON chat_conversations
    FOR INSERT WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY chat_conversations_update_policy ON chat_conversations
    FOR UPDATE USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY chat_conversations_delete_policy ON chat_conversations
    FOR DELETE USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- RLS policies for messages
CREATE POLICY chat_messages_select_policy ON chat_messages
    FOR SELECT USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY chat_messages_insert_policy ON chat_messages
    FOR INSERT WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY chat_messages_update_policy ON chat_messages
    FOR UPDATE USING (user_id = current_setting('app.current_user_id', true)::UUID);

CREATE POLICY chat_messages_delete_policy ON chat_messages
    FOR DELETE USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- Trigger for direct updates to chat_conversations (using existing function from triggers migration)
CREATE TRIGGER update_chat_conversations_updated_at
    BEFORE UPDATE ON chat_conversations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger to update conversation updated_at when messages are added
-- Uses SECURITY DEFINER to bypass RLS for the update operation
-- Includes user_id check to prevent cross-user updates
CREATE OR REPLACE FUNCTION update_chat_conversation_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE chat_conversations
    SET updated_at = NOW()
    WHERE id = NEW.conversation_id
      AND user_id = NEW.user_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Conversation does not belong to user';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = 'pg_catalog, public';

CREATE TRIGGER chat_messages_update_conversation_timestamp
    AFTER INSERT ON chat_messages
    FOR EACH ROW
    EXECUTE FUNCTION update_chat_conversation_updated_at();

COMMENT ON TABLE chat_conversations IS 'AI chat conversation sessions per user';
COMMENT ON TABLE chat_messages IS 'Individual messages within chat conversations';
COMMENT ON COLUMN chat_messages.sequence_number IS 'Message order within conversation (monotonically increasing)';
COMMENT ON COLUMN chat_messages.tool_calls IS 'Array of tool calls in OpenAI format: [{"id": "call_xxx", "type": "function", "function": {"name": "...", "arguments": "..."}}]';
COMMENT ON COLUMN chat_messages.tool_call_id IS 'ID linking tool result message to its original call';
COMMENT ON COLUMN chat_messages.context_snapshot IS 'User context at message time: {"current_track_id": UUID, "listening_history": [...], ...}';
