//! Chat GraphQL types
//!
//! This module defines the GraphQL types for chat conversations and messages.

use async_graphql::{Object, SimpleObject};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::chat::{
    ChatConversation as DbChatConversation, ChatMessage as DbChatMessage, ChatRole as DbChatRole,
};

// =============================================================================
// Enums
// =============================================================================

/// Role of a message sender in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, async_graphql::Enum)]
pub enum ChatRole {
    /// Message from the user
    User,
    /// Message from the AI assistant
    Assistant,
    /// System message (context, instructions)
    System,
    /// Tool result message
    Tool,
}

impl From<DbChatRole> for ChatRole {
    fn from(role: DbChatRole) -> Self {
        match role {
            DbChatRole::User => ChatRole::User,
            DbChatRole::Assistant => ChatRole::Assistant,
            DbChatRole::System => ChatRole::System,
            DbChatRole::Tool => ChatRole::Tool,
        }
    }
}

// =============================================================================
// Chat Conversation
// =============================================================================

/// A chat conversation with the AI assistant
pub struct ChatConversation {
    inner: DbChatConversation,
}

impl ChatConversation {
    /// Create a new GraphQL ChatConversation from a database model
    pub fn new(conversation: DbChatConversation) -> Self {
        Self {
            inner: conversation,
        }
    }
}

impl From<DbChatConversation> for ChatConversation {
    fn from(conversation: DbChatConversation) -> Self {
        Self::new(conversation)
    }
}

#[Object]
impl ChatConversation {
    /// Unique conversation identifier
    async fn id(&self) -> Uuid {
        self.inner.id
    }

    /// User who owns this conversation
    async fn user_id(&self) -> Uuid {
        self.inner.user_id
    }

    /// Conversation title (auto-generated or user-set)
    async fn title(&self) -> &str {
        &self.inner.title
    }

    /// Creation timestamp
    async fn created_at(&self) -> DateTime<Utc> {
        self.inner.created_at
    }

    /// Last update timestamp
    async fn updated_at(&self) -> DateTime<Utc> {
        self.inner.updated_at
    }
}

// =============================================================================
// Chat Message
// =============================================================================

/// A message in a chat conversation
pub struct ChatMessage {
    inner: DbChatMessage,
}

impl ChatMessage {
    /// Create a new GraphQL ChatMessage from a database model
    pub fn new(message: DbChatMessage) -> Self {
        Self { inner: message }
    }
}

impl From<DbChatMessage> for ChatMessage {
    fn from(message: DbChatMessage) -> Self {
        Self::new(message)
    }
}

#[Object]
impl ChatMessage {
    /// Unique message identifier
    async fn id(&self) -> Uuid {
        self.inner.id
    }

    /// Conversation this message belongs to
    async fn conversation_id(&self) -> Uuid {
        self.inner.conversation_id
    }

    /// User who owns this message
    async fn user_id(&self) -> Uuid {
        self.inner.user_id
    }

    /// Role of the message sender (user, assistant, system)
    async fn role(&self) -> ChatRole {
        self.inner.role.into()
    }

    /// Message content (may be None for tool-only messages)
    async fn content(&self) -> Option<&str> {
        self.inner.content.as_deref()
    }

    /// AI model used to generate this message (if assistant)
    async fn model_used(&self) -> Option<&str> {
        self.inner.model_used.as_deref()
    }

    /// Token count for this message
    async fn token_count(&self) -> Option<i32> {
        self.inner.token_count
    }

    /// Creation timestamp
    async fn created_at(&self) -> DateTime<Utc> {
        self.inner.created_at
    }
}

// =============================================================================
// Conversation with Messages (for detail queries)
// =============================================================================

/// A conversation with its messages loaded
#[derive(SimpleObject)]
pub struct ChatConversationWithMessages {
    /// The conversation
    pub conversation: ChatConversation,
    /// Messages in the conversation (in chronological order)
    pub messages: Vec<ChatMessage>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_role_conversion() {
        assert_eq!(ChatRole::from(DbChatRole::User), ChatRole::User);
        assert_eq!(ChatRole::from(DbChatRole::Assistant), ChatRole::Assistant);
        assert_eq!(ChatRole::from(DbChatRole::System), ChatRole::System);
        assert_eq!(ChatRole::from(DbChatRole::Tool), ChatRole::Tool);
    }
}
