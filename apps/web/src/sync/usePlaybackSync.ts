/**
 * Playback Sync Hook
 *
 * Handles bidirectional synchronization of playback state (play/pause, volume,
 * position, mute) between local player and remote devices.
 *
 * Extracted from useSyncState.ts for better modularity.
 */

import { useRef, useCallback, useEffect } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useIsActiveDevice } from '../stores/deviceStore';
import type { PlaybackState } from './types';
import {
  toSyncPlaybackState,
  fromSyncPlaybackState,
  adjustPositionForClockDrift,
} from './adapters';

/** Source of a state change for loop prevention */
export type StateChangeSource = 'local' | 'remote' | null;

/** Throttle interval for playback updates (ms) */
const PLAYBACK_THROTTLE_MS = 250;

/** Minimum position difference to trigger a seek sync (ms) */
const SEEK_THRESHOLD_MS = 1000;

export interface UsePlaybackSyncOptions {
  /** Whether sync connection is active */
  isConnected: boolean;
  /** Send playback state to remote devices */
  sendPlaybackUpdate: (state: PlaybackState) => void;
  /** Shared ref to track state change source (for loop prevention) */
  stateSourceRef: React.MutableRefObject<StateChangeSource>;
  /** Callback when remote device changes the track */
  onRemoteTrackChange?: (trackId: string) => void;
}

export interface UsePlaybackSyncValue {
  /** Handle incoming playback sync from remote device */
  handlePlaybackSync: (syncState: PlaybackState) => void;
  /** Handle incoming seek sync from remote device */
  handleSeekSync: (positionMs: number, timestamp: number) => void;
  /** Manually trigger a playback state broadcast */
  broadcastPlaybackState: () => void;
}

/**
 * Hook for syncing playback state across devices
 */
export function usePlaybackSync(options: UsePlaybackSyncOptions): UsePlaybackSyncValue {
  const { isConnected, sendPlaybackUpdate, stateSourceRef, onRemoteTrackChange } = options;

  const lastBroadcastRef = useRef<number>(0);

  // Get player state
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const currentTime = usePlayerStore((s) => s.currentTime);
  const volume = usePlayerStore((s) => s.volume);
  const isMuted = usePlayerStore((s) => s.isMuted);
  const shuffle = usePlayerStore((s) => s.shuffle);
  const repeat = usePlayerStore((s) => s.repeat);

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
      if (Math.abs(adjustedPosition - localPositionMs) > SEEK_THRESHOLD_MS) {
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
    [isActiveDevice, play, pause, setVolume, toggleMute, setCurrentTime, onRemoteTrackChange, stateSourceRef]
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
      if (Math.abs(adjustedPosition - localPositionMs) <= SEEK_THRESHOLD_MS) {
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
    [isActiveDevice, setCurrentTime, stateSourceRef]
  );

  // Broadcast playback state (throttled)
  const broadcastPlaybackState = useCallback(() => {
    if (!isConnected || !isActiveDevice) return;

    const now = Date.now();
    if (now - lastBroadcastRef.current < PLAYBACK_THROTTLE_MS) return;
    lastBroadcastRef.current = now;

    sendPlaybackUpdate(buildPlaybackState());
  }, [isConnected, isActiveDevice, sendPlaybackUpdate, buildPlaybackState]);

  // Auto-broadcast on state changes (only if we're active and change is local)
  useEffect(() => {
    if (!isConnected || !isActiveDevice) return;
    if (stateSourceRef.current === 'remote') return;

    broadcastPlaybackState();
  }, [isConnected, isActiveDevice, currentTrack?.id, isPlaying, volume, isMuted, shuffle, repeat, broadcastPlaybackState, stateSourceRef]);

  // Immediate broadcast on track changes (bypass throttle for critical state)
  // Track changes need instant sync to ensure all devices switch together
  const prevTrackIdRef = useRef<string | undefined>(currentTrack?.id);
  useEffect(() => {
    const prevTrackId = prevTrackIdRef.current;
    prevTrackIdRef.current = currentTrack?.id;

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

  // Periodic position broadcast while playing (every 5 seconds)
  // This keeps other devices in sync even without explicit seeks
  useEffect(() => {
    if (!isConnected || !isActiveDevice || !isPlaying) return;

    const interval = setInterval(() => {
      // Only broadcast if not processing a remote update
      if (stateSourceRef.current !== 'remote') {
        broadcastPlaybackStateRef.current();
      }
    }, 5000);

    return () => clearInterval(interval);
  }, [isConnected, isActiveDevice, isPlaying, stateSourceRef]);

  // Track last position to detect local seeks
  const lastLocalPositionRef = useRef<number>(currentTime);

  // Broadcast immediately on large local seeks to reduce sync lag
  useEffect(() => {
    if (!isConnected || !isActiveDevice) return;
    if (stateSourceRef.current === 'remote') return;

    const prev = lastLocalPositionRef.current;
    lastLocalPositionRef.current = currentTime;

    // Detect if this is a significant position jump (likely a seek)
    const deltaMs = Math.abs(currentTime - prev) * 1000;
    if (deltaMs >= SEEK_THRESHOLD_MS) {
      broadcastPlaybackStateRef.current();
    }
  }, [isConnected, isActiveDevice, currentTime, stateSourceRef]);

  return {
    handlePlaybackSync,
    handleSeekSync,
    broadcastPlaybackState,
  };
}
