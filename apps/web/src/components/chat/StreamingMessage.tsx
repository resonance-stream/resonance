/**
 * Streaming message component
 *
 * Shows the AI response as it streams in with a typing indicator
 */

interface StreamingMessageProps {
  content: string;
}

export function StreamingMessage({ content }: StreamingMessageProps): JSX.Element {
  return (
    <div className="flex w-full mb-4 justify-start">
      <div className="max-w-[80%] rounded-2xl rounded-bl-md bg-bg-elevated text-text-primary px-4 py-3">
        {/* Streaming content */}
        <div className="whitespace-pre-wrap break-words">
          {content}
          {/* Typing cursor */}
          <span className="inline-block w-2 h-4 ml-1 bg-accent-primary animate-pulse rounded-sm" />
        </div>
      </div>
    </div>
  );
}

/**
 * Typing indicator (shown before any content arrives)
 */
export function TypingIndicator(): JSX.Element {
  return (
    <div className="flex w-full mb-4 justify-start">
      <div className="rounded-2xl rounded-bl-md bg-bg-elevated px-4 py-3">
        <div className="flex items-center gap-1">
          <span
            className="w-2 h-2 bg-text-muted rounded-full animate-bounce"
            style={{ animationDelay: '0ms' }}
          />
          <span
            className="w-2 h-2 bg-text-muted rounded-full animate-bounce"
            style={{ animationDelay: '150ms' }}
          />
          <span
            className="w-2 h-2 bg-text-muted rounded-full animate-bounce"
            style={{ animationDelay: '300ms' }}
          />
        </div>
      </div>
    </div>
  );
}
