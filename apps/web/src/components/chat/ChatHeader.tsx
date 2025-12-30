/**
 * Chat header component
 *
 * Title bar with close button and new conversation button
 */

import { cn } from '../../lib/utils';

interface ChatHeaderProps {
  title?: string;
  onClose: () => void;
  onNewChat: () => void;
  isConnected: boolean;
}

export function ChatHeader({
  title = 'Resonance AI',
  onClose,
  onNewChat,
  isConnected,
}: ChatHeaderProps): JSX.Element {
  return (
    <div className="flex items-center justify-between px-4 py-3 border-b border-border-subtle bg-bg-primary">
      {/* Left side: Title and connection status */}
      <div className="flex items-center gap-2">
        <h2 className="font-display text-lg text-text-primary">
          {title}
        </h2>

        {/* Connection indicator */}
        <span
          className={cn(
            'w-2 h-2 rounded-full',
            isConnected ? 'bg-green-500' : 'bg-red-500'
          )}
          title={isConnected ? 'Connected' : 'Disconnected'}
        />
      </div>

      {/* Right side: Actions */}
      <div className="flex items-center gap-1">
        {/* New conversation button */}
        <button
          onClick={onNewChat}
          className={cn(
            'p-2 rounded-lg',
            'text-text-secondary hover:text-text-primary hover:bg-bg-elevated',
            'transition-colors duration-200'
          )}
          aria-label="New conversation"
          title="New conversation"
        >
          <PlusIcon />
        </button>

        {/* Close button */}
        <button
          onClick={onClose}
          className={cn(
            'p-2 rounded-lg',
            'text-text-secondary hover:text-text-primary hover:bg-bg-elevated',
            'transition-colors duration-200'
          )}
          aria-label="Close chat"
        >
          <CloseIcon />
        </button>
      </div>
    </div>
  );
}

function PlusIcon(): JSX.Element {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  );
}

function CloseIcon(): JSX.Element {
  return (
    <svg
      width="20"
      height="20"
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
