/**
 * Playback Sync Hook
 *
 * Handles bidirectional synchronization of playback state (play/pause, volume,
 * position, mute) between local player and remote devices.
 *
 * Extracted from useSyncState.ts for better modularity.
 */

import { useRef, useCallback, useEffect } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { usePlayerStore } from '../stores/playerStore';
import { useIsActiveDevice } from '../stores/deviceStore';
import type { PlaybackState } from './types';
import {
  toSyncPlaybackState,
  fromSyncPlaybackState,
  adjustPositionForClockDrift,
} from './adapters';

/**
 * Indicates the source of a state change for sync loop prevention.
 *
 * Used by sync hooks to determine whether a state change should be broadcast:
 * - `'local'`: Change from user action on this device → should broadcast
 * - `'remote'`: Change from sync message → should NOT re-broadcast
 * - `null`: No change in progress → subsequent changes are local
 *
 * @see useSyncState - Documents the full loop prevention pattern
 */
export type StateChangeSource = 'local' | 'remote' | null;

/** Default throttle interval for playback updates (ms) */
const DEFAULT_PLAYBACK_THROTTLE_MS = 250;

/** Default minimum position difference to trigger a seek sync (ms) */
const DEFAULT_SEEK_THRESHOLD_MS = 1000;

/** Default interval for periodic position broadcasts while playing (ms) */
const DEFAULT_POSITION_BROADCAST_INTERVAL_MS = 5000;

/**
 * Consolidated ref state for playback sync internal tracking.
 * Groups related state into a single ref object for cleaner state management
 * and more explicit state transitions.
 */
interface PlaybackSyncRefs {
  /** Timestamp of last broadcast (for throttling) */
  lastBroadcast: number;
  /** Previous track ID (for detecting track changes) */
  prevTrackId: string | undefined;
  /** Last known local position (for detecting local seeks) */
  lastLocalPosition: number;
  /** Flag to indicate if the ref has been initialized with current values */
  initialized: boolean;
}

/**
 * Configuration options for the usePlaybackSync hook.
 *
 * Provides the connection state, send function, and shared state source ref
 * needed for bidirectional playback synchronization.
 */
export interface UsePlaybackSyncOptions {
  /**
   * Whether the WebSocket sync connection is currently active.
   * When false, all sync operations are no-ops.
   */
  isConnected: boolean;

  /**
   * Function to send playback state updates to remote devices.
   * Provided by the parent useSyncConnection hook.
   *
   * @param state - The playback state to broadcast
   */
  sendPlaybackUpdate: (state: PlaybackState) => void;

  /**
   * Shared ref to track the source of state changes for loop prevention.
   * Set to 'remote' when applying incoming sync messages to prevent re-broadcast.
   *
   * @see StateChangeSource - For possible values and their meanings
   * @see useSyncState - For the full loop prevention pattern documentation
   */
  stateSourceRef: React.MutableRefObject<StateChangeSource>;

  /**
   * Callback invoked when a remote device changes the current track.
   * The parent component should use this to load the new track.
   *
   * @param trackId - The ID of the track to load
   */
  onRemoteTrackChange?: (trackId: string) => void;

  /**
   * Interval in milliseconds for periodic position broadcasts while playing.
   * Keeps other devices in sync even without explicit seek operations.
   *
   * @default 5000
   */
  positionBroadcastInterval?: number;

  /**
   * Throttle interval in milliseconds for playback state broadcasts.
   * Prevents excessive network traffic from rapid state changes.
   *
   * @default 250
   */
  playbackThrottleMs?: number;

  /**
   * Minimum position difference in milliseconds to trigger a seek sync.
   * Position changes below this threshold are considered normal playback
   * progression and won't trigger immediate broadcasts or remote seeks.
   *
   * @default 1000
   */
  seekThresholdMs?: number;
}

/**
 * Return value interface for the usePlaybackSync hook.
 *
 * Provides handlers for incoming sync messages and a function to
 * manually trigger state broadcasts.
 */
export interface UsePlaybackSyncValue {
  /**
   * Handler for incoming playback state sync messages from remote devices.
   * Applies the remote state to the local player (play/pause, volume, position, etc.).
   * Only applies state if this device is NOT the active device.
   *
   * @param syncState - The playback state received from a remote device
   */
  handlePlaybackSync: (syncState: PlaybackState) => void;

  /**
   * Handler for incoming seek sync messages from remote devices.
   * Updates the local player position with clock drift compensation.
   * Only applies if this device is NOT the active device.
   *
   * @param positionMs - The target position in milliseconds
   * @param timestamp - The Unix timestamp when the seek occurred (for drift adjustment)
   */
  handleSeekSync: (positionMs: number, timestamp: number) => void;

  /**
   * Manually trigger a playback state broadcast to all connected devices.
   * Subject to throttling (250ms minimum between broadcasts).
   * Only broadcasts if connected and this device is the active device.
   */
  broadcastPlaybackState: () => void;
}

/**
 * Hook for bidirectional playback state synchronization across devices.
 *
 * Handles syncing play/pause, volume, mute, position, shuffle, and repeat state
 * between the local player and remote devices. This hook is typically composed
 * by {@link useSyncState} rather than used directly.
 *
 * ## Key behaviors:
 * - **Active device only broadcasts**: State changes are only sent when this device
 *   is the active (controlling) device
 * - **Throttled broadcasts**: Normal state changes are throttled to 250ms minimum
 * - **Immediate track changes**: Track changes bypass throttling for instant sync
 * - **Periodic position sync**: While playing, position is broadcast periodically
 * - **Loop prevention**: Uses stateSourceRef to prevent re-broadcasting received state
 * - **Clock drift compensation**: Adjusts position based on message timestamps
 *
 * @param options - Configuration options including connection state and handlers
 * @returns Object with sync handlers and broadcast function
 *
 * @see useSyncState - The facade hook that composes this with other sync hooks
 */
export function usePlaybackSync(options: UsePlaybackSyncOptions): UsePlaybackSyncValue {
  const {
    isConnected,
    sendPlaybackUpdate,
    stateSourceRef,
    onRemoteTrackChange,
    positionBroadcastInterval = DEFAULT_POSITION_BROADCAST_INTERVAL_MS,
    playbackThrottleMs = DEFAULT_PLAYBACK_THROTTLE_MS,
    seekThresholdMs = DEFAULT_SEEK_THRESHOLD_MS,
  } = options;

  // Get player state using useShallow for optimal re-render behavior
  // Groups multiple selectors into a single subscription with shallow comparison
  const { currentTrack, isPlaying, currentTime, volume, isMuted, shuffle, repeat } = usePlayerStore(
    useShallow((s) => ({
      currentTrack: s.currentTrack,
      isPlaying: s.isPlaying,
      currentTime: s.currentTime,
      volume: s.volume,
      isMuted: s.isMuted,
      shuffle: s.shuffle,
      repeat: s.repeat,
    }))
  );

  // Consolidated ref for internal playback sync state tracking
  // Uses lazy initialization pattern to set prevTrackId and lastLocalPosition
  // to current values, avoiding spurious broadcasts on initial mount
  const syncStateRef = useRef<PlaybackSyncRefs>({
    lastBroadcast: 0,
    prevTrackId: undefined,
    lastLocalPosition: 0,
    initialized: false,
  });

  // Lazy initialization with current values (only runs once)
  if (!syncStateRef.current.initialized) {
    syncStateRef.current.prevTrackId = currentTrack?.id;
    syncStateRef.current.lastLocalPosition = currentTime;
    syncStateRef.current.initialized = true;
  }

  // Get player actions
  const setCurrentTime = usePlayerStore((s) => s.setCurrentTime);
  const play = usePlayerStore((s) => s.play);
  const pause = usePlayerStore((s) => s.pause);
  const setVolume = usePlayerStore((s) => s.setVolume);
  const toggleMute = usePlayerStore((s) => s.toggleMute);

  // Device state
  const isActiveDevice = useIsActiveDevice();

  // Build current playback state
  const buildPlaybackState = useCallback((): PlaybackState => {
    return toSyncPlaybackState({
      trackId: currentTrack?.id ?? null,
      isPlaying,
      currentTime,
      volume,
      isMuted,
      shuffle,
      repeat,
    });
  }, [currentTrack?.id, isPlaying, currentTime, volume, isMuted, shuffle, repeat]);

  // Handle incoming playback sync
  const handlePlaybackSync = useCallback(
    (syncState: PlaybackState) => {
      // Don't apply if we're the active device (we're the source of truth)
      if (isActiveDevice) return;

      stateSourceRef.current = 'remote';

      const local = fromSyncPlaybackState(syncState);

      // Get current state from store to avoid stale closure values
      const current = usePlayerStore.getState();

      // Apply playback state
      if (local.isPlaying && !current.isPlaying) {
        play();
      } else if (!local.isPlaying && current.isPlaying) {
        pause();
      }

      // Apply volume if significantly different
      if (Math.abs(local.volume - current.volume) > 0.01) {
        setVolume(local.volume);
      }

      // Apply mute state - check current state to ensure idempotency
      // (toggleMute is not a setter, so we verify state before calling)
      if (local.isMuted !== current.isMuted) {
        toggleMute();
      }

      // Apply position with clock drift adjustment
      const adjustedPosition = adjustPositionForClockDrift(syncState);
      const localPositionMs = current.currentTime * 1000;
      if (Math.abs(adjustedPosition - localPositionMs) > seekThresholdMs) {
        setCurrentTime(adjustedPosition / 1000);
      }

      // Handle track change (need external handler since we don't have track loading logic)
      if (syncState.track_id !== current.currentTrack?.id && syncState.track_id) {
        onRemoteTrackChange?.(syncState.track_id);
      }

      // Clear source after microtask
      queueMicrotask(() => {
        if (stateSourceRef.current === 'remote') {
          stateSourceRef.current = null;
        }
      });
    },
    [isActiveDevice, play, pause, setVolume, toggleMute, setCurrentTime, onRemoteTrackChange, stateSourceRef, seekThresholdMs]
  );

  // Handle incoming seek sync
  const handleSeekSync = useCallback(
    (positionMs: number, timestamp: number) => {
      if (isActiveDevice) return;

      // Get current state from store to avoid stale closure values
      const current = usePlayerStore.getState();

      // Reuse clock drift adjustment logic from types.ts
      const seekState: PlaybackState = {
        track_id: null,
        is_playing: current.isPlaying,
        position_ms: positionMs,
        timestamp,
        volume: 0,
        is_muted: false,
        shuffle: false,
        repeat: 'off',
      };
      const adjustedPosition = adjustPositionForClockDrift(seekState);

      // Skip seek if position difference is within threshold (avoid jitter)
      const localPositionMs = current.currentTime * 1000;
      if (Math.abs(adjustedPosition - localPositionMs) <= seekThresholdMs) {
        return;
      }

      stateSourceRef.current = 'remote';
      setCurrentTime(adjustedPosition / 1000);

      queueMicrotask(() => {
        if (stateSourceRef.current === 'remote') {
          stateSourceRef.current = null;
        }
      });
    },
    [isActiveDevice, setCurrentTime, stateSourceRef, seekThresholdMs]
  );

  // Broadcast playback state (throttled)
  const broadcastPlaybackState = useCallback(() => {
    if (!isConnected || !isActiveDevice) return;

    const now = Date.now();
    if (now - syncStateRef.current.lastBroadcast < playbackThrottleMs) return;
    syncStateRef.current.lastBroadcast = now;

    sendPlaybackUpdate(buildPlaybackState());
  }, [isConnected, isActiveDevice, sendPlaybackUpdate, buildPlaybackState, playbackThrottleMs]);

  // Auto-broadcast on state changes (only if we're active and change is local)
  useEffect(() => {
    if (!isConnected || !isActiveDevice) return;
    if (stateSourceRef.current === 'remote') return;

    broadcastPlaybackState();
  }, [isConnected, isActiveDevice, currentTrack?.id, isPlaying, volume, isMuted, shuffle, repeat, broadcastPlaybackState, stateSourceRef]);

  // Immediate broadcast on track changes (bypass throttle for critical state)
  // Track changes need instant sync to ensure all devices switch together
  useEffect(() => {
    const prevTrackId = syncStateRef.current.prevTrackId;
    syncStateRef.current.prevTrackId = currentTrack?.id;

    // Skip if track hasn't changed or no connection
    if (!isConnected || !isActiveDevice) return;
    if (stateSourceRef.current === 'remote') return;
    if (currentTrack?.id === prevTrackId) return;

    // Bypass throttle by directly calling sendPlaybackUpdate
    sendPlaybackUpdate(buildPlaybackState());
  }, [isConnected, isActiveDevice, currentTrack?.id, sendPlaybackUpdate, buildPlaybackState, stateSourceRef]);

  // Ref for broadcast function to avoid recreating interval on every change
  const broadcastPlaybackStateRef = useRef(broadcastPlaybackState);
  useEffect(() => {
    broadcastPlaybackStateRef.current = broadcastPlaybackState;
  }, [broadcastPlaybackState]);

  // Periodic position broadcast while playing (configurable interval, default 5 seconds)
  // This keeps other devices in sync even without explicit seeks
  useEffect(() => {
    if (!isConnected || !isActiveDevice || !isPlaying) return;

    const interval = setInterval(() => {
      // Only broadcast if not processing a remote update
      if (stateSourceRef.current !== 'remote') {
        broadcastPlaybackStateRef.current();
      }
    }, positionBroadcastInterval);

    return () => clearInterval(interval);
  }, [isConnected, isActiveDevice, isPlaying, stateSourceRef, positionBroadcastInterval]);

  // Broadcast immediately on large local seeks to reduce sync lag
  useEffect(() => {
    if (!isConnected || !isActiveDevice) return;
    if (stateSourceRef.current === 'remote') return;

    const prev = syncStateRef.current.lastLocalPosition;
    syncStateRef.current.lastLocalPosition = currentTime;

    // Detect if this is a significant position jump (likely a seek)
    const deltaMs = Math.abs(currentTime - prev) * 1000;
    if (deltaMs >= seekThresholdMs) {
      broadcastPlaybackStateRef.current();
    }
  }, [isConnected, isActiveDevice, currentTime, stateSourceRef, seekThresholdMs]);

  return {
    handlePlaybackSync,
    handleSeekSync,
    broadcastPlaybackState,
  };
}
