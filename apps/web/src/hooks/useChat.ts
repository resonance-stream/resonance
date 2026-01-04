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
import type { Track } from '../stores/playerStore';
import {
  CREATE_PLAYLIST_MUTATION,
  ADD_TRACKS_TO_PLAYLIST_MUTATION,
} from '../lib/graphql/playlist';
import { SIMILAR_TRACKS_QUERY } from '../lib/graphql/similarity';
import {
  CHAT_CONVERSATIONS_QUERY,
  CHAT_CONVERSATION_QUERY,
  DELETE_CONVERSATION_MUTATION,
  DELETE_ALL_CONVERSATIONS_MUTATION,
  type ChatConversationsResponse,
  type ChatConversationDetailResponse,
} from '../lib/graphql/chat';
import { libraryKeys, chatKeys } from '../lib/queryKeys';
import { mapScoredTrackToPlayerTrack } from '../lib/mappers';
import type {
  CreatePlaylistInput,
  CreatePlaylistResponse,
  AddTracksResponse,
} from '../types/playlist';
import type { SimilarTracksResponse } from '../types/similarity';

// Re-export chatKeys for backwards compatibility
export { chatKeys } from '../lib/queryKeys';

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

  // Type guard for filtering tracks
  const isValidTrack = (track: Track | null): track is Track => track !== null;

  // Execute a chat action (play track, add to queue, etc.)
  const executeAction = useCallback(async (action: ChatAction) => {
    console.log('[Chat] Executing action:', action);

    switch (action.type) {
      case 'play_track': {
        const { track_id: trackId } = action.payload;
        if (!trackId) {
          console.warn('[Chat] play_track action missing track_id');
          break;
        }

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
        const { track_ids: trackIds } = action.payload;
        if (!trackIds?.length) {
          console.warn('[Chat] add_to_queue action has empty track_ids');
          break;
        }

        try {
          // Fetch all tracks in parallel for better performance
          const results = await Promise.all(
            trackIds.map(async (trackId) => {
              try {
                return await fetchTrackById(trackId);
              } catch (err) {
                console.warn('[Chat] Failed to fetch track:', trackId, err);
                return null;
              }
            })
          );

          // Add successfully fetched tracks to queue using type guard
          const validTracks = results.filter(isValidTrack);
          validTracks.forEach((track) => addToQueue(track));
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
        const { name, description, track_ids: trackIds } = action.payload;

        if (!name) {
          console.warn('[Chat] create_playlist action missing name');
          break;
        }

        try {
          // Create a manual playlist with optional description from AI
          const createInput: CreatePlaylistInput = {
            name,
            description,
            isPublic: false,
            playlistType: 'Manual',
          };

          const createResponse = await graphqlClient.request<CreatePlaylistResponse>(
            CREATE_PLAYLIST_MUTATION,
            { input: createInput }
          );

          const playlistId = createResponse.createPlaylist.id;

          // Invalidate playlist queries to refresh the library
          queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.all() });

          // Navigate to the new playlist immediately
          navigate(`/playlist/${playlistId}`);

          // Add tracks to the playlist if provided (best-effort)
          if (trackIds?.length) {
            try {
              await graphqlClient.request<AddTracksResponse>(
                ADD_TRACKS_TO_PLAYLIST_MUTATION,
                {
                  playlistId,
                  input: { trackIds },
                }
              );
            } catch (err) {
              const errorMsg = err instanceof Error ? err.message : String(err);
              console.error('[Chat] Failed to add tracks to playlist:', err);
              handleError({
                conversation_id: useChatStore.getState().conversationId,
                error: `Playlist created, but failed to add tracks: ${errorMsg}`,
              });
            }
          }
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
      case 'show_search': {
        const { query, result_type } = action.payload;
        if (!query) {
          console.warn('[Chat] show_search action missing query');
          break;
        }
        // Include result_type as a filter parameter if provided
        const searchParams = new URLSearchParams({ q: query });
        if (result_type) {
          searchParams.set('type', result_type);
        }
        navigate(`/search?${searchParams.toString()}`);
        break;
      }
      case 'get_recommendations': {
        const { track_id: trackId } = action.payload;
        if (!trackId) {
          console.warn('[Chat] get_recommendations action missing track_id');
          break;
        }

        try {
          // Fetch seed track and similar tracks in parallel for better performance
          const [seedTrack, similarResponse] = await Promise.all([
            fetchTrackById(trackId),
            graphqlClient.request<SimilarTracksResponse>(
              SIMILAR_TRACKS_QUERY,
              { trackId, limit: 15 }
            ),
          ]);

          const similarTracks = similarResponse.similarTracks ?? [];

          // Build queue with seed track first, then similar tracks
          const queueTracks: Track[] = [];

          if (seedTrack) {
            queueTracks.push(seedTrack);
          }

          if (similarTracks.length > 0) {
            const recommendedTracks = similarTracks.map(mapScoredTrackToPlayerTrack);
            queueTracks.push(...recommendedTracks);
          }

          if (queueTracks.length === 0) {
            console.log('[Chat] No tracks available for recommendations:', trackId);
            break;
          }

          // Set queue starting at the seed track (or first recommendation if seed failed)
          setQueue(queueTracks, 0);
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
      default: {
        // This should never happen with discriminated unions, but handle gracefully
        const _exhaustiveCheck: never = action;
        console.warn('[Chat] Unknown action type:', _exhaustiveCheck);
      }
    }
  }, [setTrack, addToQueue, setQueue, navigate, handleError, queryClient]);

  // Set up WebSocket connection with chat handlers
  const { isConnected, sendChatMessage } = useSyncConnection({
    onChatToken: useCallback((payload: ChatTokenPayload) => {
      receiveToken(payload);
    }, [receiveToken]),

    onChatComplete: useCallback((payload: ChatCompletePayload) => {
      completeResponse(payload);
      // Execute any actions sequentially with proper error handling
      // Each action is wrapped in try-catch so one failure doesn't stop subsequent actions
      if (payload.actions?.length) {
        (async () => {
          for (const action of payload.actions) {
            try {
              await executeAction(action);
            } catch (err) {
              // Log the error but continue with remaining actions
              console.error('[Chat] Action execution failed, continuing with next action:', action.type, err);
            }
          }
        })().catch((err) => {
          // This handles any truly unexpected errors in the loop itself
          console.error('[Chat] Unhandled error in action execution loop:', err);
        });
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
