/**
 * Chat panel component
 *
 * Main container for the chat interface, displayed as a slide-out panel
 */

import { cn } from '../../lib/utils';
import { useChat } from '../../hooks/useChat';
import { ChatHeader } from './ChatHeader';
import { ChatMessageList } from './ChatMessageList';
import { ChatInput } from './ChatInput';
import { ChatToggle } from './ChatToggle';

interface ChatPanelProps {
  /** Override panel visibility (controlled mode) */
  isOpen?: boolean;
  /** Override toggle handler (controlled mode) */
  onToggle?: () => void;
}

export function ChatPanel({ isOpen: controlledIsOpen, onToggle }: ChatPanelProps): JSX.Element {
  const {
    isOpen: internalIsOpen,
    messages,
    isStreaming,
    streamingContent,
    inputValue,
    error,
    isConnected,
    isLoadingMessages,
    closeChat,
    toggleChat,
    setInputValue,
    sendMessage,
    startNewConversation,
    clearError,
  } = useChat();

  // Use controlled or internal state
  const isOpen = controlledIsOpen ?? internalIsOpen;
  const handleToggle = onToggle ?? toggleChat;
  const handleClose = onToggle ? () => onToggle() : closeChat;

  return (
    <>
      {/* Toggle button */}
      <ChatToggle isOpen={isOpen} onClick={handleToggle} />

      {/* Panel overlay (mobile) */}
      {isOpen && (
        <div
          className="fixed inset-0 bg-black/50 z-40 md:hidden"
          onClick={handleClose}
          aria-hidden="true"
        />
      )}

      {/* Chat panel */}
      <aside
        className={cn(
          'fixed top-0 right-0 bottom-0 z-50',
          'w-full md:w-96 lg:w-[420px]',
          'flex flex-col',
          'bg-bg-primary border-l border-border-subtle',
          'shadow-2xl',
          'transition-transform duration-300 ease-in-out',
          isOpen ? 'translate-x-0' : 'translate-x-full'
        )}
        aria-hidden={!isOpen}
      >
        {/* Header */}
        <ChatHeader
          onClose={handleClose}
          onNewChat={startNewConversation}
          isConnected={isConnected}
        />

        {/* Error banner */}
        {error && (
          <div className="px-4 py-3 bg-red-500/10 border-b border-red-500/20">
            <div className="flex items-center justify-between">
              <span className="text-sm text-red-400">
                {error.message}
              </span>
              <button
                onClick={clearError}
                className="text-red-400 hover:text-red-300 p-1"
                aria-label="Dismiss error"
              >
                <CloseIcon />
              </button>
            </div>
          </div>
        )}

        {/* Message list */}
        <ChatMessageList
          messages={messages}
          isStreaming={isStreaming}
          streamingContent={streamingContent}
          isLoading={isLoadingMessages}
        />

        {/* Input */}
        <ChatInput
          value={inputValue}
          onChange={setInputValue}
          onSend={sendMessage}
          disabled={!isConnected || isStreaming}
          placeholder={
            !isConnected
              ? 'Connecting...'
              : isStreaming
                ? 'Waiting for response...'
                : 'Ask about your music...'
          }
        />
      </aside>
    </>
  );
}

function CloseIcon(): JSX.Element {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  );
}
