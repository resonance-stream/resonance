/**
 * ChatMessageList Component Tests
 *
 * Tests for the scrollable message list container with auto-scroll.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { ChatMessageList } from './ChatMessageList';
import type { ChatMessage } from '../../stores/chatStore';

// Mock scrollIntoView since jsdom doesn't support it
Element.prototype.scrollIntoView = vi.fn();

// Mock child components to isolate ChatMessageList testing
vi.mock('./ChatMessage', () => ({
  ChatMessage: ({ message }: { message: ChatMessage }) => (
    <div data-testid={`chat-message-${message.id}`} data-role={message.role}>
      {message.content}
    </div>
  ),
}));

vi.mock('./StreamingMessage', () => ({
  StreamingMessage: ({ content }: { content: string }) => (
    <div data-testid="streaming-message">{content}</div>
  ),
  TypingIndicator: () => <div data-testid="typing-indicator">Typing...</div>,
}));

// Sample test messages
const testMessages: ChatMessage[] = [
  {
    id: 'msg-1',
    conversationId: 'conv-1',
    role: 'user',
    content: 'Hello, can you help me?',
    createdAt: '2024-01-01T10:00:00Z',
  },
  {
    id: 'msg-2',
    conversationId: 'conv-1',
    role: 'assistant',
    content: 'Of course! How can I help you today?',
    createdAt: '2024-01-01T10:00:05Z',
  },
  {
    id: 'msg-3',
    conversationId: 'conv-1',
    role: 'user',
    content: 'Find me some jazz music',
    createdAt: '2024-01-01T10:00:10Z',
  },
];

describe('ChatMessageList', () => {
  const defaultProps = {
    messages: [] as ChatMessage[],
    isStreaming: false,
    streamingContent: '',
    isLoading: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('empty state', () => {
    it('shows welcome message when no messages and not streaming', () => {
      render(<ChatMessageList {...defaultProps} />);

      expect(screen.getByText('How can I help?')).toBeInTheDocument();
      expect(screen.getByText(/ask me about your music library/i)).toBeInTheDocument();
    });

    it('does not show welcome message when there are messages', () => {
      render(<ChatMessageList {...defaultProps} messages={testMessages} />);

      expect(screen.queryByText('How can I help?')).not.toBeInTheDocument();
    });
  });

  describe('loading state', () => {
    it('shows loading spinner when isLoading is true', () => {
      render(<ChatMessageList {...defaultProps} isLoading={true} />);

      expect(screen.getByText(/loading messages/i)).toBeInTheDocument();
    });

    it('does not show messages when loading', () => {
      render(<ChatMessageList {...defaultProps} messages={testMessages} isLoading={true} />);

      expect(screen.queryByTestId('chat-message-msg-1')).not.toBeInTheDocument();
    });
  });

  describe('message rendering', () => {
    it('renders all messages in the list', () => {
      render(<ChatMessageList {...defaultProps} messages={testMessages} />);

      expect(screen.getByTestId('chat-message-msg-1')).toBeInTheDocument();
      expect(screen.getByTestId('chat-message-msg-2')).toBeInTheDocument();
      expect(screen.getByTestId('chat-message-msg-3')).toBeInTheDocument();
    });

    it('renders messages with correct content', () => {
      render(<ChatMessageList {...defaultProps} messages={testMessages} />);

      expect(screen.getByText('Hello, can you help me?')).toBeInTheDocument();
      expect(screen.getByText('Of course! How can I help you today?')).toBeInTheDocument();
      expect(screen.getByText('Find me some jazz music')).toBeInTheDocument();
    });

    it('renders messages with correct roles', () => {
      render(<ChatMessageList {...defaultProps} messages={testMessages} />);

      expect(screen.getByTestId('chat-message-msg-1')).toHaveAttribute('data-role', 'user');
      expect(screen.getByTestId('chat-message-msg-2')).toHaveAttribute('data-role', 'assistant');
      expect(screen.getByTestId('chat-message-msg-3')).toHaveAttribute('data-role', 'user');
    });
  });

  describe('streaming state', () => {
    it('shows typing indicator when streaming with no content yet', () => {
      render(
        <ChatMessageList
          {...defaultProps}
          messages={testMessages}
          isStreaming={true}
          streamingContent=""
        />
      );

      expect(screen.getByTestId('typing-indicator')).toBeInTheDocument();
    });

    it('shows streaming message when streaming with content', () => {
      render(
        <ChatMessageList
          {...defaultProps}
          messages={testMessages}
          isStreaming={true}
          streamingContent="I found some great jazz tracks..."
        />
      );

      expect(screen.getByTestId('streaming-message')).toBeInTheDocument();
      expect(screen.getByText('I found some great jazz tracks...')).toBeInTheDocument();
    });

    it('does not show streaming components when not streaming', () => {
      render(<ChatMessageList {...defaultProps} messages={testMessages} isStreaming={false} />);

      expect(screen.queryByTestId('streaming-message')).not.toBeInTheDocument();
      expect(screen.queryByTestId('typing-indicator')).not.toBeInTheDocument();
    });
  });

  describe('scrolling', () => {
    it('renders scroll container', () => {
      render(<ChatMessageList {...defaultProps} messages={testMessages} />);

      // The component should have overflow-y-auto class
      const scrollContainer = screen.getByTestId('chat-message-msg-1').parentElement;
      expect(scrollContainer).toHaveClass('overflow-y-auto');
    });
  });
});
