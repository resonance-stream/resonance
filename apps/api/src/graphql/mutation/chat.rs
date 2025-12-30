//! Chat mutations for Resonance GraphQL API
//!
//! This module provides mutations for chat management:
//! - deleteConversation: Delete a chat conversation and its messages
//! - updateConversationTitle: Update a conversation's title
//!
//! Note: Chat messages are created via WebSocket, not GraphQL mutations.
//! These mutations are for managing existing conversations.

use async_graphql::{Context, InputObject, Object, Result, ID};
use uuid::Uuid;

use crate::graphql::types::chat::ChatConversation;
use crate::models::user::Claims;
use crate::repositories::ChatRepository;

// =============================================================================
// Input Validation Limits
// =============================================================================

/// Maximum length of conversation title
const MAX_TITLE_LENGTH: usize = 255;

// =============================================================================
// Input Types
// =============================================================================

/// Input for updating a conversation
#[derive(Debug, InputObject)]
pub struct UpdateConversationInput {
    /// New title for the conversation
    pub title: String,
}

// =============================================================================
// Mutations
// =============================================================================

/// Chat mutations
#[derive(Default)]
pub struct ChatMutation;

#[Object]
impl ChatMutation {
    /// Delete a chat conversation
    ///
    /// Permanently deletes the conversation and all its messages.
    /// Requires authentication and ownership of the conversation.
    ///
    /// # Arguments
    /// * `id` - The conversation ID to delete
    ///
    /// # Returns
    /// True if the conversation was deleted successfully
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if conversation not found
    /// - Returns error if user doesn't own the conversation
    async fn delete_conversation(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let conversation_id: Uuid = id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid conversation ID"))?;

        let repo = ctx.data::<ChatRepository>()?;

        // Delete the conversation (ownership checked by repository, messages cascade)
        let deleted = repo.delete_conversation(conversation_id, claims.sub).await?;

        if !deleted {
            return Err(async_graphql::Error::new("Conversation not found"));
        }

        tracing::info!(
            conversation_id = %conversation_id,
            user_id = %claims.sub,
            "Chat conversation deleted"
        );

        Ok(true)
    }

    /// Update a conversation's title
    ///
    /// Updates the title of an existing conversation.
    /// Requires authentication and ownership of the conversation.
    ///
    /// # Arguments
    /// * `id` - The conversation ID to update
    /// * `input` - The update input with new title
    ///
    /// # Returns
    /// The updated conversation
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if conversation not found
    /// - Returns error if user doesn't own the conversation
    /// - Returns error if title is too long
    async fn update_conversation_title(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateConversationInput,
    ) -> Result<ChatConversation> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let conversation_id: Uuid = id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid conversation ID"))?;

        // Validate title
        let title = input.title.trim();
        if title.is_empty() {
            return Err(async_graphql::Error::new("Title cannot be empty"));
        }
        if title.len() > MAX_TITLE_LENGTH {
            return Err(async_graphql::Error::new(format!(
                "Title cannot exceed {} characters",
                MAX_TITLE_LENGTH
            )));
        }

        let repo = ctx.data::<ChatRepository>()?;

        // Update the title (ownership checked by repository)
        let updated = repo
            .update_conversation_title(conversation_id, claims.sub, title)
            .await?
            .ok_or_else(|| async_graphql::Error::new("Conversation not found"))?;

        Ok(ChatConversation::from(updated))
    }

    /// Delete all conversations for the authenticated user
    ///
    /// Permanently deletes all chat conversations and their messages.
    /// This action cannot be undone.
    ///
    /// # Returns
    /// The number of conversations deleted
    ///
    /// # Errors
    /// - Returns error if not authenticated
    async fn delete_all_conversations(&self, ctx: &Context<'_>) -> Result<i64> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let repo = ctx.data::<ChatRepository>()?;

        let deleted_count = repo.delete_all_user_conversations(claims.sub).await?;

        tracing::info!(
            user_id = %claims.sub,
            deleted_count = deleted_count,
            "All chat conversations deleted"
        );

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_mutation_constructable() {
        let _ = ChatMutation;
    }

    #[test]
    fn test_max_title_length() {
        assert_eq!(MAX_TITLE_LENGTH, 255);
    }
}
