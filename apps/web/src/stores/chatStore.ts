/**
 * Chat store for Resonance
 *
 * Manages AI chat assistant state including:
 * - Conversations and messages
 * - Streaming response state
 * - Panel visibility
 * - WebSocket message handling
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type {
  ChatRole,
  ChatMessageData,
  ChatTokenPayload,
  ChatCompletePayload,
  ChatErrorPayload,
} from '../sync/types';

// =============================================================================
// Types
// =============================================================================

/** Local message representation */
export interface ChatMessage {
  id: string;
  conversationId: string;
  role: ChatRole;
  content: string | null;
  modelUsed?: string;
  tokenCount?: number;
  createdAt: string;
  /** Whether this is a streaming message still being received */
  isStreaming?: boolean;
}

/** Conversation summary */
export interface ChatConversation {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  /** Preview of the last message */
  lastMessagePreview?: string;
}

/** Chat panel status */
export type ChatStatus = 'idle' | 'loading' | 'streaming' | 'error';

/** Chat error */
export interface ChatError {
  message: string;
  code?: string;
}

// =============================================================================
// State Interface
// =============================================================================

interface ChatState {
  // UI State
  /** Whether the chat panel is open */
  isOpen: boolean;
  /** Current input value */
  inputValue: string;
  /** Chat status */
  status: ChatStatus;
  /** Last error */
  error: ChatError | null;

  // Conversation State
  /** Current active conversation ID */
  conversationId: string | null;
  /** List of user's conversations */
  conversations: ChatConversation[];
  /** Messages in the current conversation */
  messages: ChatMessage[];
  /** Streaming content being received (for the current assistant response) */
  streamingContent: string;

  // Actions - Panel
  openChat: () => void;
  closeChat: () => void;
  toggleChat: () => void;
  setInputValue: (value: string) => void;

  // Actions - Conversations
  setConversation: (conversationId: string | null) => void;
  setConversations: (conversations: ChatConversation[]) => void;
  addConversation: (conversation: ChatConversation) => void;
  updateConversation: (id: string, updates: Partial<ChatConversation>) => void;
  removeConversation: (id: string) => void;
  clearConversations: () => void;

  // Actions - Messages
  addMessage: (message: ChatMessage) => void;
  setMessages: (messages: ChatMessage[]) => void;
  clearMessages: () => void;

  // Actions - Streaming (called from WebSocket handler)
  /** Start a new streaming response */
  startStreaming: (conversationId: string) => void;
  /** Receive a streaming token */
  receiveToken: (payload: ChatTokenPayload) => void;
  /** Complete streaming response */
  completeResponse: (payload: ChatCompletePayload) => void;
  /** Handle chat error */
  handleError: (payload: ChatErrorPayload) => void;

  // Actions - Status
  setStatus: (status: ChatStatus) => void;
  clearError: () => void;

  // Actions - Reset
  reset: () => void;
}

// =============================================================================
// Initial State
// =============================================================================

const initialState = {
  isOpen: false,
  inputValue: '',
  status: 'idle' as ChatStatus,
  error: null,
  conversationId: null,
  conversations: [],
  messages: [],
  streamingContent: '',
};

// =============================================================================
// Store
// =============================================================================

export const useChatStore = create<ChatState>()(
  persist(
    (set, get) => ({
      // Initial state
      ...initialState,

      // =========================================================================
      // Panel Actions
      // =========================================================================

      openChat: () => {
        set({ isOpen: true });
      },

      closeChat: () => {
        set({ isOpen: false });
      },

      toggleChat: () => {
        set((state) => ({ isOpen: !state.isOpen }));
      },

      setInputValue: (value: string) => {
        set({ inputValue: value });
      },

      // =========================================================================
      // Conversation Actions
      // =========================================================================

      setConversation: (conversationId: string | null) => {
        set({
          conversationId,
          messages: [],
          streamingContent: '',
          status: 'idle',
          error: null,
        });
      },

      setConversations: (conversations: ChatConversation[]) => {
        set({ conversations });
      },

      addConversation: (conversation: ChatConversation) => {
        set((state) => ({
          conversations: [conversation, ...state.conversations],
        }));
      },

      updateConversation: (id: string, updates: Partial<ChatConversation>) => {
        set((state) => ({
          conversations: state.conversations.map((c) =>
            c.id === id ? { ...c, ...updates } : c
          ),
        }));
      },

      removeConversation: (id: string) => {
        const { conversationId } = get();
        set((state) => ({
          conversations: state.conversations.filter((c) => c.id !== id),
          // Clear current conversation if it was removed
          ...(conversationId === id
            ? { conversationId: null, messages: [], streamingContent: '' }
            : {}),
        }));
      },

      clearConversations: () => {
        set({
          conversations: [],
          conversationId: null,
          messages: [],
          streamingContent: '',
        });
      },

      // =========================================================================
      // Message Actions
      // =========================================================================

      addMessage: (message: ChatMessage) => {
        set((state) => ({
          messages: [...state.messages, message],
        }));
      },

      setMessages: (messages: ChatMessage[]) => {
        set({ messages });
      },

      clearMessages: () => {
        set({ messages: [], streamingContent: '' });
      },

      // =========================================================================
      // Streaming Actions
      // =========================================================================

      startStreaming: (conversationId: string) => {
        set({
          conversationId,
          status: 'streaming',
          streamingContent: '',
          error: null,
        });
      },

      receiveToken: (payload: ChatTokenPayload) => {
        const { conversationId: currentId } = get();

        // Ignore tokens for a different conversation
        if (currentId !== payload.conversation_id) {
          return;
        }

        set((state) => ({
          streamingContent: state.streamingContent + payload.token,
        }));
      },

      completeResponse: (payload: ChatCompletePayload) => {
        const { conversationId: currentId, messages } = get();

        // Ignore completion for a different conversation
        if (currentId !== payload.conversation_id) {
          return;
        }

        // Create the final message from the completed response
        const assistantMessage: ChatMessage = {
          id: payload.message_id,
          conversationId: payload.conversation_id,
          role: 'assistant',
          content: payload.full_response,
          createdAt: new Date().toISOString(),
          isStreaming: false,
        };

        set({
          messages: [...messages, assistantMessage],
          streamingContent: '',
          status: 'idle',
          inputValue: '',
        });

        // Execute any actions returned by the AI
        if (payload.actions && payload.actions.length > 0) {
          // Actions will be handled by the useChat hook or a separate action executor
          // We just expose them here; actual execution happens elsewhere
          console.log('[Chat] Actions to execute:', payload.actions);
        }
      },

      handleError: (payload: ChatErrorPayload) => {
        const { conversationId: currentId } = get();

        // Only handle errors for current conversation or global errors
        if (payload.conversation_id && currentId !== payload.conversation_id) {
          return;
        }

        set({
          status: 'error',
          streamingContent: '',
          error: {
            message: payload.error,
            code: payload.code,
          },
        });
      },

      // =========================================================================
      // Status Actions
      // =========================================================================

      setStatus: (status: ChatStatus) => {
        set({ status });
      },

      clearError: () => {
        set({ error: null });
      },

      // =========================================================================
      // Reset
      // =========================================================================

      reset: () => {
        set(initialState);
      },
    }),
    {
      name: 'resonance-chat',
      // Only persist panel open state and last conversation
      partialize: (state) => ({
        isOpen: state.isOpen,
        conversationId: state.conversationId,
      }),
    }
  )
);

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Convert server message data to local message format
 */
export function fromServerMessage(data: ChatMessageData): ChatMessage {
  return {
    id: data.id,
    conversationId: data.conversation_id,
    role: data.role,
    content: data.content,
    modelUsed: data.model_used,
    tokenCount: data.token_count,
    createdAt: data.created_at,
    isStreaming: false,
  };
}

/**
 * Create a user message for optimistic update
 */
export function createUserMessage(
  conversationId: string,
  content: string
): ChatMessage {
  return {
    id: `temp-${Date.now()}`,
    conversationId,
    role: 'user',
    content,
    createdAt: new Date().toISOString(),
    isStreaming: false,
  };
}

/**
 * Create a temporary streaming message placeholder
 */
export function createStreamingPlaceholder(conversationId: string): ChatMessage {
  return {
    id: `streaming-${Date.now()}`,
    conversationId,
    role: 'assistant',
    content: null,
    createdAt: new Date().toISOString(),
    isStreaming: true,
  };
}
