import { memo } from 'react';
import { Button } from '../ui/Button';

interface QueueHeaderProps {
  trackCount: number;
  onClear: () => void;
  onClose: () => void;
}

/**
 * Header for the queue panel with title, track count, and clear button
 */
export const QueueHeader = memo(function QueueHeader({
  trackCount,
  onClear,
  onClose,
}: QueueHeaderProps): JSX.Element {
  return (
    <div className="flex items-center justify-between pb-4 border-b border-border">
      <div>
        <h2 className="font-medium text-text-primary">Queue</h2>
        <p className="text-xs text-text-muted">
          {trackCount} {trackCount === 1 ? 'track' : 'tracks'}
        </p>
      </div>
      <div className="flex items-center gap-2">
        {trackCount > 0 && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onClear}
            aria-label="Clear queue"
          >
            Clear
          </Button>
        )}
        <Button
          variant="ghost"
          size="icon"
          onClick={onClose}
          aria-label="Close queue"
        >
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </Button>
      </div>
    </div>
  );
});
