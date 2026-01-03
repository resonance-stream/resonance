/**
 * Queue Sync Hook
 *
 * Handles bidirectional synchronization of queue state (tracks, current index)
 * between local player and remote devices.
 *
 * Extracted from useSyncState.ts for better modularity.
 */

import { useCallback, useEffect } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useIsActiveDevice } from '../stores/deviceStore';
import type { QueueState } from './types';
import {
  toSyncQueueState,
  fromSyncQueueState,
  type LocalQueueTrack,
} from './adapters';
import type { StateChangeSource } from './usePlaybackSync';

export interface UseQueueSyncOptions {
  /** Whether sync connection is active */
  isConnected: boolean;
  /** Send queue state to remote devices */
  sendQueueUpdate: (state: QueueState) => void;
  /** Shared ref to track state change source (for loop prevention) */
  stateSourceRef: React.MutableRefObject<StateChangeSource>;
}

export interface UseQueueSyncValue {
  /** Handle incoming queue sync from remote device */
  handleQueueSync: (syncQueue: QueueState) => void;
  /** Manually trigger a queue state broadcast */
  broadcastQueueState: () => void;
}

/**
 * Hook for syncing queue state across devices
 */
export function useQueueSync(options: UseQueueSyncOptions): UseQueueSyncValue {
  const { isConnected, sendQueueUpdate, stateSourceRef } = options;

  // Get player state
  const queue = usePlayerStore((s) => s.queue);
  const queueIndex = usePlayerStore((s) => s.queueIndex);

  // Get player actions
  const setQueue = usePlayerStore((s) => s.setQueue);

  // Device state
  const isActiveDevice = useIsActiveDevice();

  // Build current queue state
  const buildQueueState = useCallback((): QueueState => {
    const localTracks: LocalQueueTrack[] = queue.map((t) => ({
      id: t.id,
      title: t.title,
      artist: t.artist,
      albumId: t.albumId,
      albumTitle: t.albumTitle,
      duration: t.duration,
      coverUrl: t.coverUrl,
    }));
    return toSyncQueueState(localTracks, queueIndex);
  }, [queue, queueIndex]);

  // Handle incoming queue sync
  const handleQueueSync = useCallback(
    (syncQueue: QueueState) => {
      if (isActiveDevice) return;

      stateSourceRef.current = 'remote';

      const { tracks, currentIndex } = fromSyncQueueState(syncQueue);

      // Convert to playerStore track format
      // albumId is now properly synced from the active device
      const playerTracks = tracks.map((t) => ({
        id: t.id,
        title: t.title,
        artist: t.artist,
        albumId: t.albumId ?? '',
        albumTitle: t.albumTitle,
        duration: t.duration,
        coverUrl: t.coverUrl,
      }));

      setQueue(playerTracks, currentIndex);

      queueMicrotask(() => {
        if (stateSourceRef.current === 'remote') {
          stateSourceRef.current = null;
        }
      });
    },
    [isActiveDevice, setQueue, stateSourceRef]
  );

  // Broadcast queue state
  const broadcastQueueState = useCallback(() => {
    if (!isConnected || !isActiveDevice) return;
    sendQueueUpdate(buildQueueState());
  }, [isConnected, isActiveDevice, sendQueueUpdate, buildQueueState]);

  // Broadcast queue changes
  useEffect(() => {
    if (!isConnected || !isActiveDevice) return;
    if (stateSourceRef.current === 'remote') return;

    broadcastQueueState();
  }, [isConnected, isActiveDevice, queue, queueIndex, broadcastQueueState, stateSourceRef]);

  return {
    handleQueueSync,
    broadcastQueueState,
  };
}
