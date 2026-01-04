/**
 * ChatPanel Component Tests
 *
 * Tests for the main ChatPanel component that contains all chat components.
 */

import { describe, it, expect, vi, beforeEach, type MockedFunction } from 'vitest';
import { render, screen, userEvent } from '@/test/test-utils';
import { ChatPanel } from './ChatPanel';
import { useChat } from '../../hooks/useChat';
import type { UseChatReturn } from '../../hooks/useChat';
import type { ChatMessage } from '../../stores/chatStore';

// Mock the useChat hook
vi.mock('../../hooks/useChat');

const mockedUseChat = useChat as MockedFunction<typeof useChat>;

// Mock child components to isolate ChatPanel testing
vi.mock('./ChatHeader', () => ({
  ChatHeader: ({
    onClose,
    onNewChat,
    isConnected,
  }: {
    onClose: () => void;
    onNewChat: () => void;
    isConnected: boolean;
  }) => (
    <div data-testid="chat-header">
      Chat Header Mock
      <button onClick={onClose}>Close</button>
      <button onClick={onNewChat}>New Chat</button>
      <span data-testid="connection-status">{isConnected ? 'connected' : 'disconnected'}</span>
    </div>
  ),
}));

vi.mock('./ChatMessageList', () => ({
  ChatMessageList: ({
    messages,
    isStreaming,
    streamingContent,
    isLoading,
  }: {
    messages: unknown[];
    isStreaming: boolean;
    streamingContent: string;
    isLoading?: boolean;
  }) => (
    <div data-testid="chat-message-list">
      Message List Mock
      <span data-testid="message-count">{messages.length}</span>
      <span data-testid="is-streaming">{isStreaming ? 'streaming' : 'idle'}</span>
      <span data-testid="streaming-content">{streamingContent}</span>
      <span data-testid="is-loading">{isLoading ? 'loading' : 'loaded'}</span>
    </div>
  ),
}));

vi.mock('./ChatInput', () => ({
  ChatInput: ({
    value,
    onChange,
    onSend,
    disabled,
    placeholder,
  }: {
    value: string;
    onChange: (value: string) => void;
    onSend: (message: string) => void;
    disabled?: boolean;
    placeholder?: string;
  }) => (
    <div data-testid="chat-input">
      Chat Input Mock
      <input
        data-testid="input-field"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        placeholder={placeholder}
      />
      <button onClick={() => onSend(value)} disabled={disabled}>
        Send
      </button>
    </div>
  ),
}));

vi.mock('./ChatToggle', () => ({
  ChatToggle: ({ isOpen, onClick }: { isOpen: boolean; onClick: () => void }) => (
    <button data-testid="chat-toggle" onClick={onClick} aria-label={isOpen ? 'Close chat' : 'Open chat'}>
      {isOpen ? 'Close' : 'Open'}
    </button>
  ),
}));

// Sample test messages
const testMessages: ChatMessage[] = [
  {
    id: 'msg-1',
    conversationId: 'conv-1',
    role: 'user',
    content: 'Hello',
    createdAt: new Date().toISOString(),
  },
  {
    id: 'msg-2',
    conversationId: 'conv-1',
    role: 'assistant',
    content: 'Hi there!',
    createdAt: new Date().toISOString(),
  },
];

// Default mock return values
function createMockChatReturn(overrides: Partial<UseChatReturn> = {}): UseChatReturn {
  return {
    isOpen: false,
    messages: [],
    conversationId: null,
    conversations: [],
    isStreaming: false,
    streamingContent: '',
    inputValue: '',
    error: null,
    isConnected: true,
    openChat: vi.fn(),
    closeChat: vi.fn(),
    toggleChat: vi.fn(),
    setInputValue: vi.fn(),
    sendMessage: vi.fn(),
    selectConversation: vi.fn(),
    deleteConversation: vi.fn(),
    deleteAllConversations: vi.fn(),
    startNewConversation: vi.fn(),
    clearError: vi.fn(),
    isLoadingConversations: false,
    isLoadingMessages: false,
    ...overrides,
  };
}

describe('ChatPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedUseChat.mockReturnValue(createMockChatReturn());
  });

  it('renders the chat toggle button', () => {
    render(<ChatPanel />);

    expect(screen.getByTestId('chat-toggle')).toBeInTheDocument();
  });

  it('does not show panel content when closed', () => {
    mockedUseChat.mockReturnValue(createMockChatReturn({ isOpen: false }));

    const { container } = render(<ChatPanel />);

    // Panel should have translate-x-full class and aria-hidden when closed
    const panel = container.querySelector('aside');
    expect(panel).toHaveAttribute('aria-hidden', 'true');
    expect(panel).toHaveClass('translate-x-full');
  });

  it('shows panel content when open', () => {
    mockedUseChat.mockReturnValue(createMockChatReturn({ isOpen: true }));

    render(<ChatPanel />);

    expect(screen.getByTestId('chat-header')).toBeInTheDocument();
    expect(screen.getByTestId('chat-message-list')).toBeInTheDocument();
    expect(screen.getByTestId('chat-input')).toBeInTheDocument();
  });

  it('toggles panel when toggle button is clicked', async () => {
    const toggleChat = vi.fn();
    mockedUseChat.mockReturnValue(createMockChatReturn({ toggleChat }));
    const user = userEvent.setup();

    render(<ChatPanel />);

    await user.click(screen.getByTestId('chat-toggle'));
    expect(toggleChat).toHaveBeenCalledTimes(1);
  });

  it('shows error banner when there is an error', () => {
    mockedUseChat.mockReturnValue(
      createMockChatReturn({
        isOpen: true,
        error: { message: 'Connection lost', code: 'CONN_ERROR' },
      })
    );

    render(<ChatPanel />);

    expect(screen.getByText('Connection lost')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /dismiss error/i })).toBeInTheDocument();
  });

  it('calls clearError when dismiss button is clicked', async () => {
    const clearError = vi.fn();
    mockedUseChat.mockReturnValue(
      createMockChatReturn({
        isOpen: true,
        error: { message: 'Connection lost', code: 'CONN_ERROR' },
        clearError,
      })
    );
    const user = userEvent.setup();

    render(<ChatPanel />);

    await user.click(screen.getByRole('button', { name: /dismiss error/i }));
    expect(clearError).toHaveBeenCalledTimes(1);
  });

  it('passes messages to ChatMessageList', () => {
    mockedUseChat.mockReturnValue(createMockChatReturn({ isOpen: true, messages: testMessages }));

    render(<ChatPanel />);

    expect(screen.getByTestId('message-count')).toHaveTextContent('2');
  });

  it('shows mobile overlay when open', () => {
    mockedUseChat.mockReturnValue(createMockChatReturn({ isOpen: true }));

    render(<ChatPanel />);

    // The overlay should be present (with md:hidden class)
    const overlay = document.querySelector('.fixed.inset-0.bg-black\\/50');
    expect(overlay).toBeInTheDocument();
  });
});
