//! Chat queries for Resonance GraphQL API
//!
//! This module provides queries for chat history:
//! - chatConversations: List user's chat conversations
//! - chatConversation: Get a specific conversation with messages
//! - chatMessages: Get messages for a conversation

use async_graphql::{Context, Object, Result, ID};
use uuid::Uuid;

use crate::graphql::pagination::{clamp_limit, clamp_offset, MAX_LIMIT};
use crate::graphql::types::chat::{ChatConversation, ChatConversationWithMessages, ChatMessage};
use crate::models::user::Claims;
use crate::repositories::ChatRepository;

/// Maximum number of messages to return per query
const MAX_MESSAGES: i32 = 100;

/// Chat-related queries
#[derive(Default)]
pub struct ChatQuery;

#[Object]
impl ChatQuery {
    /// List the authenticated user's chat conversations
    ///
    /// Returns conversations in reverse chronological order (most recent first).
    /// Requires authentication.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of conversations to return (default: 20, max: 100)
    /// * `offset` - Number of conversations to skip (default: 0)
    ///
    /// # Returns
    /// List of chat conversations
    async fn chat_conversations(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<ChatConversation>> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let repo = ctx.data::<ChatRepository>()?;
        let conversations = repo
            .find_conversations_by_user(
                claims.sub,
                clamp_limit(limit, MAX_LIMIT),
                clamp_offset(offset),
            )
            .await?;

        Ok(conversations.into_iter().map(ChatConversation::from).collect())
    }

    /// Get a specific chat conversation with its messages
    ///
    /// Returns the conversation and all its messages in chronological order.
    /// Requires authentication and ownership of the conversation.
    ///
    /// # Arguments
    /// * `id` - The conversation ID
    /// * `message_limit` - Maximum messages to return (default: 50, max: 100)
    ///
    /// # Returns
    /// The conversation with messages, or None if not found
    async fn chat_conversation(
        &self,
        ctx: &Context<'_>,
        id: ID,
        #[graphql(default = 50)] message_limit: i32,
    ) -> Result<Option<ChatConversationWithMessages>> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let conversation_id: Uuid = id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid conversation ID"))?;

        let repo = ctx.data::<ChatRepository>()?;

        // Find the conversation (ownership checked by repository)
        let conversation = match repo.find_conversation_by_id(conversation_id, claims.sub).await? {
            Some(c) => c,
            None => return Ok(None),
        };

        // Get messages for the conversation
        let messages = repo
            .get_messages(
                conversation_id,
                claims.sub,
                clamp_limit(message_limit, MAX_MESSAGES),
            )
            .await?;

        Ok(Some(ChatConversationWithMessages {
            conversation: ChatConversation::from(conversation),
            messages: messages.into_iter().map(ChatMessage::from).collect(),
        }))
    }

    /// Get messages for a specific conversation
    ///
    /// Returns messages in chronological order.
    /// Requires authentication and ownership of the conversation.
    ///
    /// # Arguments
    /// * `conversation_id` - The conversation ID
    /// * `limit` - Maximum messages to return (default: 50, max: 100)
    ///
    /// # Returns
    /// List of messages, or error if conversation not found/unauthorized
    async fn chat_messages(
        &self,
        ctx: &Context<'_>,
        conversation_id: ID,
        #[graphql(default = 50)] limit: i32,
    ) -> Result<Vec<ChatMessage>> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let conversation_id: Uuid = conversation_id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid conversation ID"))?;

        let repo = ctx.data::<ChatRepository>()?;

        // Verify conversation exists and user owns it (ownership checked by repository)
        let _conversation = repo
            .find_conversation_by_id(conversation_id, claims.sub)
            .await?
            .ok_or_else(|| async_graphql::Error::new("Conversation not found"))?;

        let messages = repo
            .get_messages(
                conversation_id,
                claims.sub,
                clamp_limit(limit, MAX_MESSAGES),
            )
            .await?;

        Ok(messages.into_iter().map(ChatMessage::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_query_constructable() {
        let _ = ChatQuery;
    }
}
