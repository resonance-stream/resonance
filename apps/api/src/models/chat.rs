//! Chat models for AI assistant functionality
//!
//! This module contains the database models for AI chat conversations
//! and messages, including support for function calling via tool_calls.

// Allow dead_code - these types are prepared for Phase 3 (Chat Service) implementation
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Chat message role enum matching PostgreSQL role column
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    User,
    Assistant,
    System,
    Tool,
}

impl ChatRole {
    /// Returns the string representation of the role
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::System => "system",
            ChatRole::Tool => "tool",
        }
    }
}

impl std::fmt::Display for ChatRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single tool call request from the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool call (used to match with tool result)
    pub id: String,
    /// Type of tool (always "function" for now)
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function to call
    pub function: ToolCallFunction,
}

/// Function details within a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    /// Name of the function to call
    pub name: String,
    /// JSON string of function arguments
    pub arguments: String,
}

/// User context snapshot captured at message time
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// Total tracks in user's library
    #[serde(default)]
    pub track_count: i64,
    /// Total artists in user's library
    #[serde(default)]
    pub artist_count: i64,
    /// Total albums in user's library
    #[serde(default)]
    pub album_count: i64,
    /// Total playlists owned by user
    #[serde(default)]
    pub playlist_count: i64,
    /// User's top genres
    #[serde(default)]
    pub top_genres: Vec<String>,
    /// Currently playing track ID
    pub current_track_id: Option<Uuid>,
    /// Currently playing track title
    pub current_track_title: Option<String>,
}

/// Chat conversation record from the chat_conversations table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatConversation {
    /// Unique conversation identifier
    pub id: Uuid,

    /// User who owns this conversation
    pub user_id: Uuid,

    /// Conversation title (auto-generated or user-set)
    pub title: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Soft delete timestamp (None if not deleted)
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Chat message record from the chat_messages table
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatMessage {
    /// Unique message identifier
    pub id: Uuid,

    /// Conversation this message belongs to
    pub conversation_id: Uuid,

    /// User who owns this message
    pub user_id: Uuid,

    /// Message role (user, assistant, system, tool)
    pub role: ChatRole,

    /// Message content (can be None for tool call messages)
    pub content: Option<String>,

    /// Message sequence within conversation
    pub sequence_number: i32,

    /// Tool calls made by assistant (for function calling)
    #[sqlx(json)]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Tool call ID (for tool result messages)
    pub tool_call_id: Option<String>,

    /// User context at time of message
    #[sqlx(json)]
    pub context_snapshot: Option<ContextSnapshot>,

    /// Model used for generation
    pub model_used: Option<String>,

    /// Token count for this message
    pub token_count: Option<i32>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl ChatMessage {
    /// Returns true if this is a tool call message (assistant with tool_calls)
    pub fn is_tool_call(&self) -> bool {
        self.role == ChatRole::Assistant && self.tool_calls.is_some()
    }

    /// Returns true if this is a tool result message
    pub fn is_tool_result(&self) -> bool {
        self.role == ChatRole::Tool && self.tool_call_id.is_some()
    }
}

/// Input for creating a new chat message
#[derive(Debug, Clone)]
pub struct CreateChatMessage {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub role: ChatRole,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub context_snapshot: Option<ContextSnapshot>,
    pub model_used: Option<String>,
    pub token_count: Option<i32>,
}

/// Input for creating a new conversation
#[derive(Debug, Clone)]
pub struct CreateConversation {
    pub user_id: Uuid,
    pub title: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_role_as_str() {
        assert_eq!(ChatRole::User.as_str(), "user");
        assert_eq!(ChatRole::Assistant.as_str(), "assistant");
        assert_eq!(ChatRole::System.as_str(), "system");
        assert_eq!(ChatRole::Tool.as_str(), "tool");
    }

    #[test]
    fn test_tool_call_serialization() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: "search_tracks".to_string(),
                arguments: r#"{"query": "jazz"}"#.to_string(),
            },
        };

        let json = serde_json::to_string(&tool_call).expect("serialization should succeed");
        assert!(json.contains("call_123"));
        assert!(json.contains("search_tracks"));
    }

    #[test]
    fn test_context_snapshot_default() {
        let ctx = ContextSnapshot::default();
        assert_eq!(ctx.track_count, 0);
        assert_eq!(ctx.artist_count, 0);
        assert!(ctx.top_genres.is_empty());
        assert!(ctx.current_track_id.is_none());
    }

    #[test]
    fn test_chat_message_helpers() {
        let mut msg = create_test_message();

        // Regular user message
        assert!(!msg.is_tool_call());
        assert!(!msg.is_tool_result());

        // Tool call message
        msg.role = ChatRole::Assistant;
        msg.tool_calls = Some(vec![ToolCall {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: "test".to_string(),
                arguments: "{}".to_string(),
            },
        }]);
        assert!(msg.is_tool_call());
        assert!(!msg.is_tool_result());

        // Tool result message
        msg.role = ChatRole::Tool;
        msg.tool_calls = None;
        msg.tool_call_id = Some("call_1".to_string());
        assert!(!msg.is_tool_call());
        assert!(msg.is_tool_result());
    }

    fn create_test_message() -> ChatMessage {
        ChatMessage {
            id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            role: ChatRole::User,
            content: Some("Hello".to_string()),
            sequence_number: 0,
            tool_calls: None,
            tool_call_id: None,
            context_snapshot: None,
            model_used: None,
            token_count: None,
            created_at: Utc::now(),
        }
    }
}
