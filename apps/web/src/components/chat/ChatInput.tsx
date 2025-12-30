/**
 * Chat input component
 *
 * Text input with send button for submitting messages
 */

import { useCallback, useRef, useEffect, type KeyboardEvent } from 'react';
import { cn } from '../../lib/utils';

interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSend: (message: string) => void;
  disabled?: boolean;
  placeholder?: string;
}

export function ChatInput({
  value,
  onChange,
  onSend,
  disabled = false,
  placeholder = 'Ask about your music...',
}: ChatInputProps): JSX.Element {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const didAutoFocusRef = useRef(false);

  // Auto-resize textarea
  useEffect(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = 'auto';
      textarea.style.height = `${Math.min(textarea.scrollHeight, 150)}px`;
    }
  }, [value]);

  // Focus on initial mount only (not on every disabled change)
  useEffect(() => {
    if (!disabled && !didAutoFocusRef.current) {
      textareaRef.current?.focus();
      didAutoFocusRef.current = true;
    }
  }, [disabled]);

  const handleKeyDown = useCallback((e: KeyboardEvent<HTMLTextAreaElement>) => {
    // Send on Enter (but not Shift+Enter for new lines)
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (value.trim() && !disabled) {
        onSend(value);
      }
    }
  }, [value, disabled, onSend]);

  const handleSubmit = useCallback(() => {
    if (value.trim() && !disabled) {
      onSend(value);
    }
  }, [value, disabled, onSend]);

  return (
    <div className="flex items-end gap-2 p-4 border-t border-border-subtle bg-bg-primary">
      {/* Text input */}
      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        disabled={disabled}
        rows={1}
        className={cn(
          'flex-1 resize-none rounded-xl px-4 py-3',
          'bg-bg-elevated text-text-primary placeholder:text-text-muted',
          'border border-border-subtle focus:border-accent-primary focus:outline-none',
          'transition-colors duration-200',
          'min-h-[44px] max-h-[150px]',
          disabled && 'opacity-50 cursor-not-allowed'
        )}
      />

      {/* Send button */}
      <button
        onClick={handleSubmit}
        disabled={disabled || !value.trim()}
        className={cn(
          'flex-shrink-0 w-11 h-11 rounded-xl',
          'flex items-center justify-center',
          'bg-accent-primary text-white',
          'hover:bg-accent-hover active:scale-95',
          'transition-all duration-200',
          'disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-accent-primary disabled:active:scale-100'
        )}
        aria-label="Send message"
      >
        <SendIcon />
      </button>
    </div>
  );
}

function SendIcon(): JSX.Element {
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
      <line x1="22" y1="2" x2="11" y2="13" />
      <polygon points="22 2 15 22 11 13 2 9 22 2" />
    </svg>
  );
}
