/**
 * Shared Test Utilities for Sync Hooks
 *
 * This module provides common factory functions and utilities used across
 * sync hook tests. Centralizing these utilities ensures consistency and
 * reduces code duplication.
 *
 * @example
 * ```typescript
 * import {
 *   createPlaybackState,
 *   createQueueState,
 *   resetPlayerStore,
 *   resetAllSyncStores,
 * } from './test-utils';
 *
 * beforeEach(() => {
 *   resetAllSyncStores();
 * });
 *
 * it('handles playback sync', () => {
 *   const state = createPlaybackState({ is_playing: true });
 *   // ... test logic
 * });
 * ```
 */

import { vi } from 'vitest';
import type { PlaybackState, QueueState, QueueTrack, StateChangeSource } from './types';
import type { LocalQueueTrack } from './adapters';
import type { UsePlaybackSyncOptions } from './usePlaybackSync';
import type { UseQueueSyncOptions } from './useQueueSync';
import { usePlayerStore } from '../stores/playerStore';
import { useDeviceStore } from '../stores/deviceStore';
import type { Track } from '../stores/playerStore';

// Re-export options types from source hooks for test convenience
export type { UsePlaybackSyncOptions, UseQueueSyncOptions };

/**
 * @deprecated Use UsePlaybackSyncOptions instead
 */
export type PlaybackSyncOptions = UsePlaybackSyncOptions;

/**
 * @deprecated Use UseQueueSyncOptions instead
 */
export type QueueSyncOptions = UseQueueSyncOptions;

// =============================================================================
// Playback State Factory
// =============================================================================

/**
 * Default playback state values for testing
 */
const DEFAULT_PLAYBACK_STATE: PlaybackState = {
  track_id: 'track-1',
  is_playing: true,
  position_ms: 30000,
  timestamp: Date.now(),
  volume: 0.75,
  is_muted: false,
  shuffle: false,
  repeat: 'off',
};

/**
 * Create a PlaybackState for testing with optional overrides
 *
 * @param overrides - Partial PlaybackState to override defaults
 * @returns Complete PlaybackState object
 *
 * @example
 * ```typescript
 * // Create with defaults
 * const state = createPlaybackState();
 *
 * // Create with custom playing state
 * const playing = createPlaybackState({ is_playing: true, volume: 0.9 });
 *
 * // Create paused state
 * const paused = createPlaybackState({ is_playing: false });
 * ```
 */
export function createPlaybackState(
  overrides: Partial<PlaybackState> = {}
): PlaybackState {
  return {
    ...DEFAULT_PLAYBACK_STATE,
    timestamp: Date.now(), // Always use fresh timestamp
    ...overrides,
  };
}

// =============================================================================
// Queue State Factory
// =============================================================================

/**
 * Default sync queue track for testing (snake_case format)
 */
const DEFAULT_QUEUE_TRACK: QueueTrack = {
  id: 'track-1',
  title: 'Song One',
  artist: 'Artist A',
  album_id: 'album-1',
  album_title: 'Album X',
  duration_ms: 180000,
  cover_url: 'https://example.com/cover1.jpg',
};

/**
 * Create a QueueTrack (sync format) for testing with optional overrides
 *
 * @param overrides - Partial QueueTrack to override defaults
 * @returns Complete QueueTrack object in sync format (snake_case)
 *
 * @example
 * ```typescript
 * const track = createSyncQueueTrack({ id: 'custom-id', title: 'Custom Song' });
 * ```
 */
export function createSyncQueueTrack(
  overrides: Partial<QueueTrack> = {}
): QueueTrack {
  return {
    ...DEFAULT_QUEUE_TRACK,
    ...overrides,
  };
}

/**
 * Default queue state for testing
 */
const DEFAULT_QUEUE_STATE: QueueState = {
  tracks: [
    {
      id: 'track-1',
      title: 'Song One',
      artist: 'Artist A',
      album_id: 'album-1',
      album_title: 'Album X',
      duration_ms: 180000,
      cover_url: 'https://example.com/cover1.jpg',
    },
    {
      id: 'track-2',
      title: 'Song Two',
      artist: 'Artist B',
      album_id: null,
      album_title: 'Album Y',
      duration_ms: 240000,
      cover_url: null,
    },
  ],
  current_index: 0,
};

/**
 * Create a QueueState for testing with optional overrides
 *
 * @param overrides - Partial QueueState to override defaults
 * @returns Complete QueueState object
 *
 * @example
 * ```typescript
 * // Create with defaults
 * const queue = createQueueState();
 *
 * // Create with custom index
 * const atSecond = createQueueState({ current_index: 1 });
 *
 * // Create with custom tracks
 * const custom = createQueueState({
 *   tracks: [createSyncQueueTrack({ id: 'my-track' })],
 *   current_index: 0,
 * });
 * ```
 */
export function createQueueState(
  overrides: Partial<QueueState> = {}
): QueueState {
  return {
    ...DEFAULT_QUEUE_STATE,
    ...overrides,
  };
}

// =============================================================================
// Local Queue Track Factory
// =============================================================================

// Re-export LocalQueueTrack from adapters for convenience
export type { LocalQueueTrack } from './adapters';

/**
 * Default local queue track for testing (camelCase format)
 */
const DEFAULT_LOCAL_QUEUE_TRACK: LocalQueueTrack = {
  id: 'track-1',
  title: 'Local Song One',
  artist: 'Local Artist A',
  albumId: 'local-album-1',
  albumTitle: 'Local Album X',
  duration: 120,
  coverUrl: 'https://example.com/local1.jpg',
};

/**
 * Create a local queue track (playerStore format) for testing
 *
 * @param overrides - Partial LocalQueueTrack to override defaults
 * @returns Complete LocalQueueTrack object in local format (camelCase)
 *
 * @example
 * ```typescript
 * const track = createLocalQueueTrack({ id: 'my-track', duration: 200 });
 * ```
 */
export function createLocalQueueTrack(
  overrides: Partial<LocalQueueTrack> = {}
): LocalQueueTrack {
  return {
    ...DEFAULT_LOCAL_QUEUE_TRACK,
    ...overrides,
  };
}

/**
 * Create a default local queue array for testing
 *
 * @returns Array of Track for use in playerStore
 *
 * @example
 * ```typescript
 * usePlayerStore.setState({ queue: createLocalQueue() });
 * ```
 */
export function createLocalQueue(): Track[] {
  return [
    {
      id: 'track-1',
      title: 'Local Song One',
      artist: 'Local Artist A',
      albumId: 'local-album-1',
      albumTitle: 'Local Album X',
      duration: 120,
      coverUrl: 'https://example.com/local1.jpg',
    },
    {
      id: 'track-2',
      title: 'Local Song Two',
      artist: 'Local Artist B',
      albumId: 'local-album-2',
      albumTitle: 'Local Album Y',
      duration: 200,
      coverUrl: undefined,
    },
  ];
}

// =============================================================================
// Track Factory (for playerStore currentTrack)
// =============================================================================

/**
 * Default track for playerStore testing
 */
const DEFAULT_TRACK: Track = {
  id: 'track-1',
  title: 'Test Track',
  artist: 'Test Artist',
  albumId: 'album-1',
  albumTitle: 'Test Album',
  duration: 180,
  coverUrl: 'https://example.com/cover.jpg',
};

/**
 * Create a Track for playerStore testing
 *
 * @param overrides - Partial Track to override defaults
 * @returns Complete Track object
 *
 * @example
 * ```typescript
 * const track = createTrack({ id: 'my-track', title: 'My Song' });
 * usePlayerStore.setState({ currentTrack: track });
 * ```
 */
export function createTrack(overrides: Partial<Track> = {}): Track {
  return {
    ...DEFAULT_TRACK,
    ...overrides,
  };
}

// =============================================================================
// Store Reset Utilities
// =============================================================================

/**
 * Default player store state for testing
 */
const DEFAULT_PLAYER_STATE = {
  currentTrack: null,
  isPlaying: false,
  currentTime: 0,
  volume: 0.75,
  isMuted: false,
  isLoading: false,
  isBuffering: false,
  queue: [] as Track[],
  queueIndex: 0,
  shuffle: false,
  repeat: 'off' as const,
};

/**
 * Reset playerStore to default test state
 *
 * @example
 * ```typescript
 * beforeEach(() => {
 *   resetPlayerStore();
 * });
 * ```
 */
export function resetPlayerStore(): void {
  usePlayerStore.setState(DEFAULT_PLAYER_STATE);
}

/**
 * Default device store state for testing
 */
const DEFAULT_DEVICE_STATE = {
  connectionState: 'connected' as const,
  sessionId: 'test-session',
  lastError: null,
  reconnectAttempt: 0,
  deviceId: 'mock-device-id',
  deviceName: 'Mock Device',
  deviceType: 'web' as const,
  devices: [],
  activeDeviceId: 'mock-device-id', // Default to active
};

/**
 * Reset deviceStore to default test state
 *
 * @example
 * ```typescript
 * beforeEach(() => {
 *   resetDeviceStore();
 * });
 * ```
 */
export function resetDeviceStore(): void {
  useDeviceStore.setState(DEFAULT_DEVICE_STATE);
}

/**
 * Reset all sync-related stores to default test state
 *
 * This is the recommended reset function to use in beforeEach hooks
 * to ensure a clean state for each test.
 *
 * @example
 * ```typescript
 * beforeEach(() => {
 *   resetAllSyncStores();
 *   vi.clearAllMocks();
 * });
 * ```
 */
export function resetAllSyncStores(): void {
  resetPlayerStore();
  resetDeviceStore();
}

// =============================================================================
// Mock Option Factories
// =============================================================================

/**
 * Create mock options for usePlaybackSync hook
 *
 * @param overrides - Partial options to override defaults
 * @returns Complete options object with mock functions
 *
 * @example
 * ```typescript
 * const options = createMockPlaybackSyncOptions();
 * const { result } = renderHook(() => usePlaybackSync(options));
 *
 * // With custom send handler
 * const sendMock = vi.fn();
 * const options = createMockPlaybackSyncOptions({ sendPlaybackUpdate: sendMock });
 * ```
 */
export function createMockPlaybackSyncOptions(
  overrides: Partial<PlaybackSyncOptions> = {}
): PlaybackSyncOptions {
  return {
    isConnected: true,
    sendPlaybackUpdate: vi.fn(),
    stateSourceRef: { current: null },
    ...overrides,
  };
}

/**
 * Create mock options for useQueueSync hook
 *
 * @param overrides - Partial options to override defaults
 * @returns Complete options object with mock functions
 *
 * @example
 * ```typescript
 * const options = createMockQueueSyncOptions();
 * const { result } = renderHook(() => useQueueSync(options));
 * ```
 */
export function createMockQueueSyncOptions(
  overrides: Partial<QueueSyncOptions> = {}
): QueueSyncOptions {
  return {
    isConnected: true,
    sendQueueUpdate: vi.fn(),
    stateSourceRef: { current: null },
    ...overrides,
  };
}

/**
 * Handlers returned by usePlaybackSync hook
 */
export interface PlaybackSyncHandlers {
  handlePlaybackSync: (state: PlaybackState) => void;
  handleSeekSync: (positionMs: number, timestamp: number) => void;
  broadcastPlaybackState: () => void;
}

/**
 * Create mock handlers for testing components that use usePlaybackSync results
 *
 * @param overrides - Partial handlers to override defaults
 * @returns Complete handler object with mock functions
 *
 * @example
 * ```typescript
 * const handlers = createMockPlaybackSyncHandlers();
 * // Use in component tests that receive these handlers as props
 * ```
 */
export function createMockPlaybackSyncHandlers(
  overrides: Partial<PlaybackSyncHandlers> = {}
): PlaybackSyncHandlers {
  return {
    handlePlaybackSync: vi.fn(),
    handleSeekSync: vi.fn(),
    broadcastPlaybackState: vi.fn(),
    ...overrides,
  };
}

/**
 * Handlers returned by useQueueSync hook
 */
export interface QueueSyncHandlers {
  handleQueueSync: (state: QueueState) => void;
  broadcastQueueState: () => void;
}

/**
 * Create mock handlers for testing components that use useQueueSync results
 *
 * @param overrides - Partial handlers to override defaults
 * @returns Complete handler object with mock functions
 *
 * @example
 * ```typescript
 * const handlers = createMockQueueSyncHandlers();
 * // Use in component tests that receive these handlers as props
 * ```
 */
export function createMockQueueSyncHandlers(
  overrides: Partial<QueueSyncHandlers> = {}
): QueueSyncHandlers {
  return {
    handleQueueSync: vi.fn(),
    broadcastQueueState: vi.fn(),
    ...overrides,
  };
}

// =============================================================================
// State Source Ref Factory
// =============================================================================

/**
 * Create a StateChangeSource ref for testing loop prevention logic
 *
 * @param initial - Initial value for the ref
 * @returns MutableRefObject for StateChangeSource
 *
 * @example
 * ```typescript
 * // Default null (local changes)
 * const ref = createStateSourceRef();
 *
 * // Pre-set to remote (simulating incoming sync)
 * const remoteRef = createStateSourceRef('remote');
 * ```
 */
export function createStateSourceRef(
  initial: StateChangeSource = null
): React.MutableRefObject<StateChangeSource> {
  return { current: initial };
}

// =============================================================================
// Test Assertion Helpers
// =============================================================================

/**
 * Get current player store state for assertions
 *
 * @returns Current playerStore state
 *
 * @example
 * ```typescript
 * const { isPlaying, volume } = getPlayerState();
 * expect(isPlaying).toBe(true);
 * ```
 */
export function getPlayerState() {
  return usePlayerStore.getState();
}

/**
 * Get current device store state for assertions
 *
 * @returns Current deviceStore state
 *
 * @example
 * ```typescript
 * const { activeDeviceId } = getDeviceState();
 * expect(activeDeviceId).toBe('mock-device-id');
 * ```
 */
export function getDeviceState() {
  return useDeviceStore.getState();
}

/**
 * Set this device as active (controlling playback)
 *
 * @example
 * ```typescript
 * setAsActiveDevice();
 * // Now this device controls playback
 * ```
 */
export function setAsActiveDevice(): void {
  useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
}

/**
 * Set this device as passive (receiving sync from another device)
 *
 * @param activeDeviceId - ID of the active device (default: 'other-device-id')
 *
 * @example
 * ```typescript
 * setAsPassiveDevice();
 * // Now this device receives sync updates
 * ```
 */
export function setAsPassiveDevice(activeDeviceId: string = 'other-device-id'): void {
  useDeviceStore.setState({ activeDeviceId });
}
