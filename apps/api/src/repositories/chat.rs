//! Chat repository for AI assistant database operations
//!
//! This module provides all chat-related database operations including
//! conversation management and message storage with function calling support.

// Allow dead_code - these methods are prepared for Phase 3 (Chat Service) implementation
#![allow(dead_code)]

use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::models::chat::{
    ChatConversation, ChatMessage, ChatRole, CreateChatMessage, CreateConversation, ToolCall,
};

/// Repository for chat database operations
///
/// Centralizes all chat-related database queries for AI conversations
/// and messages with support for function calling.
#[derive(Clone)]
pub struct ChatRepository {
    pool: PgPool,
}

impl ChatRepository {
    /// Create a new ChatRepository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a reference to the underlying connection pool
    #[allow(dead_code)]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ==================== Conversation Operations ====================

    /// Create a new chat conversation
    ///
    /// # Arguments
    /// * `input` - Conversation creation parameters
    ///
    /// # Returns
    /// * `Ok(ChatConversation)` - The newly created conversation
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self, input), fields(user_id = %input.user_id))]
    pub async fn create_conversation(
        &self,
        input: CreateConversation,
    ) -> Result<ChatConversation, sqlx::Error> {
        let title = input
            .title
            .unwrap_or_else(|| "New Conversation".to_string());

        sqlx::query_as::<_, ChatConversation>(
            r#"
            INSERT INTO chat_conversations (user_id, title)
            VALUES ($1, $2)
            RETURNING id, user_id, title, created_at, updated_at, deleted_at
            "#,
        )
        .bind(input.user_id)
        .bind(title)
        .fetch_one(&self.pool)
        .await
    }

    /// Find a conversation by ID for a specific user
    ///
    /// # Arguments
    /// * `id` - The conversation UUID
    /// * `user_id` - The user's UUID (for ownership check)
    ///
    /// # Returns
    /// * `Ok(Some(ChatConversation))` - If the conversation exists and belongs to user
    /// * `Ok(None)` - If no conversation found or not owned by user
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn find_conversation_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ChatConversation>, sqlx::Error> {
        sqlx::query_as::<_, ChatConversation>(
            r#"
            SELECT id, user_id, title, created_at, updated_at, deleted_at
            FROM chat_conversations
            WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Find all conversations for a user, ordered by most recent
    ///
    /// # Arguments
    /// * `user_id` - The user's UUID
    /// * `limit` - Maximum number of conversations to return
    /// * `offset` - Number of conversations to skip (for pagination)
    ///
    /// # Returns
    /// * `Ok(Vec<ChatConversation>)` - List of conversations
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn find_conversations_by_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChatConversation>, sqlx::Error> {
        sqlx::query_as::<_, ChatConversation>(
            r#"
            SELECT id, user_id, title, created_at, updated_at, deleted_at
            FROM chat_conversations
            WHERE user_id = $1 AND deleted_at IS NULL
            ORDER BY updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Count total conversations for a user
    ///
    /// # Arguments
    /// * `user_id` - The user's UUID
    ///
    /// # Returns
    /// * `Ok(i64)` - Total number of active conversations
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn count_conversations_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM chat_conversations
            WHERE user_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map(|count: Option<i64>| count.unwrap_or(0))
    }

    /// Update conversation title
    ///
    /// # Arguments
    /// * `id` - The conversation UUID
    /// * `user_id` - The user's UUID (for ownership check)
    /// * `title` - The new title
    ///
    /// # Returns
    /// * `Ok(Some(ChatConversation))` - Updated conversation
    /// * `Ok(None)` - If conversation not found
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn update_conversation_title(
        &self,
        id: Uuid,
        user_id: Uuid,
        title: &str,
    ) -> Result<Option<ChatConversation>, sqlx::Error> {
        sqlx::query_as::<_, ChatConversation>(
            r#"
            UPDATE chat_conversations
            SET title = $3, updated_at = NOW()
            WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL
            RETURNING id, user_id, title, created_at, updated_at, deleted_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(title)
        .fetch_optional(&self.pool)
        .await
    }

    /// Soft delete a conversation
    ///
    /// # Arguments
    /// * `id` - The conversation UUID
    /// * `user_id` - The user's UUID (for ownership check)
    ///
    /// # Returns
    /// * `Ok(true)` - If conversation was deleted
    /// * `Ok(false)` - If conversation not found
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn delete_conversation(&self, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE chat_conversations
            SET deleted_at = NOW()
            WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Soft delete all conversations for a user
    ///
    /// # Arguments
    /// * `user_id` - The user's UUID
    ///
    /// # Returns
    /// * `Ok(i64)` - Number of conversations deleted
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn delete_all_user_conversations(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE chat_conversations
            SET deleted_at = NOW()
            WHERE user_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    // ==================== Message Operations ====================

    /// Add a message to a conversation using atomic sequence number assignment
    ///
    /// Uses a CTE to atomically select and increment the sequence number,
    /// preventing race conditions when multiple messages are added concurrently.
    ///
    /// # Arguments
    /// * `input` - Message creation parameters
    ///
    /// # Returns
    /// * `Ok(ChatMessage)` - The newly created message
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self, input), fields(conversation_id = %input.conversation_id, role = %input.role))]
    pub async fn add_message(&self, input: CreateChatMessage) -> Result<ChatMessage, sqlx::Error> {
        // Serialize JSON fields with proper error handling
        let tool_calls_json = input
            .tool_calls
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| sqlx::Error::Protocol(format!("tool_calls serialization error: {}", e)))?;

        let context_json = input
            .context_snapshot
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| {
                sqlx::Error::Protocol(format!("context_snapshot serialization error: {}", e))
            })?;

        // Use a transaction with row-level lock to prevent race conditions on sequence_number
        let mut tx = self.pool.begin().await?;

        // Lock the conversation row and verify ownership/existence
        // Use fetch_optional since rows_affected() returns 0 for SELECT queries
        let locked: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM chat_conversations WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(input.conversation_id)
        .bind(input.user_id)
        .fetch_optional(&mut *tx)
        .await?;

        if locked.is_none() {
            return Err(sqlx::Error::RowNotFound);
        }

        // Now safe to compute and insert with sequence number
        let message = sqlx::query_as::<_, ChatMessage>(
            r#"
            INSERT INTO chat_messages (
                conversation_id, user_id, role, content, sequence_number,
                tool_calls, tool_call_id, context_snapshot, model_used, token_count
            )
            VALUES (
                $1, $2, $3, $4,
                (SELECT COALESCE(MAX(sequence_number), 0) + 1 FROM chat_messages WHERE conversation_id = $1),
                $5, $6, $7, $8, $9
            )
            RETURNING
                id, conversation_id, user_id, role, content, sequence_number,
                tool_calls, tool_call_id, context_snapshot, model_used, token_count, created_at
            "#,
        )
        .bind(input.conversation_id)
        .bind(input.user_id)
        .bind(input.role.as_str())
        .bind(input.content)
        .bind(tool_calls_json)
        .bind(input.tool_call_id)
        .bind(context_json)
        .bind(input.model_used)
        .bind(input.token_count)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(message)
    }

    /// Add multiple tool result messages atomically
    ///
    /// When the AI makes multiple parallel tool calls, this method inserts
    /// all tool results with sequential sequence numbers in a single transaction.
    ///
    /// # Arguments
    /// * `results` - Vector of tool result messages to insert
    ///
    /// # Returns
    /// * `Ok(Vec<ChatMessage>)` - The newly created messages
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self, results), fields(count = results.len()))]
    pub async fn add_tool_results(
        &self,
        results: Vec<CreateChatMessage>,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        if results.is_empty() {
            return Ok(vec![]);
        }

        // All results must share the same conversation_id and user_id
        let first = results.first().unwrap();
        if results
            .iter()
            .any(|r| r.conversation_id != first.conversation_id)
        {
            return Err(sqlx::Error::Protocol(
                "all tool results must share the same conversation_id".to_string(),
            ));
        }
        if results.iter().any(|r| r.user_id != first.user_id) {
            return Err(sqlx::Error::Protocol(
                "all tool results must share the same user_id".to_string(),
            ));
        }

        let mut messages = Vec::with_capacity(results.len());
        let mut tx = self.pool.begin().await?;

        // Lock the conversation row and verify ownership to prevent:
        // 1. Concurrent message insertions causing sequence number race conditions
        // 2. Users writing to other users' conversations
        // Use fetch_optional since rows_affected() returns 0 for SELECT queries
        let locked: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM chat_conversations WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(first.conversation_id)
        .bind(first.user_id)
        .fetch_optional(&mut *tx)
        .await?;

        if locked.is_none() {
            return Err(sqlx::Error::RowNotFound);
        }

        for input in results {
            let tool_calls_json = input
                .tool_calls
                .map(serde_json::to_value)
                .transpose()
                .map_err(|e| {
                    sqlx::Error::Protocol(format!("tool_calls serialization error: {}", e))
                })?;

            let context_json = input
                .context_snapshot
                .map(serde_json::to_value)
                .transpose()
                .map_err(|e| {
                    sqlx::Error::Protocol(format!("context_snapshot serialization error: {}", e))
                })?;

            let msg = sqlx::query_as::<_, ChatMessage>(
                r#"
                INSERT INTO chat_messages (
                    conversation_id, user_id, role, content, sequence_number,
                    tool_calls, tool_call_id, context_snapshot, model_used, token_count
                )
                VALUES (
                    $1, $2, $3, $4,
                    (SELECT COALESCE(MAX(sequence_number), 0) + 1 FROM chat_messages WHERE conversation_id = $1),
                    $5, $6, $7, $8, $9
                )
                RETURNING
                    id, conversation_id, user_id, role, content, sequence_number,
                    tool_calls, tool_call_id, context_snapshot, model_used, token_count, created_at
                "#,
            )
            .bind(input.conversation_id)
            .bind(input.user_id)
            .bind(input.role.as_str())
            .bind(input.content)
            .bind(tool_calls_json)
            .bind(input.tool_call_id)
            .bind(context_json)
            .bind(input.model_used)
            .bind(input.token_count)
            .fetch_one(&mut *tx)
            .await?;

            messages.push(msg);
        }

        tx.commit().await?;
        Ok(messages)
    }

    /// Get messages for a conversation, ordered by sequence
    ///
    /// # Arguments
    /// * `conversation_id` - The conversation UUID
    /// * `user_id` - The user's UUID (for ownership check)
    /// * `limit` - Maximum number of messages to return
    ///
    /// # Returns
    /// * `Ok(Vec<ChatMessage>)` - List of messages in order
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn get_messages(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessage>(
            r#"
            SELECT
                id, conversation_id, user_id, role, content, sequence_number,
                tool_calls, tool_call_id, context_snapshot, model_used, token_count, created_at
            FROM chat_messages
            WHERE conversation_id = $1 AND user_id = $2
            ORDER BY sequence_number ASC
            LIMIT $3
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Get messages for a conversation starting from a sequence number
    /// Useful for loading recent context for AI
    ///
    /// # Arguments
    /// * `conversation_id` - The conversation UUID
    /// * `user_id` - The user's UUID (for ownership check)
    /// * `from_sequence` - Start from this sequence number (inclusive)
    /// * `limit` - Maximum number of messages to return
    ///
    /// # Returns
    /// * `Ok(Vec<ChatMessage>)` - List of messages from the sequence
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn get_messages_from_sequence(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        from_sequence: i32,
        limit: i64,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessage>(
            r#"
            SELECT
                id, conversation_id, user_id, role, content, sequence_number,
                tool_calls, tool_call_id, context_snapshot, model_used, token_count, created_at
            FROM chat_messages
            WHERE conversation_id = $1 AND user_id = $2 AND sequence_number >= $3
            ORDER BY sequence_number ASC
            LIMIT $4
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(from_sequence)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Get the most recent N messages for context building
    ///
    /// # Arguments
    /// * `conversation_id` - The conversation UUID
    /// * `user_id` - The user's UUID (for ownership check)
    /// * `limit` - Number of recent messages to fetch
    ///
    /// # Returns
    /// * `Ok(Vec<ChatMessage>)` - List of recent messages in ascending order
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn get_recent_messages(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        // Subquery to get recent messages in desc order, then reverse
        sqlx::query_as::<_, ChatMessage>(
            r#"
            SELECT * FROM (
                SELECT
                    id, conversation_id, user_id, role, content, sequence_number,
                    tool_calls, tool_call_id, context_snapshot, model_used, token_count, created_at
                FROM chat_messages
                WHERE conversation_id = $1 AND user_id = $2
                ORDER BY sequence_number DESC
                LIMIT $3
            ) AS recent
            ORDER BY sequence_number ASC
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Count messages in a conversation
    ///
    /// # Arguments
    /// * `conversation_id` - The conversation UUID
    /// * `user_id` - The user's UUID (for ownership check)
    ///
    /// # Returns
    /// * `Ok(i64)` - Total number of messages
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn count_messages(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM chat_messages
            WHERE conversation_id = $1 AND user_id = $2
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map(|count: Option<i64>| count.unwrap_or(0))
    }

    /// Get total token count for a conversation (for context management)
    ///
    /// # Arguments
    /// * `conversation_id` - The conversation UUID
    /// * `user_id` - The user's UUID (for ownership check)
    ///
    /// # Returns
    /// * `Ok(i64)` - Total tokens used in conversation
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn get_total_tokens(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(token_count), 0)
            FROM chat_messages
            WHERE conversation_id = $1 AND user_id = $2
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Find a message by its tool_call_id
    /// Used to match tool results with their originating calls
    ///
    /// # Arguments
    /// * `tool_call_id` - The tool call ID to search for
    /// * `user_id` - The user's UUID (for ownership check)
    ///
    /// # Returns
    /// * `Ok(Some(ChatMessage))` - If message with tool_call_id found
    /// * `Ok(None)` - If no matching message found
    /// * `Err(sqlx::Error)` - If a database error occurs
    #[instrument(skip(self))]
    pub async fn find_message_by_tool_call_id(
        &self,
        tool_call_id: &str,
        user_id: Uuid,
    ) -> Result<Option<ChatMessage>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessage>(
            r#"
            SELECT
                id, conversation_id, user_id, role, content, sequence_number,
                tool_calls, tool_call_id, context_snapshot, model_used, token_count, created_at
            FROM chat_messages
            WHERE tool_call_id = $1 AND user_id = $2
            "#,
        )
        .bind(tool_call_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::{ContextSnapshot, ToolCallFunction};

    #[test]
    fn test_chat_repository_new() {
        // Basic compile-time test to verify the API is correct.
        // Full integration tests would require a test database.
    }

    #[test]
    fn test_create_chat_message_input() {
        let user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();

        let input = CreateChatMessage {
            conversation_id,
            user_id,
            role: ChatRole::User,
            content: Some("Hello, AI!".to_string()),
            tool_calls: None,
            tool_call_id: None,
            context_snapshot: None,
            model_used: None,
            token_count: Some(5),
        };

        assert_eq!(input.conversation_id, conversation_id);
        assert_eq!(input.role, ChatRole::User);
        assert!(input.content.is_some());
    }

    #[test]
    fn test_create_conversation_input() {
        let user_id = Uuid::new_v4();

        let input = CreateConversation {
            user_id,
            title: Some("Test Conversation".to_string()),
        };

        assert_eq!(input.user_id, user_id);
        assert_eq!(input.title, Some("Test Conversation".to_string()));
    }

    #[test]
    fn test_create_tool_call_message() {
        let user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();

        let tool_calls = vec![ToolCall {
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: "search_library".to_string(),
                arguments: r#"{"query": "jazz"}"#.to_string(),
            },
        }];

        let input = CreateChatMessage {
            conversation_id,
            user_id,
            role: ChatRole::Assistant,
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            context_snapshot: None,
            model_used: Some("ministral-3:8b".to_string()),
            token_count: Some(50),
        };

        assert!(input.tool_calls.is_some());
        assert_eq!(input.tool_calls.as_ref().unwrap().len(), 1);
        assert_eq!(
            input.tool_calls.as_ref().unwrap()[0].function.name,
            "search_library"
        );
    }

    #[test]
    fn test_create_tool_result_message() {
        let user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();

        let input = CreateChatMessage {
            conversation_id,
            user_id,
            role: ChatRole::Tool,
            content: Some(r#"{"tracks": [{"id": "abc", "title": "Jazz Song"}]}"#.to_string()),
            tool_calls: None,
            tool_call_id: Some("call_123".to_string()),
            context_snapshot: None,
            model_used: None,
            token_count: Some(25),
        };

        assert_eq!(input.role, ChatRole::Tool);
        assert!(input.tool_call_id.is_some());
        assert!(input.content.is_some());
    }

    #[test]
    fn test_context_snapshot_serialization() {
        let ctx = ContextSnapshot {
            track_count: 100,
            artist_count: 50,
            album_count: 25,
            playlist_count: 5,
            top_genres: vec!["jazz".to_string(), "rock".to_string()],
            current_track_id: None,
            current_track_title: None,
        };

        let json = serde_json::to_value(&ctx).expect("serialization should succeed");
        assert_eq!(json["track_count"], 100);
        assert_eq!(json["top_genres"][0], "jazz");
    }
}
