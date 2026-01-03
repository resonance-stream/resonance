/**
 * State Adapters for Sync Module
 *
 * This module provides conversion functions between local player state
 * (used by playerStore) and sync state (used for WebSocket communication).
 *
 * Separating adapters from types follows the Single Responsibility Principle:
 * - types.ts: Pure type/interface definitions
 * - adapters.ts: State transformation logic
 */

import type { PlaybackState, QueueState, RepeatMode } from './types';

// =============================================================================
// Local State Types
// =============================================================================

/**
 * Local player state that can be converted to/from sync state
 * This matches the shape used in playerStore
 */
export interface LocalPlayerState {
  trackId: string | null;
  isPlaying: boolean;
  currentTime: number; // in seconds
  volume: number;
  isMuted: boolean;
  shuffle: boolean;
  repeat: RepeatMode;
}

/**
 * Local queue track that can be converted to/from sync queue
 */
export interface LocalQueueTrack {
  id: string;
  title: string;
  artist: string;
  albumId?: string;
  albumTitle: string;
  duration: number; // in seconds
  coverUrl?: string;
}

// =============================================================================
// Playback State Utilities
// =============================================================================

/**
 * Create a playback state with current timestamp
 */
export function createPlaybackState(
  partial: Partial<Omit<PlaybackState, 'timestamp'>>
): PlaybackState {
  return {
    track_id: null,
    is_playing: false,
    position_ms: 0,
    timestamp: Date.now(),
    volume: 1.0,
    is_muted: false,
    shuffle: false,
    repeat: 'off',
    ...partial,
  };
}

/**
 * Maximum allowed time drift in milliseconds (5 seconds)
 * If drift exceeds this, we assume clock desync and don't adjust
 */
const MAX_CLOCK_DRIFT_MS = 5000;

/**
 * Calculate adjusted position accounting for clock drift
 *
 * Bounds the adjustment to prevent extreme corrections from clock desync
 * or stale state data.
 */
export function adjustPositionForClockDrift(
  state: PlaybackState,
  currentTime: number = Date.now()
): number {
  if (!state.is_playing) {
    return state.position_ms;
  }

  const elapsed = currentTime - state.timestamp;

  // Bound the adjustment to prevent extreme corrections
  // Negative elapsed could mean clock desync, positive could be stale data
  if (elapsed < -MAX_CLOCK_DRIFT_MS || elapsed > MAX_CLOCK_DRIFT_MS) {
    console.warn(`[Sync] Clock drift out of bounds: ${elapsed}ms, using raw position`);
    return state.position_ms;
  }

  return Math.max(0, state.position_ms + elapsed);
}

// =============================================================================
// Playback State Adapters
// =============================================================================

/**
 * Convert local player state to sync playback state
 */
export function toSyncPlaybackState(local: LocalPlayerState): PlaybackState {
  return {
    track_id: local.trackId,
    is_playing: local.isPlaying,
    position_ms: Math.round(local.currentTime * 1000),
    timestamp: Date.now(),
    volume: local.volume,
    is_muted: local.isMuted,
    shuffle: local.shuffle,
    repeat: local.repeat,
  };
}

/**
 * Convert sync playback state to local player state updates
 * Returns partial state that can be applied to playerStore
 */
export function fromSyncPlaybackState(sync: PlaybackState): LocalPlayerState {
  return {
    trackId: sync.track_id,
    isPlaying: sync.is_playing,
    currentTime: sync.position_ms / 1000,
    volume: sync.volume,
    isMuted: sync.is_muted,
    shuffle: sync.shuffle,
    repeat: sync.repeat,
  };
}

// =============================================================================
// Queue State Adapters
// =============================================================================

/**
 * Convert local queue to sync queue state
 */
export function toSyncQueueState(
  tracks: LocalQueueTrack[],
  currentIndex: number
): QueueState {
  return {
    tracks: tracks.map((t) => ({
      id: t.id,
      title: t.title,
      artist: t.artist,
      album_id: t.albumId ?? null,
      album_title: t.albumTitle,
      duration_ms: Math.round(t.duration * 1000),
      cover_url: t.coverUrl ?? null,
    })),
    current_index: currentIndex,
  };
}

/**
 * Convert sync queue state to local queue format
 */
export function fromSyncQueueState(sync: QueueState): {
  tracks: LocalQueueTrack[];
  currentIndex: number;
} {
  return {
    tracks: sync.tracks.map((t) => ({
      id: t.id,
      title: t.title,
      artist: t.artist,
      albumId: t.album_id ?? undefined,
      albumTitle: t.album_title,
      duration: t.duration_ms / 1000,
      coverUrl: t.cover_url ?? undefined,
    })),
    currentIndex: sync.current_index,
  };
}
