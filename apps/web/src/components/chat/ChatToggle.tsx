/**
 * Chat toggle button
 *
 * Floating action button to open/close the chat panel
 */

import { cn } from '../../lib/utils';

interface ChatToggleProps {
  isOpen: boolean;
  onClick: () => void;
  hasUnread?: boolean;
}

export function ChatToggle({
  isOpen,
  onClick,
  hasUnread = false,
}: ChatToggleProps): JSX.Element {
  return (
    <button
      onClick={onClick}
      className={cn(
        'fixed bottom-20 right-4 z-40',
        'w-14 h-14 rounded-full',
        'flex items-center justify-center',
        'bg-accent-primary text-white shadow-lg',
        'hover:bg-accent-hover hover:shadow-xl',
        'active:scale-95',
        'transition-all duration-200',
        isOpen && 'rotate-0',
        !isOpen && 'rotate-0'
      )}
      aria-label={isOpen ? 'Close chat' : 'Open chat'}
      aria-expanded={isOpen}
    >
      {/* Unread indicator */}
      {hasUnread && !isOpen && (
        <span className="absolute top-0 right-0 w-3 h-3 bg-red-500 rounded-full border-2 border-bg-primary" />
      )}

      {/* Icon */}
      {isOpen ? <CloseIcon /> : <ChatIcon />}
    </button>
  );
}

function ChatIcon(): JSX.Element {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
    </svg>
  );
}

function CloseIcon(): JSX.Element {
  return (
    <svg
      width="24"
      height="24"
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
