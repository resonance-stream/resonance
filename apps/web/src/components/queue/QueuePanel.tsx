import { useCallback, useState } from 'react';
import { usePlayerStore } from '../../stores/playerStore';
import { QueueHeader } from './QueueHeader';
import { QueueTrack } from './QueueTrack';
import { cn } from '../../lib/utils';

interface QueuePanelProps {
  className?: string;
  onClose: () => void;
}

/**
 * Queue panel showing all tracks in the playback queue
 * Supports drag-and-drop reordering and track removal
 * Note: QueueTrack components subscribe to store actions directly for stable callbacks
 */
export function QueuePanel({ className, onClose }: QueuePanelProps): JSX.Element {
  const queue = usePlayerStore((s) => s.queue);
  const queueIndex = usePlayerStore((s) => s.queueIndex);
  const reorderQueue = usePlayerStore((s) => s.reorderQueue);
  const clearQueue = usePlayerStore((s) => s.clearQueue);

  // Drag state for reordering
  const [draggedIndex, setDraggedIndex] = useState<number | null>(null);

  const handleDragStart = useCallback((e: React.DragEvent, index: number) => {
    setDraggedIndex(index);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', String(index));
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent, toIndex: number) => {
      e.preventDefault();
      if (draggedIndex !== null && draggedIndex !== toIndex) {
        reorderQueue(draggedIndex, toIndex);
      }
      setDraggedIndex(null);
    },
    [draggedIndex, reorderQueue]
  );

  const handleDragEnd = useCallback(() => {
    setDraggedIndex(null);
  }, []);

  // Split queue into "Up Next" (after current) and "History" (before current)
  const upNext = queue.slice(queueIndex + 1);
  const history = queue.slice(0, queueIndex);
  const currentTrack = queue[queueIndex];

  return (
    <div
      className={cn(
        'bg-background-secondary rounded-xl border border-border p-4 w-80 max-h-[70vh] flex flex-col',
        className
      )}
      role="region"
      aria-label="Playback queue"
      onDragEnd={handleDragEnd}
    >
      <QueueHeader
        trackCount={queue.length}
        onClear={clearQueue}
        onClose={onClose}
      />

      {queue.length === 0 ? (
        <div className="flex-1 flex items-center justify-center py-8">
          <p className="text-text-muted text-sm">Queue is empty</p>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto mt-4 space-y-4">
          {/* Now Playing */}
          {currentTrack && (
            <div>
              <h3 className="text-xs font-medium text-text-muted uppercase tracking-wide mb-2">
                Now Playing
              </h3>
              <div role="list" aria-label="Now playing">
                <QueueTrack
                  key={`now-${currentTrack.id}`}
                  track={currentTrack}
                  index={queueIndex}
                  isCurrent={true}
                  onDragStart={handleDragStart}
                  onDragOver={handleDragOver}
                  onDrop={handleDrop}
                  onReorder={reorderQueue}
                />
              </div>
            </div>
          )}

          {/* Up Next */}
          {upNext.length > 0 && (
            <div>
              <h3 className="text-xs font-medium text-text-muted uppercase tracking-wide mb-2">
                Up Next ({upNext.length})
              </h3>
              <div role="list" aria-label="Up next tracks" className="space-y-1">
                {upNext.map((track, i) => {
                  const actualIndex = queueIndex + 1 + i;
                  return (
                    <QueueTrack
                      key={`next-${track.id}-${actualIndex}`}
                      track={track}
                      index={actualIndex}
                      isCurrent={false}
                      onDragStart={handleDragStart}
                      onDragOver={handleDragOver}
                      onDrop={handleDrop}
                      onReorder={reorderQueue}
                    />
                  );
                })}
              </div>
            </div>
          )}

          {/* History */}
          {history.length > 0 && (
            <div>
              <h3 className="text-xs font-medium text-text-muted uppercase tracking-wide mb-2">
                History ({history.length})
              </h3>
              <div role="list" aria-label="Previously played tracks" className="space-y-1 opacity-60">
                {history.map((track, i) => (
                  <QueueTrack
                    key={`history-${track.id}-${i}`}
                    track={track}
                    index={i}
                    isCurrent={false}
                    onDragStart={handleDragStart}
                    onDragOver={handleDragOver}
                    onDrop={handleDrop}
                    onReorder={reorderQueue}
                  />
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
