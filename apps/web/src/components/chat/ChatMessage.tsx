/**
 * Individual chat message component
 *
 * Renders a single message bubble with role-appropriate styling
 */

import { cn } from '../../lib/utils';
import type { ChatMessage as ChatMessageType } from '../../stores/chatStore';

interface ChatMessageProps {
  message: ChatMessageType;
}

export function ChatMessage({ message }: ChatMessageProps): JSX.Element {
  const isUser = message.role === 'user';
  const isAssistant = message.role === 'assistant';
  const isSystem = message.role === 'system';

  return (
    <div
      className={cn(
        'flex w-full mb-4',
        isUser ? 'justify-end' : 'justify-start'
      )}
    >
      <div
        className={cn(
          'max-w-[80%] rounded-2xl px-4 py-3',
          isUser && 'bg-accent-primary text-white rounded-br-md',
          isAssistant && 'bg-bg-elevated text-text-primary rounded-bl-md',
          isSystem && 'bg-bg-tertiary text-text-secondary text-sm italic rounded-md'
        )}
      >
        {/* Message content */}
        <div className="whitespace-pre-wrap break-words">
          {message.content ?? (
            <span className="text-text-muted italic">No content</span>
          )}
        </div>

        {/* Metadata (for assistant messages) */}
        {isAssistant && message.modelUsed && (
          <div className="mt-2 pt-2 border-t border-border-subtle">
            <span className="text-xs text-text-muted">
              {message.modelUsed}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
