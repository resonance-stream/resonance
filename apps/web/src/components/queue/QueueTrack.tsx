import { memo, useCallback } from 'react';
import { usePlayerStore, type Track } from '../../stores/playerStore';
import { cn } from '../../lib/utils';

interface QueueTrackProps {
  track: Track;
  index: number;
  isCurrent: boolean;
  onDragStart: (e: React.DragEvent, index: number) => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: (e: React.DragEvent, index: number) => void;
  onReorder: (fromIndex: number, toIndex: number) => void;
}

/**
 * Individual track item in the queue
 * Supports drag and drop for reordering
 */
export const QueueTrack = memo(function QueueTrack({
  track,
  index,
  isCurrent,
  onDragStart,
  onDragOver,
  onDrop,
  onReorder,
}: QueueTrackProps): JSX.Element {
  // Get store actions directly to avoid callback instability
  const jumpToIndex = usePlayerStore((s) => s.jumpToIndex);
  const removeFromQueue = usePlayerStore((s) => s.removeFromQueue);
  const queueLength = usePlayerStore((s) => s.queue.length);

  // Stable callbacks using useCallback
  const handlePlay = useCallback(() => {
    jumpToIndex(index);
  }, [jumpToIndex, index]);

  const handleRemove = useCallback(() => {
    removeFromQueue(index);
  }, [removeFromQueue, index]);

  // Keyboard navigation for reordering (Shift+Arrow keys)
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.shiftKey) {
      if (e.key === 'ArrowUp' && index > 0) {
        e.preventDefault();
        onReorder(index, index - 1);
      } else if (e.key === 'ArrowDown' && index < queueLength - 1) {
        e.preventDefault();
        onReorder(index, index + 1);
      }
    }
    // Enter/Space to play
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handlePlay();
    }
    // Delete to remove
    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      handleRemove();
    }
  }, [index, queueLength, onReorder, handlePlay, handleRemove]);

  // Format duration as mm:ss
  const formatDuration = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div
      draggable
      tabIndex={0}
      onDragStart={(e) => onDragStart(e, index)}
      onDragOver={onDragOver}
      onDrop={(e) => onDrop(e, index)}
      onKeyDown={handleKeyDown}
      className={cn(
        'flex items-center gap-3 p-2 rounded-lg cursor-grab active:cursor-grabbing',
        'hover:bg-background-tertiary transition-colors',
        'focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow',
        isCurrent && 'bg-background-tertiary border border-accent/30'
      )}
      role="listitem"
      aria-label={`${track.title} by ${track.artist}. Press Enter to play, Delete to remove, Shift+Arrow to reorder.`}
      aria-current={isCurrent ? true : undefined}
    >
      {/* Drag handle */}
      <div className="text-text-muted hover:text-text-secondary cursor-grab" aria-hidden="true">
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 8h16M4 16h16" />
        </svg>
      </div>

      {/* Track number or playing indicator */}
      <div className="w-6 text-center text-sm text-text-muted">
        {isCurrent ? (
          <span className="text-accent" aria-label="Now playing">
            <svg className="w-4 h-4 mx-auto" fill="currentColor" viewBox="0 0 24 24">
              <path d="M8 5v14l11-7z" />
            </svg>
          </span>
        ) : (
          <span>{index + 1}</span>
        )}
      </div>

      {/* Cover art */}
      <div className="w-10 h-10 rounded overflow-hidden bg-background-tertiary flex-shrink-0">
        {track.coverUrl ? (
          <img
            src={track.coverUrl}
            alt=""
            className="w-full h-full object-cover"
            loading="lazy"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-text-muted">
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"
              />
            </svg>
          </div>
        )}
      </div>

      {/* Track info */}
      <button
        onClick={handlePlay}
        className="flex-1 min-w-0 text-left hover:text-accent transition-colors"
      >
        <p
          className={cn(
            'text-sm font-medium truncate',
            isCurrent ? 'text-accent' : 'text-text-primary'
          )}
        >
          {track.title}
        </p>
        <p className="text-xs text-text-muted truncate">
          {track.artist}
        </p>
      </button>

      {/* Duration */}
      <span className="text-xs text-text-muted">
        {formatDuration(track.duration)}
      </span>

      {/* Remove button */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          handleRemove();
        }}
        className="p-1 text-text-muted hover:text-error transition-colors"
        aria-label={`Remove ${track.title} from queue`}
      >
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  );
});
