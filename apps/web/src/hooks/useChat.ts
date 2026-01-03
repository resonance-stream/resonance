/**
 * Chat hook for AI assistant integration
 *
 * Provides chat functionality including:
 * - Sending messages via WebSocket
 * - Receiving streaming responses
 * - Loading conversation history
 * - Executing AI-suggested actions
 */

import { useCallback, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { graphqlClient } from '../lib/api';
import {
  useChatStore,
  createUserMessage,
  fromServerMessage,
  type ChatMessage,
  type ChatConversation,
  type ChatError,
} from '../stores/chatStore';
import { useSyncConnection } from '../sync/useSyncConnection';
import type {
  ChatAction,
  ChatSendPayload,
  ChatTokenPayload,
  ChatCompletePayload,
  ChatErrorPayload,
} from '../sync/types';
import { usePlayerStore } from '../stores/playerStore';
import { fetchTrackById } from '../sync/fetchTrackById';
import {
  CREATE_PLAYLIST_MUTATION,
  ADD_TRACKS_TO_PLAYLIST_MUTATION,
} from '../lib/graphql/playlist';
import { SIMILAR_TRACKS_QUERY } from '../lib/graphql/similarity';
import { libraryKeys } from '../lib/queryKeys';
import { mapScoredTrackToPlayerTrack } from '../lib/mappers';
import type {
  CreatePlaylistInput,
  CreatePlaylistResponse,
  AddTracksResponse,
} from '../types/playlist';
import type { SimilarTracksResponse } from '../types/similarity';

// =============================================================================
// GraphQL Queries
// =============================================================================

const CHAT_CONVERSATIONS_QUERY = `
  query ChatConversations($limit: Int, $offset: Int) {
    chatConversations(limit: $limit, offset: $offset) {
      id
      title
      createdAt
      updatedAt
    }
  }
`;

const CHAT_CONVERSATION_QUERY = `
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
`;

const DELETE_CONVERSATION_MUTATION = `
  mutation DeleteConversation($id: ID!) {
    deleteConversation(id: $id)
  }
`;

const DELETE_ALL_CONVERSATIONS_MUTATION = `
  mutation DeleteAllConversations {
    deleteAllConversations
  }
`;

// =============================================================================
// Query Keys
// =============================================================================

export const chatKeys = {
  all: ['chat'] as const,
  conversations: () => [...chatKeys.all, 'conversations'] as const,
  conversation: (id: string) => [...chatKeys.all, 'conversation', id] as const,
};

// =============================================================================
// Types
// =============================================================================

interface ChatConversationResponse {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
}

interface ChatMessageResponse {
  id: string;
  conversationId: string;
  role: 'USER' | 'ASSISTANT' | 'SYSTEM' | 'TOOL';
  content: string | null;
  modelUsed: string | null;
  tokenCount: number | null;
  createdAt: string;
}

interface ChatConversationsResponse {
  chatConversations: ChatConversationResponse[];
}

interface ChatConversationDetailResponse {
  chatConversation: {
    conversation: ChatConversationResponse;
    messages: ChatMessageResponse[];
  } | null;
}

// =============================================================================
// Hook
// =============================================================================

export interface UseChatOptions {
  /** Whether to auto-load conversations on mount */
  autoLoadConversations?: boolean;
}

export interface UseChatReturn {
  // State
  isOpen: boolean;
  messages: ChatMessage[];
  conversationId: string | null;
  conversations: ChatConversation[];
  isStreaming: boolean;
  streamingContent: string;
  inputValue: string;
  error: ChatError | null;
  isConnected: boolean;

  // Actions
  openChat: () => void;
  closeChat: () => void;
  toggleChat: () => void;
  setInputValue: (value: string) => void;
  sendMessage: (message: string) => void;
  selectConversation: (id: string | null) => void;
  deleteConversation: (id: string) => Promise<void>;
  deleteAllConversations: () => Promise<void>;
  startNewConversation: () => void;
  clearError: () => void;

  // Loading states
  isLoadingConversations: boolean;
  isLoadingMessages: boolean;
}

/**
 * Hook for chat functionality
 *
 * Integrates chatStore with WebSocket sync and GraphQL queries
 */
export function useChat(options: UseChatOptions = {}): UseChatReturn {
  const { autoLoadConversations = true } = options;
  const queryClient = useQueryClient();

  // Store state
  const isOpen = useChatStore((s) => s.isOpen);
  const messages = useChatStore((s) => s.messages);
  const conversationId = useChatStore((s) => s.conversationId);
  const conversations = useChatStore((s) => s.conversations);
  const status = useChatStore((s) => s.status);
  const streamingContent = useChatStore((s) => s.streamingContent);
  const inputValue = useChatStore((s) => s.inputValue);
  const error = useChatStore((s) => s.error);

  // Store actions
  const openChat = useChatStore((s) => s.openChat);
  const closeChat = useChatStore((s) => s.closeChat);
  const toggleChat = useChatStore((s) => s.toggleChat);
  const setInputValue = useChatStore((s) => s.setInputValue);
  const setConversation = useChatStore((s) => s.setConversation);
  const setConversations = useChatStore((s) => s.setConversations);
  const removeConversation = useChatStore((s) => s.removeConversation);
  const addMessage = useChatStore((s) => s.addMessage);
  const setMessages = useChatStore((s) => s.setMessages);
  const startStreaming = useChatStore((s) => s.startStreaming);
  const receiveToken = useChatStore((s) => s.receiveToken);
  const completeResponse = useChatStore((s) => s.completeResponse);
  const handleError = useChatStore((s) => s.handleError);
  const clearError = useChatStore((s) => s.clearError);
  const setStatus = useChatStore((s) => s.setStatus);

  // Player store for action execution
  const setTrack = usePlayerStore((s) => s.setTrack);
  const addToQueue = usePlayerStore((s) => s.addToQueue);
  const setQueue = usePlayerStore((s) => s.setQueue);

  // Navigation for action execution
  const navigate = useNavigate();

  // Execute a chat action (play track, add to queue, etc.)
  const executeAction = useCallback(async (action: ChatAction) => {
    console.log('[Chat] Executing action:', action);

    switch (action.type) {
      case 'play_track': {
        const trackId = action.payload.track_id as string | undefined;
        if (!trackId) break;

        try {
          const track = await fetchTrackById(trackId);
          if (track) {
            setTrack(track);
          }
        } catch (err) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          console.error('[Chat] Failed to play track:', err);
          handleError({
            conversation_id: useChatStore.getState().conversationId,
            error: `Failed to play track: ${errorMsg}`,
          });
        }
        break;
      }
      case 'add_to_queue': {
        const trackIds = action.payload.track_ids as string[] | undefined;
        if (!trackIds?.length) break;

        try {
          // Fetch all tracks in parallel for better performance
          const tracks = await Promise.all(
            trackIds.map(async (trackId) => {
              try {
                return await fetchTrackById(trackId);
              } catch (err) {
                console.warn('[Chat] Failed to fetch track:', trackId, err);
                return null;
              }
            })
          );

          // Add successfully fetched tracks to queue
          tracks.filter(Boolean).forEach((track) => addToQueue(track!));
        } catch (err) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          console.error('[Chat] Failed to add to queue:', err);
          handleError({
            conversation_id: useChatStore.getState().conversationId,
            error: `Failed to add tracks to queue: ${errorMsg}`,
          });
        }
        break;
      }
      case 'create_playlist': {
        const name = action.payload.name as string | undefined;
        const trackIds = action.payload.track_ids as string[] | undefined;

        if (!name) {
          console.warn('[Chat] create_playlist action missing name');
          break;
        }

        try {
          // Create a manual playlist
          const createInput: CreatePlaylistInput = {
            name,
            isPublic: false,
            playlistType: 'Manual',
          };

          const createResponse = await graphqlClient.request<CreatePlaylistResponse>(
            CREATE_PLAYLIST_MUTATION,
            { input: createInput }
          );

          const playlistId = createResponse.createPlaylist.id;

          // Add tracks to the playlist if provided
          if (trackIds?.length) {
            await graphqlClient.request<AddTracksResponse>(
              ADD_TRACKS_TO_PLAYLIST_MUTATION,
              {
                playlistId,
                input: { trackIds },
              }
            );
          }

          // Invalidate playlist queries to refresh the library
          queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.all() });

          // Navigate to the new playlist
          navigate(`/playlist/${playlistId}`);
        } catch (err) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          console.error('[Chat] Failed to create playlist:', err);
          handleError({
            conversation_id: useChatStore.getState().conversationId,
            error: `Failed to create playlist: ${errorMsg}`,
          });
        }
        break;
      }
      case 'search_library': {
        const query = action.payload.query as string | undefined;
        if (query) {
          navigate(`/search?q=${encodeURIComponent(query)}`);
        }
        break;
      }
      case 'get_recommendations': {
        const trackId = action.payload.track_id as string | undefined;
        if (!trackId) break;

        try {
          // Fetch similar tracks using the combined similarity algorithm
          const response = await graphqlClient.request<SimilarTracksResponse>(
            SIMILAR_TRACKS_QUERY,
            { trackId, limit: 15 }
          );

          const similarTracks = response.similarTracks ?? [];
          if (similarTracks.length === 0) {
            console.log('[Chat] No similar tracks found for', trackId);
            break;
          }

          // Convert to player tracks and set as queue, starting playback
          const playerTracks = similarTracks.map(mapScoredTrackToPlayerTrack);
          setQueue(playerTracks, 0);
        } catch (err) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          console.error('[Chat] Failed to get recommendations:', err);
          handleError({
            conversation_id: useChatStore.getState().conversationId,
            error: `Failed to get recommendations: ${errorMsg}`,
          });
        }
        break;
      }
      default:
        console.warn('[Chat] Unknown action type:', action);
    }
  }, [setTrack, addToQueue, setQueue, navigate, handleError, queryClient]);

  // Set up WebSocket connection with chat handlers
  const { isConnected, sendChatMessage } = useSyncConnection({
    onChatToken: useCallback((payload: ChatTokenPayload) => {
      receiveToken(payload);
    }, [receiveToken]),

    onChatComplete: useCallback((payload: ChatCompletePayload) => {
      completeResponse(payload);
      // Execute any actions sequentially
      if (payload.actions?.length) {
        (async () => {
          for (const action of payload.actions) {
            await executeAction(action);
          }
        })();
      }
    }, [completeResponse, executeAction]),

    onChatError: useCallback((payload: ChatErrorPayload) => {
      handleError(payload);
    }, [handleError]),
  });

  // Query: Load conversations list
  const conversationsQuery = useQuery({
    queryKey: chatKeys.conversations(),
    queryFn: async () => {
      const response = await graphqlClient.request<ChatConversationsResponse>(
        CHAT_CONVERSATIONS_QUERY,
        { limit: 50, offset: 0 }
      );
      return response.chatConversations;
    },
    enabled: autoLoadConversations && isOpen,
    staleTime: 30 * 1000, // 30 seconds
  });

  // Update store when conversations are loaded
  useEffect(() => {
    if (conversationsQuery.data) {
      setConversations(
        conversationsQuery.data.map((c) => ({
          id: c.id,
          title: c.title,
          createdAt: c.createdAt,
          updatedAt: c.updatedAt,
        }))
      );
    }
  }, [conversationsQuery.data, setConversations]);

  // Query: Load conversation messages when selected
  const messagesQuery = useQuery({
    queryKey: chatKeys.conversation(conversationId ?? ''),
    queryFn: async () => {
      if (!conversationId) return null;
      const response = await graphqlClient.request<ChatConversationDetailResponse>(
        CHAT_CONVERSATION_QUERY,
        { id: conversationId, messageLimit: 100 }
      );
      return response.chatConversation;
    },
    enabled: Boolean(conversationId),
    staleTime: 10 * 1000, // 10 seconds
  });

  // Update store when messages are loaded
  useEffect(() => {
    if (messagesQuery.data?.messages) {
      setMessages(
        messagesQuery.data.messages.map((m) => fromServerMessage({
          id: m.id,
          conversation_id: m.conversationId,
          role: m.role.toLowerCase() as 'user' | 'assistant' | 'system' | 'tool',
          content: m.content,
          model_used: m.modelUsed ?? undefined,
          token_count: m.tokenCount ?? undefined,
          created_at: m.createdAt,
        }))
      );
    }
  }, [messagesQuery.data, setMessages]);

  // Mutation: Delete conversation
  const deleteConversationMutation = useMutation({
    mutationFn: async (id: string) => {
      await graphqlClient.request(DELETE_CONVERSATION_MUTATION, { id });
      return id;
    },
    onSuccess: (id) => {
      removeConversation(id);
      queryClient.invalidateQueries({ queryKey: chatKeys.conversations() });
    },
  });

  // Mutation: Delete all conversations
  const deleteAllMutation = useMutation({
    mutationFn: async () => {
      await graphqlClient.request(DELETE_ALL_CONVERSATIONS_MUTATION);
    },
    onSuccess: () => {
      useChatStore.getState().clearConversations();
      queryClient.invalidateQueries({ queryKey: chatKeys.conversations() });
    },
  });

  // Send a message
  const sendMessage = useCallback((message: string) => {
    const trimmedMessage = message.trim();
    if (!trimmedMessage || !isConnected) return;

    // Get current conversation ID (may be null for new conversation)
    const currentConversationId = useChatStore.getState().conversationId;

    // Optimistically add user message to UI
    const userMessage = createUserMessage(
      currentConversationId ?? 'pending',
      trimmedMessage
    );
    addMessage(userMessage);

    // Start streaming state
    startStreaming(currentConversationId ?? 'pending');

    // Send via WebSocket
    const payload: ChatSendPayload = {
      conversation_id: currentConversationId,
      message: trimmedMessage,
    };
    sendChatMessage(payload);
  }, [isConnected, addMessage, startStreaming, sendChatMessage]);

  // Select a conversation
  const selectConversation = useCallback((id: string | null) => {
    setConversation(id);
    // Messages will be loaded automatically by the query
  }, [setConversation]);

  // Start a new conversation
  const startNewConversation = useCallback(() => {
    setConversation(null);
    setInputValue('');
    setStatus('idle');
  }, [setConversation, setInputValue, setStatus]);

  return {
    // State
    isOpen,
    messages,
    conversationId,
    conversations,
    isStreaming: status === 'streaming',
    streamingContent,
    inputValue,
    error,
    isConnected,

    // Actions
    openChat,
    closeChat,
    toggleChat,
    setInputValue,
    sendMessage,
    selectConversation,
    deleteConversation: async (id: string) => {
      await deleteConversationMutation.mutateAsync(id);
    },
    deleteAllConversations: async () => {
      await deleteAllMutation.mutateAsync();
    },
    startNewConversation,
    clearError,

    // Loading states
    isLoadingConversations: conversationsQuery.isLoading,
    isLoadingMessages: messagesQuery.isLoading,
  };
}
