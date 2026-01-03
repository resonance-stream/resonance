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
import { usePlaybackSync } from './usePlaybackSync';
import type { StateChangeSource } from './types';
import { useQueueSync } from './useQueueSync';
import { useTransferControl } from './useTransferControl';

/**
 * Configuration options for the useSyncState hook.
 *
 * Extends the base sync connection options while omitting internal callbacks
 * that are managed by the composed sync hooks.
 *
 * @example
 * ```tsx
 * const syncState = useSyncState({
 *   enabled: true,
 *   onRemoteTrackChange: (trackId) => loadTrack(trackId),
 * });
 * ```
 */
export interface UseSyncStateOptions extends Omit<UseSyncConnectionOptions, 'onPlaybackSync' | 'onSeekSync' | 'onQueueSync'> {
  /**
   * Callback invoked when a remote device changes the current track.
   * Use this to load the new track in the local player.
   *
   * @param trackId - The ID of the track to load
   */
  onRemoteTrackChange?: (trackId: string) => void;
}

/**
 * Return value interface for the useSyncState hook.
 *
 * Provides access to sync connection status, device control state,
 * and functions for broadcasting state and transferring control.
 */
export interface SyncStateValue {
  /**
   * Whether sync is enabled and the WebSocket connection is active.
   * Use this to conditionally render sync-related UI elements.
   */
  isSyncActive: boolean;

  /**
   * Whether this device is currently controlling playback.
   * Only the active device broadcasts state changes; other devices receive them.
   */
  isActiveDevice: boolean;

  /**
   * Manually trigger a playback state broadcast to all connected devices.
   * Normally called automatically on state changes, but can be used for
   * forced re-sync scenarios.
   */
  broadcastPlaybackState: () => void;

  /**
   * Manually trigger a queue state broadcast to all connected devices.
   * Normally called automatically on queue changes.
   */
  broadcastQueueState: () => void;

  /**
   * Transfer playback control to another device.
   * The target device will become the active device and start broadcasting state.
   *
   * @param deviceId - The ID of the device to transfer control to
   */
  transferToDevice: (deviceId: string) => void;

  /**
   * Request to become the active device (take control from the current active device).
   * Equivalent to calling `transferToDevice` with the local device ID.
   */
  requestControl: () => void;
}

/**
 * Facade hook for cross-device playback synchronization.
 *
 * Provides bidirectional state sync between the local player and remote devices
 * through a WebSocket connection. This is a composition of three specialized hooks:
 * - {@link usePlaybackSync}: Playback state (play/pause, volume, position, etc.)
 * - {@link useQueueSync}: Queue state (track list, current index)
 * - {@link useTransferControl}: Device transfer logic
 *
 * ## Loop Prevention Pattern (stateSourceRef)
 *
 * A critical aspect of bidirectional sync is preventing infinite loops where:
 * 1. Device A changes state → broadcasts to Device B
 * 2. Device B receives and applies state → triggers a "change" event
 * 3. Device B broadcasts back to Device A → infinite loop!
 *
 * The `stateSourceRef` ref tracks the origin of each state change:
 * - `'local'`: Change originated from user action on this device
 * - `'remote'`: Change came from a sync message from another device
 * - `null`: No change in progress
 *
 * When applying remote state, the ref is set to `'remote'` before updating
 * local state, then cleared after a microtask. The broadcast effects check
 * this ref and skip broadcasting when the source is `'remote'`.
 *
 * @param options - Configuration options for sync behavior
 * @returns Object containing sync state and control functions
 *
 * @example
 * ```tsx
 * function PlayerComponent() {
 *   const {
 *     isSyncActive,
 *     isActiveDevice,
 *     requestControl,
 *   } = useSyncState({
 *     onRemoteTrackChange: (trackId) => loadTrack(trackId),
 *   });
 *
 *   return (
 *     <div>
 *       <span>{isSyncActive ? 'Connected' : 'Disconnected'}</span>
 *       {!isActiveDevice && (
 *         <button onClick={requestControl}>Take Control</button>
 *       )}
 *     </div>
 *   );
 * }
 * ```
 */
export function useSyncState(options: UseSyncStateOptions = {}): SyncStateValue {
  const { onRemoteTrackChange, ...syncOptions } = options;

  /**
   * Shared ref tracking the source of state changes to prevent sync loops.
   *
   * This ref is passed to all composed sync hooks. When a remote sync message
   * is received, the handler sets this to 'remote' before applying state changes.
   * The broadcast effects check this value and skip broadcasting when it's 'remote',
   * preventing the received state from being re-broadcast back to other devices.
   *
   * The ref is cleared to `null` via `queueMicrotask()` after state updates,
   * allowing subsequent local changes to broadcast normally.
   *
   * @see usePlaybackSync - Uses this for playback state loop prevention
   * @see useQueueSync - Uses this for queue state loop prevention
   */
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
