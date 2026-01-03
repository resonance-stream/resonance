/**
 * Sync State Management Hook (Facade)
 *
 * Provides bidirectional synchronization between local player state and
 * remote devices. This is a facade that composes three specialized hooks:
 * - usePlaybackSync: Playback state sync (play/pause, volume, position)
 * - useQueueSync: Queue state sync (tracks, current index)
 * - useTransferControl: Device transfer logic
 *
 * The key pattern is tracking the "source" of each state change:
 * - 'local': User action on this device -> broadcast to others
 * - 'remote': Sync message from another device -> apply locally
 *
 * Changes are only broadcast when the source is 'local', preventing
 * an incoming sync message from being re-broadcast back out.
 */

import { useRef } from 'react';
import { useIsActiveDevice } from '../stores/deviceStore';
import { useSyncConnection, type UseSyncConnectionOptions } from './useSyncConnection';
import { usePlaybackSync, type StateChangeSource } from './usePlaybackSync';
import { useQueueSync } from './useQueueSync';
import { useTransferControl } from './useTransferControl';

export interface UseSyncStateOptions extends Omit<UseSyncConnectionOptions, 'onPlaybackSync' | 'onSeekSync' | 'onQueueSync'> {
  /** Callback when remote device changes the track */
  onRemoteTrackChange?: (trackId: string) => void;
}

export interface SyncStateValue {
  /** Whether sync is enabled and connected */
  isSyncActive: boolean;
  /** Whether this device is controlling playback */
  isActiveDevice: boolean;
  /** Manually trigger a playback state broadcast */
  broadcastPlaybackState: () => void;
  /** Manually trigger a queue state broadcast */
  broadcastQueueState: () => void;
  /** Transfer control to another device */
  transferToDevice: (deviceId: string) => void;
  /** Request to become the active device */
  requestControl: () => void;
}

/**
 * Hook for syncing playback state across devices
 *
 * Integrates with playerStore and provides loop-free bidirectional sync.
 * This is a facade that composes usePlaybackSync, useQueueSync, and useTransferControl.
 */
export function useSyncState(options: UseSyncStateOptions = {}): SyncStateValue {
  const { onRemoteTrackChange, ...syncOptions } = options;

  // Shared ref to track state change source (prevents sync loops)
  const stateSourceRef = useRef<StateChangeSource>(null);

  // Device state
  const isActiveDevice = useIsActiveDevice();

  // Refs for forwarding handlers to useSyncConnection callbacks
  // This is needed because we need to set up useSyncConnection first to get
  // the send functions, but we also need to pass handlers to useSyncConnection
  const playbackSyncHandlersRef = useRef<{
    handlePlaybackSync: ReturnType<typeof usePlaybackSync>['handlePlaybackSync'];
    handleSeekSync: ReturnType<typeof usePlaybackSync>['handleSeekSync'];
  } | null>(null);

  const queueSyncHandlersRef = useRef<{
    handleQueueSync: ReturnType<typeof useQueueSync>['handleQueueSync'];
  } | null>(null);

  // Set up sync connection with handlers that forward to our hooks
  const {
    isConnected,
    sendPlaybackUpdate,
    sendQueueUpdate,
    requestTransfer,
  } = useSyncConnection({
    ...syncOptions,
    onPlaybackSync: (state) => playbackSyncHandlersRef.current?.handlePlaybackSync(state),
    onSeekSync: (positionMs, timestamp) => playbackSyncHandlersRef.current?.handleSeekSync(positionMs, timestamp),
    onQueueSync: (state) => queueSyncHandlersRef.current?.handleQueueSync(state),
  });

  // Set up playback sync
  const {
    handlePlaybackSync,
    handleSeekSync,
    broadcastPlaybackState,
  } = usePlaybackSync({
    isConnected,
    sendPlaybackUpdate,
    stateSourceRef,
    onRemoteTrackChange,
  });

  // Store handlers in ref for useSyncConnection callbacks
  playbackSyncHandlersRef.current = { handlePlaybackSync, handleSeekSync };

  // Set up queue sync
  const {
    handleQueueSync,
    broadcastQueueState,
  } = useQueueSync({
    isConnected,
    sendQueueUpdate,
    stateSourceRef,
  });

  // Store handlers in ref for useSyncConnection callbacks
  queueSyncHandlersRef.current = { handleQueueSync };

  // Set up transfer control
  const {
    transferToDevice,
    requestControl,
  } = useTransferControl({
    isConnected,
    requestTransfer,
  });

  return {
    isSyncActive: isConnected,
    isActiveDevice,
    broadcastPlaybackState,
    broadcastQueueState,
    transferToDevice,
    requestControl,
  };
}
