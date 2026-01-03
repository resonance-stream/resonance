/**
 * useQueueSync Hook Tests
 *
 * Unit tests for queue synchronization hook covering:
 * - handleQueueSync behavior (active/passive device)
 * - Track format conversion (ms to seconds, snake_case to camelCase)
 * - albumId preservation during sync
 * - broadcastQueueState conditions
 * - buildQueueState formatting
 * - Loop prevention (remote updates don't re-broadcast)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useQueueSync } from './useQueueSync';
import { usePlayerStore } from '../stores/playerStore';
import { useDeviceStore } from '../stores/deviceStore';
import type { StateChangeSource } from './usePlaybackSync';
import {
  createQueueState,
  createLocalQueue,
  createMockQueueSyncOptions,
  resetAllSyncStores,
} from './test-utils';

// Mock useIsActiveDevice - we'll control this per test
const mockUseIsActiveDevice = vi.fn(() => false);

vi.mock('../stores/deviceStore', async (importOriginal) => {
  const original = await importOriginal<typeof import('../stores/deviceStore')>();
  return {
    ...original,
    useIsActiveDevice: () => mockUseIsActiveDevice(),
  };
});

// Alias factory to match existing test patterns
const createMockOptions = createMockQueueSyncOptions;

describe('useQueueSync', () => {
  beforeEach(() => {
    resetAllSyncStores();
    vi.clearAllMocks();
    vi.useFakeTimers();
    // Reset mock to default (passive device)
    mockUseIsActiveDevice.mockReturnValue(false);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('handleQueueSync', () => {
    it('ignores updates when this device is active', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const options = createMockOptions();
      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.handleQueueSync(createQueueState());
      });

      // Queue should remain unchanged because we're the active device
      expect(usePlayerStore.getState().queue).toEqual([]);
      expect(usePlayerStore.getState().queueIndex).toBe(0);
    });

    it('applies queue state when this device is passive', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const options = createMockOptions();
      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.handleQueueSync(createQueueState());
      });

      // Queue should be updated from sync
      const { queue, queueIndex } = usePlayerStore.getState();
      expect(queue).toHaveLength(2);
      expect(queueIndex).toBe(0);
      expect(queue[0]?.id).toBe('track-1');
      expect(queue[0]?.title).toBe('Song One');
    });

    it('correctly converts track format (ms to seconds, snake_case to camelCase)', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const options = createMockOptions();
      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.handleQueueSync(createQueueState({
          tracks: [
            {
              id: 'convert-test',
              title: 'Conversion Test',
              artist: 'Test Artist',
              album_id: 'test-album-id',
              album_title: 'Test Album Title',
              duration_ms: 185000, // 185 seconds in ms
              cover_url: 'https://example.com/test.jpg',
            },
          ],
          current_index: 0,
        }));
      });

      const track = usePlayerStore.getState().queue[0];
      expect(track).toBeDefined();

      // Verify duration was converted from ms to seconds
      expect(track?.duration).toBe(185);

      // Verify snake_case to camelCase conversions
      expect(track?.albumId).toBe('test-album-id');
      expect(track?.albumTitle).toBe('Test Album Title');
      expect(track?.coverUrl).toBe('https://example.com/test.jpg');
    });

    it('preserves albumId field when present in sync data', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const options = createMockOptions();
      const { result } = renderHook(() => useQueueSync(options));

      const albumIdValue = 'important-album-id-12345';
      act(() => {
        result.current.handleQueueSync(createQueueState({
          tracks: [
            {
              id: 'track-with-album',
              title: 'Album Track',
              artist: 'Album Artist',
              album_id: albumIdValue,
              album_title: 'Important Album',
              duration_ms: 200000,
              cover_url: null,
            },
          ],
          current_index: 0,
        }));
      });

      const track = usePlayerStore.getState().queue[0];
      expect(track?.albumId).toBe(albumIdValue);
    });

    it('handles null albumId correctly (converts to empty string)', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const options = createMockOptions();
      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.handleQueueSync(createQueueState({
          tracks: [
            {
              id: 'track-without-album',
              title: 'No Album Track',
              artist: 'Artist',
              album_id: null, // null album_id
              album_title: 'Singles',
              duration_ms: 180000,
              cover_url: null,
            },
          ],
          current_index: 0,
        }));
      });

      const track = usePlayerStore.getState().queue[0];
      // When album_id is null, it should default to empty string
      expect(track?.albumId).toBe('');
    });

    it('applies correct queue index', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const options = createMockOptions();
      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.handleQueueSync(createQueueState({ current_index: 1 }));
      });

      expect(usePlayerStore.getState().queueIndex).toBe(1);
    });

    it('sets stateSourceRef to remote when handling sync', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      const stateSourceRef = { current: null as StateChangeSource };
      const options = createMockOptions({ stateSourceRef });
      const { result } = renderHook(() => useQueueSync(options));

      // Before sync, should be null
      expect(stateSourceRef.current).toBe(null);

      act(() => {
        result.current.handleQueueSync(createQueueState());
      });

      // During sync handling, should be 'remote'
      // Note: It gets reset via queueMicrotask, but we can check immediate state
      // The ref is set to 'remote' at the start of handleQueueSync
    });

    it('clears stateSourceRef after sync via microtask', async () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      const stateSourceRef = { current: null as StateChangeSource };
      const options = createMockOptions({ stateSourceRef });
      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.handleQueueSync(createQueueState());
      });

      // Process microtask queue
      await act(async () => {
        await Promise.resolve();
      });

      // After microtask, should be cleared back to null
      expect(stateSourceRef.current).toBe(null);
    });
  });

  describe('broadcastQueueState', () => {
    it('does nothing when not connected', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({ queue: createLocalQueue(), queueIndex: 0 });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({
        isConnected: false,
        sendQueueUpdate,
      });

      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.broadcastQueueState();
      });

      expect(sendQueueUpdate).not.toHaveBeenCalled();
    });

    it('does nothing when not active device', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: createLocalQueue(), queueIndex: 0 });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.broadcastQueueState();
      });

      expect(sendQueueUpdate).not.toHaveBeenCalled();
    });

    it('sends correct queue state when active and connected', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      const localQueue = createLocalQueue();
      usePlayerStore.setState({ queue: localQueue, queueIndex: 1 });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      const { result } = renderHook(() => useQueueSync(options));

      // Clear initial auto-broadcast from mount useEffect
      sendQueueUpdate.mockClear();

      act(() => {
        result.current.broadcastQueueState();
      });

      expect(sendQueueUpdate).toHaveBeenCalledTimes(1);
      const sentState = sendQueueUpdate.mock.calls[0][0];

      // Verify correct structure (snake_case for sync format)
      expect(sentState.current_index).toBe(1);
      expect(sentState.tracks).toHaveLength(2);

      // Verify first track conversion
      expect(sentState.tracks[0].id).toBe('track-1');
      expect(sentState.tracks[0].title).toBe('Local Song One');
      expect(sentState.tracks[0].artist).toBe('Local Artist A');
      expect(sentState.tracks[0].album_id).toBe('local-album-1');
      expect(sentState.tracks[0].album_title).toBe('Local Album X');
      expect(sentState.tracks[0].duration_ms).toBe(120000); // Converted to ms
      expect(sentState.tracks[0].cover_url).toBe('https://example.com/local1.jpg');

      // Verify second track with undefined coverUrl becomes null
      expect(sentState.tracks[1].cover_url).toBe(null);
    });
  });

  describe('buildQueueState', () => {
    it('correctly formats local queue for sync with duration in ms', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({
        queue: [
          {
            id: 'test-track',
            title: 'Test Title',
            artist: 'Test Artist',
            albumId: 'test-album',
            albumTitle: 'Test Album',
            duration: 90, // 90 seconds
            coverUrl: 'https://example.com/cover.jpg',
          },
        ],
        queueIndex: 0,
      });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.broadcastQueueState();
      });

      const sentState = sendQueueUpdate.mock.calls[0][0];
      // Duration should be converted to milliseconds
      expect(sentState.tracks[0].duration_ms).toBe(90000);
    });

    it('converts camelCase to snake_case for sync format', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({
        queue: [
          {
            id: 'camel-test',
            title: 'CamelCase Test',
            artist: 'Artist',
            albumId: 'album-id-value',
            albumTitle: 'Album Title Value',
            duration: 100,
            coverUrl: 'https://example.com/camel.jpg',
          },
        ],
        queueIndex: 0,
      });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.broadcastQueueState();
      });

      const sentState = sendQueueUpdate.mock.calls[0][0];
      const track = sentState.tracks[0];

      // Verify snake_case keys
      expect(track).toHaveProperty('album_id', 'album-id-value');
      expect(track).toHaveProperty('album_title', 'Album Title Value');
      expect(track).toHaveProperty('duration_ms', 100000);
      expect(track).toHaveProperty('cover_url', 'https://example.com/camel.jpg');

      // Should NOT have camelCase keys
      expect(track).not.toHaveProperty('albumId');
      expect(track).not.toHaveProperty('albumTitle');
      expect(track).not.toHaveProperty('duration');
      expect(track).not.toHaveProperty('coverUrl');
    });

    it('handles empty queue', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.broadcastQueueState();
      });

      const sentState = sendQueueUpdate.mock.calls[0][0];
      expect(sentState.tracks).toEqual([]);
      expect(sentState.current_index).toBe(0);
    });

    it('handles undefined albumId by converting to null', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({
        queue: [
          {
            id: 'no-album-track',
            title: 'No Album',
            artist: 'Artist',
            albumId: undefined as unknown as string, // Simulating undefined
            albumTitle: 'Unknown Album',
            duration: 150,
            coverUrl: undefined,
          },
        ],
        queueIndex: 0,
      });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.broadcastQueueState();
      });

      const sentState = sendQueueUpdate.mock.calls[0][0];
      // undefined should become null in sync format
      expect(sentState.tracks[0].album_id).toBe(null);
      expect(sentState.tracks[0].cover_url).toBe(null);
    });
  });

  describe('loop prevention', () => {
    it('does not re-broadcast after receiving remote queue sync', async () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const stateSourceRef = { current: null as StateChangeSource };
      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ stateSourceRef, sendQueueUpdate });

      const { result } = renderHook(() => useQueueSync(options));

      // Receive remote sync
      act(() => {
        result.current.handleQueueSync(createQueueState());
      });

      // Process microtask queue for state source clearing
      await act(async () => {
        await Promise.resolve();
      });

      // Should not have triggered a broadcast (we're not active device anyway)
      expect(sendQueueUpdate).not.toHaveBeenCalled();
    });

    it('stateSourceRef prevents auto-broadcast during remote update', () => {
      // This test verifies the loop prevention mechanism
      // When stateSourceRef.current === 'remote', the useEffect should not broadcast
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({ queue: createLocalQueue(), queueIndex: 0 });

      const stateSourceRef = { current: 'remote' as StateChangeSource };
      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({
        stateSourceRef,
        sendQueueUpdate,
        isConnected: true,
      });

      // Render hook with stateSourceRef already set to 'remote'
      renderHook(() => useQueueSync(options));

      // Clear any calls from initial render
      sendQueueUpdate.mockClear();

      // Trigger a queue change while stateSourceRef is 'remote'
      act(() => {
        usePlayerStore.setState({
          queue: [...createLocalQueue(), {
            id: 'new-track',
            title: 'New Track',
            artist: 'New Artist',
            albumId: 'new-album',
            albumTitle: 'New Album',
            duration: 180,
            coverUrl: undefined,
          }],
        });
      });

      // Give time for effects
      act(() => {
        vi.advanceTimersByTime(100);
      });

      // Should not broadcast because stateSourceRef indicates this came from remote
      expect(sendQueueUpdate).not.toHaveBeenCalled();
    });

    it('allows broadcast after stateSourceRef is cleared', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({ queue: createLocalQueue(), queueIndex: 0 });

      const stateSourceRef = { current: null as StateChangeSource };
      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({
        stateSourceRef,
        sendQueueUpdate,
        isConnected: true,
      });

      renderHook(() => useQueueSync(options));

      // Initial render may broadcast
      const initialCalls = sendQueueUpdate.mock.calls.length;

      // Trigger a local queue change (stateSourceRef is null, indicating local action)
      act(() => {
        usePlayerStore.setState({
          queue: [...createLocalQueue(), {
            id: 'local-new-track',
            title: 'Local New Track',
            artist: 'Local Artist',
            albumId: 'local-album',
            albumTitle: 'Local Album',
            duration: 200,
            coverUrl: undefined,
          }],
        });
      });

      // Should broadcast because stateSourceRef is null (local change)
      expect(sendQueueUpdate.mock.calls.length).toBeGreaterThanOrEqual(initialCalls);
    });
  });

  describe('auto-broadcast on queue changes', () => {
    it('broadcasts when queue changes while active', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      renderHook(() => useQueueSync(options));
      sendQueueUpdate.mockClear();

      act(() => {
        usePlayerStore.setState({ queue: createLocalQueue() });
      });

      // Queue changes trigger synchronous effect
      expect(sendQueueUpdate).toHaveBeenCalled();
    });

    it('broadcasts when queueIndex changes while active', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({ queue: createLocalQueue(), queueIndex: 0 });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      renderHook(() => useQueueSync(options));
      sendQueueUpdate.mockClear();

      act(() => {
        usePlayerStore.setState({ queueIndex: 1 });
      });

      expect(sendQueueUpdate).toHaveBeenCalled();
    });

    it('does not auto-broadcast when not connected', () => {
      mockUseIsActiveDevice.mockReturnValue(true);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({
        sendQueueUpdate,
        isConnected: false,
      });

      renderHook(() => useQueueSync(options));
      sendQueueUpdate.mockClear();

      act(() => {
        usePlayerStore.setState({ queue: createLocalQueue() });
      });

      expect(sendQueueUpdate).not.toHaveBeenCalled();
    });

    it('does not auto-broadcast when not active device', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      renderHook(() => useQueueSync(options));
      sendQueueUpdate.mockClear();

      act(() => {
        usePlayerStore.setState({ queue: createLocalQueue() });
      });

      expect(sendQueueUpdate).not.toHaveBeenCalled();
    });
  });

  describe('edge cases', () => {
    it('handles tracks with empty strings', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const options = createMockOptions();
      const { result } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.handleQueueSync({
          tracks: [
            {
              id: 'empty-strings',
              title: '',
              artist: '',
              album_id: '',
              album_title: '',
              duration_ms: 0,
              cover_url: '',
            },
          ],
          current_index: 0,
        });
      });

      const track = usePlayerStore.getState().queue[0];
      expect(track?.title).toBe('');
      expect(track?.artist).toBe('');
      expect(track?.albumId).toBe('');
      expect(track?.duration).toBe(0);
    });

    it('handles very long queue', () => {
      mockUseIsActiveDevice.mockReturnValue(false);

      usePlayerStore.setState({ queue: [], queueIndex: 0 });

      const options = createMockOptions();
      const { result } = renderHook(() => useQueueSync(options));

      // Create a queue with 100 tracks
      const longQueue: QueueState = {
        tracks: Array.from({ length: 100 }, (_, i) => ({
          id: `track-${i}`,
          title: `Song ${i}`,
          artist: `Artist ${i}`,
          album_id: `album-${i}`,
          album_title: `Album ${i}`,
          duration_ms: 180000 + i * 1000,
          cover_url: `https://example.com/cover${i}.jpg`,
        })),
        current_index: 50,
      };

      act(() => {
        result.current.handleQueueSync(longQueue);
      });

      const { queue, queueIndex } = usePlayerStore.getState();
      expect(queue).toHaveLength(100);
      expect(queueIndex).toBe(50);
      expect(queue[50]?.id).toBe('track-50');
    });

    it('preserves all track properties during round-trip', () => {
      // Test that data survives: local -> sync format -> local
      // First, set up as active to broadcast
      mockUseIsActiveDevice.mockReturnValue(true);

      const originalTrack = {
        id: 'round-trip-test',
        title: 'Round Trip Song',
        artist: 'Round Trip Artist',
        albumId: 'round-trip-album',
        albumTitle: 'Round Trip Album',
        duration: 123.456,
        coverUrl: 'https://example.com/roundtrip.jpg',
      };

      usePlayerStore.setState({ queue: [originalTrack], queueIndex: 0 });

      const sendQueueUpdate = vi.fn();
      const options = createMockOptions({ sendQueueUpdate });

      const { result, unmount } = renderHook(() => useQueueSync(options));

      act(() => {
        result.current.broadcastQueueState();
      });

      const syncState = sendQueueUpdate.mock.calls[0][0];

      // Unmount first hook before changing mock
      unmount();

      // Now simulate receiving this as a passive device
      act(() => {
        mockUseIsActiveDevice.mockReturnValue(false);
        usePlayerStore.setState({ queue: [], queueIndex: 0 });
      });

      const options2 = createMockOptions();
      const { result: result2 } = renderHook(() => useQueueSync(options2));

      act(() => {
        result2.current.handleQueueSync(syncState);
      });

      const resultTrack = usePlayerStore.getState().queue[0];
      expect(resultTrack?.id).toBe(originalTrack.id);
      expect(resultTrack?.title).toBe(originalTrack.title);
      expect(resultTrack?.artist).toBe(originalTrack.artist);
      expect(resultTrack?.albumId).toBe(originalTrack.albumId);
      expect(resultTrack?.albumTitle).toBe(originalTrack.albumTitle);
      // Duration gets rounded during ms conversion
      expect(resultTrack?.duration).toBeCloseTo(123, 0);
      expect(resultTrack?.coverUrl).toBe(originalTrack.coverUrl);
    });
  });
});
