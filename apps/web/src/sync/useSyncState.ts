/**
 * Sync State Management Hook
 *
 * Provides bidirectional synchronization between local player state and
 * remote devices. Implements loop prevention to avoid sync cycles.
 *
 * The key pattern is tracking the "source" of each state change:
 * - 'local': User action on this device -> broadcast to others
 * - 'remote': Sync message from another device -> apply locally
 *
 * Changes are only broadcast when the source is 'local', preventing
 * an incoming sync message from being re-broadcast back out.
 */

import { useRef, useCallback, useEffect } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useDeviceStore, useIsActiveDevice } from '../stores/deviceStore';
import { useSyncConnection, type UseSyncConnectionOptions } from './useSyncConnection';
import {
  toSyncPlaybackState,
  fromSyncPlaybackState,
  toSyncQueueState,
  fromSyncQueueState,
  adjustPositionForClockDrift,
  type PlaybackState,
  type QueueState,
  type LocalQueueTrack,
} from './types';

/** Source of a state change */
type StateChangeSource = 'local' | 'remote' | null;

/** Throttle interval for playback updates (ms) */
const PLAYBACK_THROTTLE_MS = 250;

/** Minimum position difference to trigger a seek sync (ms) */
const SEEK_THRESHOLD_MS = 1000;

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
 */
export function useSyncState(options: UseSyncStateOptions = {}): SyncStateValue {
  const { onRemoteTrackChange, ...syncOptions } = options;

  // Track state change source to prevent loops
  const stateSourceRef = useRef<StateChangeSource>(null);
  const lastBroadcastRef = useRef<number>(0);

  // Get player state
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const currentTime = usePlayerStore((s) => s.currentTime);
  const volume = usePlayerStore((s) => s.volume);
  const isMuted = usePlayerStore((s) => s.isMuted);
  const shuffle = usePlayerStore((s) => s.shuffle);
  const repeat = usePlayerStore((s) => s.repeat);
  const queue = usePlayerStore((s) => s.queue);
  const queueIndex = usePlayerStore((s) => s.queueIndex);

  // Get player actions
  const setCurrentTime = usePlayerStore((s) => s.setCurrentTime);
  const play = usePlayerStore((s) => s.play);
  const pause = usePlayerStore((s) => s.pause);
  const setVolume = usePlayerStore((s) => s.setVolume);
  const toggleMute = usePlayerStore((s) => s.toggleMute);
  const setQueue = usePlayerStore((s) => s.setQueue);

  // Device state
  const deviceId = useDeviceStore((s) => s.deviceId);
  const isActiveDevice = useIsActiveDevice();

  // Handle incoming playback sync
  const handlePlaybackSync = useCallback((syncState: PlaybackState) => {
    // Don't apply if we're the active device (we're the source of truth)
    if (isActiveDevice) return;

    stateSourceRef.current = 'remote';

    const local = fromSyncPlaybackState(syncState);

    // Apply playback state
    if (local.isPlaying && !isPlaying) {
      play();
    } else if (!local.isPlaying && isPlaying) {
      pause();
    }

    // Apply volume if significantly different
    if (Math.abs(local.volume - volume) > 0.01) {
      setVolume(local.volume);
    }

    // Apply mute state - check current state to ensure idempotency
    // (toggleMute is not a setter, so we verify state before calling)
    const currentMuted = usePlayerStore.getState().isMuted;
    if (local.isMuted !== currentMuted) {
      toggleMute();
    }

    // Apply position with clock drift adjustment
    const adjustedPosition = adjustPositionForClockDrift(syncState);
    const localPositionMs = currentTime * 1000;
    if (Math.abs(adjustedPosition - localPositionMs) > SEEK_THRESHOLD_MS) {
      setCurrentTime(adjustedPosition / 1000);
    }

    // Handle track change (need external handler since we don't have track loading logic)
    if (syncState.track_id !== currentTrack?.id && syncState.track_id) {
      onRemoteTrackChange?.(syncState.track_id);
    }

    // Clear source after microtask
    queueMicrotask(() => {
      if (stateSourceRef.current === 'remote') {
        stateSourceRef.current = null;
      }
    });
  }, [
    isActiveDevice, isPlaying, volume, currentTime, currentTrack?.id,
    play, pause, setVolume, toggleMute, setCurrentTime, onRemoteTrackChange
  ]);

  // Handle incoming seek sync
  const handleSeekSync = useCallback((positionMs: number, timestamp: number) => {
    if (isActiveDevice) return;

    stateSourceRef.current = 'remote';

    // Adjust for clock drift since the seek was issued
    const elapsed = Date.now() - timestamp;
    const adjustedPosition = positionMs + (isPlaying ? elapsed : 0);
    setCurrentTime(adjustedPosition / 1000);

    queueMicrotask(() => {
      if (stateSourceRef.current === 'remote') {
        stateSourceRef.current = null;
      }
    });
  }, [isActiveDevice, isPlaying, setCurrentTime]);

  // Handle incoming queue sync
  const handleQueueSync = useCallback((syncQueue: QueueState) => {
    if (isActiveDevice) return;

    stateSourceRef.current = 'remote';

    const { tracks, currentIndex } = fromSyncQueueState(syncQueue);

    // Convert to playerStore track format (adding missing fields)
    // NOTE: Tracks from sync have empty albumId - requires API fetch for album navigation
    // This is a known limitation. Full implementation would fetch track metadata.
    const playerTracks = tracks.map((t) => ({
      id: t.id,
      title: t.title,
      artist: t.artist,
      albumId: '', // SYNC_INCOMPLETE: Would need API fetch for album features
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
  }, [isActiveDevice, setQueue]);

  // Set up sync connection with handlers
  const {
    isConnected,
    sendPlaybackUpdate,
    sendQueueUpdate,
    requestTransfer,
  } = useSyncConnection({
    ...syncOptions,
    onPlaybackSync: handlePlaybackSync,
    onSeekSync: handleSeekSync,
    onQueueSync: handleQueueSync,
  });

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

  // Build current queue state
  const buildQueueState = useCallback((): QueueState => {
    const localTracks: LocalQueueTrack[] = queue.map((t) => ({
      id: t.id,
      title: t.title,
      artist: t.artist,
      albumTitle: t.albumTitle,
      duration: t.duration,
      coverUrl: t.coverUrl,
    }));
    return toSyncQueueState(localTracks, queueIndex);
  }, [queue, queueIndex]);

  // Broadcast playback state (throttled)
  const broadcastPlaybackState = useCallback(() => {
    if (!isConnected || !isActiveDevice) return;

    const now = Date.now();
    if (now - lastBroadcastRef.current < PLAYBACK_THROTTLE_MS) return;
    lastBroadcastRef.current = now;

    sendPlaybackUpdate(buildPlaybackState());
  }, [isConnected, isActiveDevice, sendPlaybackUpdate, buildPlaybackState]);

  // Broadcast queue state
  const broadcastQueueState = useCallback(() => {
    if (!isConnected || !isActiveDevice) return;
    sendQueueUpdate(buildQueueState());
  }, [isConnected, isActiveDevice, sendQueueUpdate, buildQueueState]);

  // Auto-broadcast on state changes (only if we're active and change is local)
  useEffect(() => {
    if (!isConnected || !isActiveDevice) return;
    if (stateSourceRef.current === 'remote') return;

    broadcastPlaybackState();
  }, [isConnected, isActiveDevice, isPlaying, volume, isMuted, shuffle, repeat, broadcastPlaybackState]);

  // NOTE: Explicit seek broadcast is not implemented yet.
  // Current behavior: Seek position is synced via playbackState broadcasts (throttled).
  // For more precise seek sync, consider:
  // 1. Add a seek action to playerStore that triggers a callback
  // 2. Have AudioProvider.seek() call a sync broadcast
  // 3. Track large position jumps (>2s) as seek events

  // Broadcast queue changes
  useEffect(() => {
    if (!isConnected || !isActiveDevice) return;
    if (stateSourceRef.current === 'remote') return;

    broadcastQueueState();
  }, [isConnected, isActiveDevice, queue, queueIndex, broadcastQueueState]);

  // Transfer to another device
  const transferToDevice = useCallback((targetDeviceId: string) => {
    if (!isConnected) return;
    requestTransfer(targetDeviceId);
  }, [isConnected, requestTransfer]);

  // Request to become active device
  const requestControl = useCallback(() => {
    if (!isConnected) return;
    // Transfer to self
    requestTransfer(deviceId);
  }, [isConnected, requestTransfer, deviceId]);

  return {
    isSyncActive: isConnected,
    isActiveDevice,
    broadcastPlaybackState,
    broadcastQueueState,
    transferToDevice,
    requestControl,
  };
}
