/**
 * Chat message list component
 *
 * Scrollable container for chat messages with auto-scroll
 */

import { useRef, useEffect } from 'react';
import { ChatMessage } from './ChatMessage';
import { StreamingMessage, TypingIndicator } from './StreamingMessage';
import type { ChatMessage as ChatMessageType } from '../../stores/chatStore';

interface ChatMessageListProps {
  messages: ChatMessageType[];
  isStreaming: boolean;
  streamingContent: string;
  isLoading?: boolean;
}

export function ChatMessageList({
  messages,
  isStreaming,
  streamingContent,
  isLoading = false,
}: ChatMessageListProps): JSX.Element {
  const scrollRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamingContent]);

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="flex flex-col items-center gap-2 text-text-muted">
          <LoadingSpinner />
          <span className="text-sm">Loading messages...</span>
        </div>
      </div>
    );
  }

  if (messages.length === 0 && !isStreaming) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="text-center">
          <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-bg-elevated flex items-center justify-center">
            <ChatIcon />
          </div>
          <h3 className="text-lg font-display text-text-primary mb-2">
            How can I help?
          </h3>
          <p className="text-sm text-text-secondary max-w-xs">
            Ask me about your music library, request recommendations,
            or let me help you create playlists.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div
      ref={scrollRef}
      className="flex-1 overflow-y-auto px-4 py-4"
    >
      {/* Message list */}
      {messages.map((message) => (
        <ChatMessage key={message.id} message={message} />
      ))}

      {/* Streaming response */}
      {isStreaming && streamingContent ? (
        <StreamingMessage content={streamingContent} />
      ) : isStreaming ? (
        <TypingIndicator />
      ) : null}

      {/* Scroll anchor */}
      <div ref={bottomRef} />
    </div>
  );
}

function LoadingSpinner(): JSX.Element {
  return (
    <svg
      className="animate-spin w-6 h-6"
      viewBox="0 0 24 24"
      fill="none"
    >
      <circle
        className="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        strokeWidth="4"
      />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      />
    </svg>
  );
}

function ChatIcon(): JSX.Element {
  return (
    <svg
      width="32"
      height="32"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="text-accent-primary"
    >
      <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
    </svg>
  );
}
