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

/**
 * Configuration options for the useQueueSync hook.
 *
 * Provides the connection state, send function, and shared state source ref
 * needed for bidirectional queue synchronization.
 */
export interface UseQueueSyncOptions {
  /**
   * Whether the WebSocket sync connection is currently active.
   * When false, all sync operations are no-ops.
   */
  isConnected: boolean;

  /**
   * Function to send queue state updates to remote devices.
   * Provided by the parent useSyncConnection hook.
   *
   * @param state - The queue state to broadcast
   */
  sendQueueUpdate: (state: QueueState) => void;

  /**
   * Shared ref to track the source of state changes for loop prevention.
   * Set to 'remote' when applying incoming sync messages to prevent re-broadcast.
   *
   * @see StateChangeSource - For possible values and their meanings
   * @see useSyncState - For the full loop prevention pattern documentation
   */
  stateSourceRef: React.MutableRefObject<StateChangeSource>;
}

/**
 * Return value interface for the useQueueSync hook.
 *
 * Provides a handler for incoming queue sync messages and a function
 * to manually trigger queue state broadcasts.
 */
export interface UseQueueSyncValue {
  /**
   * Handler for incoming queue state sync messages from remote devices.
   * Replaces the local queue with the remote queue state.
   * Only applies state if this device is NOT the active device.
   *
   * @param syncQueue - The queue state received from a remote device
   */
  handleQueueSync: (syncQueue: QueueState) => void;

  /**
   * Manually trigger a queue state broadcast to all connected devices.
   * Only broadcasts if connected and this device is the active device.
   */
  broadcastQueueState: () => void;
}

/**
 * Hook for bidirectional queue state synchronization across devices.
 *
 * Handles syncing the playback queue (track list and current index) between
 * the local player and remote devices. This hook is typically composed by
 * {@link useSyncState} rather than used directly.
 *
 * ## Key behaviors:
 * - **Active device only broadcasts**: Queue state is only sent when this device
 *   is the active (controlling) device
 * - **Automatic sync**: Queue changes trigger immediate broadcasts (no throttling)
 * - **Loop prevention**: Uses stateSourceRef to prevent re-broadcasting received state
 * - **Full queue replacement**: Incoming queue state replaces the entire local queue
 *
 * @param options - Configuration options including connection state and handlers
 * @returns Object with sync handler and broadcast function
 *
 * @see useSyncState - The facade hook that composes this with other sync hooks
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
