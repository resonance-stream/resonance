/**
 * Chat GraphQL Operations
 *
 * GraphQL queries and mutations for AI chat functionality.
 * Maps to backend schema in apps/api/src/graphql/query/chat.rs
 */

import { gql } from 'graphql-request'

// ============================================================================
// Queries
// ============================================================================

/**
 * Get list of chat conversations for the current user
 */
export const CHAT_CONVERSATIONS_QUERY = gql`
  query ChatConversations($limit: Int, $offset: Int) {
    chatConversations(limit: $limit, offset: $offset) {
      id
      title
      createdAt
      updatedAt
    }
  }
`

/**
 * Get a single conversation with its messages
 */
export const CHAT_CONVERSATION_QUERY = gql`
  query ChatConversation($id: ID!, $messageLimit: Int) {
    chatConversation(id: $id, messageLimit: $messageLimit) {
      conversation {
        id
        title
        createdAt
        updatedAt
      }
      messages {
        id
        conversationId
        role
        content
        modelUsed
        tokenCount
        createdAt
      }
    }
  }
`

// ============================================================================
// Mutations
// ============================================================================

/**
 * Delete a single conversation
 */
export const DELETE_CONVERSATION_MUTATION = gql`
  mutation DeleteConversation($id: ID!) {
    deleteConversation(id: $id)
  }
`

/**
 * Delete all conversations for the current user
 */
export const DELETE_ALL_CONVERSATIONS_MUTATION = gql`
  mutation DeleteAllConversations {
    deleteAllConversations
  }
`

// ============================================================================
// Types
// ============================================================================

/**
 * Chat conversation from GraphQL response
 */
export interface ChatConversationResponse {
  id: string
  title: string
  createdAt: string
  updatedAt: string
}

/**
 * Chat message from GraphQL response
 */
export interface ChatMessageResponse {
  id: string
  conversationId: string
  role: 'USER' | 'ASSISTANT' | 'SYSTEM' | 'TOOL'
  content: string | null
  modelUsed: string | null
  tokenCount: number | null
  createdAt: string
}

/**
 * Response for chat conversations list query
 */
export interface ChatConversationsResponse {
  chatConversations: ChatConversationResponse[]
}

/**
 * Response for single conversation detail query
 */
export interface ChatConversationDetailResponse {
  chatConversation: {
    conversation: ChatConversationResponse
    messages: ChatMessageResponse[]
  } | null
}
