/**
 * usePlaybackSync Hook Tests
 *
 * Comprehensive tests for the playback synchronization hook covering:
 * - handlePlaybackSync behavior (active vs passive device)
 * - handleSeekSync with clock drift compensation
 * - broadcastPlaybackState throttling
 * - Auto-broadcast on state changes
 * - Periodic position broadcast
 * - Track change broadcast (bypass throttle)
 * - Local seek detection
 * - Loop prevention (remote updates don't re-broadcast)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePlaybackSync, type UsePlaybackSyncOptions } from './usePlaybackSync';
import type { StateChangeSource } from './types';
import { usePlayerStore } from '../stores/playerStore';
import { useDeviceStore } from '../stores/deviceStore';
import {
  createPlaybackState,
  createMockPlaybackSyncOptions,
  resetAllSyncStores,
} from './test-utils';

// Alias factory to match existing test patterns
const createMockOptions = (
  overrides: Partial<UsePlaybackSyncOptions> = {}
): UsePlaybackSyncOptions => createMockPlaybackSyncOptions(overrides) as UsePlaybackSyncOptions;

describe('usePlaybackSync', () => {
  beforeEach(() => {
    resetAllSyncStores();
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('handlePlaybackSync', () => {
    it('ignores updates when this device is active', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({ isPlaying: false, volume: 0.5 });

      const stateSourceRef: React.MutableRefObject<StateChangeSource> = { current: null };
      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ stateSourceRef }))
      );

      act(() => {
        result.current.handlePlaybackSync(
          createPlaybackState({ is_playing: true, volume: 0.9 })
        );
      });

      // State should remain unchanged because we're the active device
      expect(usePlayerStore.getState().isPlaying).toBe(false);
      expect(usePlayerStore.getState().volume).toBe(0.5);
      // stateSourceRef should not be modified
      expect(stateSourceRef.current).toBeNull();
    });

    it('applies play/pause state when this device is passive', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ isPlaying: false });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      act(() => {
        result.current.handlePlaybackSync(createPlaybackState({ is_playing: true }));
      });

      expect(usePlayerStore.getState().isPlaying).toBe(true);
    });

    it('applies pause state when is_playing is false', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ isPlaying: true });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      act(() => {
        result.current.handlePlaybackSync(createPlaybackState({ is_playing: false }));
      });

      expect(usePlayerStore.getState().isPlaying).toBe(false);
    });

    it('applies volume when significantly different (> 0.01)', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ volume: 0.5 });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      act(() => {
        result.current.handlePlaybackSync(createPlaybackState({ volume: 0.8 }));
      });

      expect(usePlayerStore.getState().volume).toBe(0.8);
    });

    it('does not apply volume when difference is small (<= 0.01)', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ volume: 0.75 });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      // Difference of 0.005 is within tolerance
      act(() => {
        result.current.handlePlaybackSync(createPlaybackState({ volume: 0.755 }));
      });

      expect(usePlayerStore.getState().volume).toBe(0.75);
    });

    it('applies mute changes when state differs', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ isMuted: false });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      act(() => {
        result.current.handlePlaybackSync(createPlaybackState({ is_muted: true }));
      });

      expect(usePlayerStore.getState().isMuted).toBe(true);
    });

    it('does not toggle mute when states match', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ isMuted: true });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      // Already muted, should not toggle
      act(() => {
        result.current.handlePlaybackSync(createPlaybackState({ is_muted: true }));
      });

      expect(usePlayerStore.getState().isMuted).toBe(true);
    });

    it('applies position with clock drift adjustment when playing', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ currentTime: 0, isPlaying: true });

      const now = Date.now();
      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      // Position was 30s at timestamp 1 second ago
      act(() => {
        result.current.handlePlaybackSync(
          createPlaybackState({
            position_ms: 30000,
            timestamp: now - 1000,
            is_playing: true,
          })
        );
      });

      // Position should be ~31s (30s + 1s drift adjustment)
      expect(usePlayerStore.getState().currentTime).toBeCloseTo(31, 0);
    });

    it('does not adjust position for clock drift when paused', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ currentTime: 0, isPlaying: false });

      const now = Date.now();
      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      // When paused, no drift adjustment should occur
      act(() => {
        result.current.handlePlaybackSync(
          createPlaybackState({
            position_ms: 30000,
            timestamp: now - 2000,
            is_playing: false,
          })
        );
      });

      // Position should be exactly 30s (no drift adjustment for paused state)
      expect(usePlayerStore.getState().currentTime).toBe(30);
    });

    it('calls onRemoteTrackChange when track changes', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'old-track', title: 'Old', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
      });

      const onRemoteTrackChange = vi.fn();
      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ onRemoteTrackChange }))
      );

      act(() => {
        result.current.handlePlaybackSync(createPlaybackState({ track_id: 'new-track' }));
      });

      expect(onRemoteTrackChange).toHaveBeenCalledWith('new-track');
    });

    it('does not call onRemoteTrackChange when track is the same', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'T', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
      });

      const onRemoteTrackChange = vi.fn();
      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ onRemoteTrackChange }))
      );

      act(() => {
        result.current.handlePlaybackSync(createPlaybackState({ track_id: 'track-1' }));
      });

      expect(onRemoteTrackChange).not.toHaveBeenCalled();
    });
  });

  describe('handleSeekSync', () => {
    it('ignores seek when this device is active', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({ currentTime: 0 });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      act(() => {
        result.current.handleSeekSync(30000, Date.now());
      });

      expect(usePlayerStore.getState().currentTime).toBe(0);
    });

    it('applies seek when this device is passive', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ currentTime: 0, isPlaying: false });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      act(() => {
        result.current.handleSeekSync(30000, Date.now());
      });

      expect(usePlayerStore.getState().currentTime).toBe(30);
    });

    it('respects threshold - ignores small seeks (<= 1s difference)', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ currentTime: 30, isPlaying: false });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      // 500ms difference is within 1000ms threshold
      act(() => {
        result.current.handleSeekSync(30500, Date.now());
      });

      expect(usePlayerStore.getState().currentTime).toBe(30);
    });

    it('applies seek when difference exceeds threshold', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ currentTime: 30, isPlaying: false });

      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      // 2000ms difference exceeds 1000ms threshold
      act(() => {
        result.current.handleSeekSync(32000, Date.now());
      });

      expect(usePlayerStore.getState().currentTime).toBe(32);
    });

    it('adjusts position for clock drift when playing', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ currentTime: 0, isPlaying: true });

      const now = Date.now();
      const { result } = renderHook(() => usePlaybackSync(createMockOptions()));

      // Seek was 1 second ago
      act(() => {
        result.current.handleSeekSync(30000, now - 1000);
      });

      // Position should be ~31s (30s + 1s drift adjustment)
      expect(usePlayerStore.getState().currentTime).toBeCloseTo(31, 0);
    });
  });

  describe('broadcastPlaybackState', () => {
    it('does nothing when not connected', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      const sendPlaybackUpdate = vi.fn();

      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ isConnected: false, sendPlaybackUpdate }))
      );

      act(() => {
        result.current.broadcastPlaybackState();
      });

      expect(sendPlaybackUpdate).not.toHaveBeenCalled();
    });

    it('does nothing when not the active device', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      const sendPlaybackUpdate = vi.fn();

      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ sendPlaybackUpdate }))
      );

      act(() => {
        result.current.broadcastPlaybackState();
      });

      expect(sendPlaybackUpdate).not.toHaveBeenCalled();
    });

    it('sends correct state when active and connected', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: true,
        currentTime: 30,
        volume: 0.8,
        isMuted: false,
        shuffle: true,
        repeat: 'queue',
      });

      const sendPlaybackUpdate = vi.fn();
      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ sendPlaybackUpdate }))
      );

      act(() => {
        result.current.broadcastPlaybackState();
      });

      expect(sendPlaybackUpdate).toHaveBeenCalledWith(
        expect.objectContaining({
          track_id: 'track-1',
          is_playing: true,
          position_ms: 30000,
          volume: 0.8,
          is_muted: false,
          shuffle: true,
          repeat: 'queue',
        })
      );
    });

    it('throttles calls (250ms)', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: true,
      });

      const sendPlaybackUpdate = vi.fn();
      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ sendPlaybackUpdate }))
      );

      // First broadcast should work
      act(() => {
        result.current.broadcastPlaybackState();
      });
      expect(sendPlaybackUpdate).toHaveBeenCalledTimes(1);

      // Second immediate broadcast should be throttled
      act(() => {
        result.current.broadcastPlaybackState();
      });
      expect(sendPlaybackUpdate).toHaveBeenCalledTimes(1);

      // After 250ms, should work again
      act(() => {
        vi.advanceTimersByTime(250);
      });

      act(() => {
        result.current.broadcastPlaybackState();
      });
      expect(sendPlaybackUpdate).toHaveBeenCalledTimes(2);
    });
  });

  describe('auto-broadcast on state changes', () => {
    it('broadcasts when isPlaying changes', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: false,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      // Wait for throttle window to pass from any initial broadcasts
      act(() => {
        vi.advanceTimersByTime(300);
      });

      sendPlaybackUpdate.mockClear();

      // Change isPlaying
      act(() => {
        usePlayerStore.getState().play();
      });

      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });

    it('broadcasts when volume changes', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        volume: 0.5,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      act(() => {
        vi.advanceTimersByTime(300);
      });

      sendPlaybackUpdate.mockClear();

      act(() => {
        usePlayerStore.getState().setVolume(0.8);
      });

      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });

    it('broadcasts when mute state changes', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isMuted: false,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      act(() => {
        vi.advanceTimersByTime(300);
      });

      sendPlaybackUpdate.mockClear();

      act(() => {
        usePlayerStore.getState().toggleMute();
      });

      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });

    it('broadcasts when shuffle changes', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        shuffle: false,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      act(() => {
        vi.advanceTimersByTime(300);
      });

      sendPlaybackUpdate.mockClear();

      act(() => {
        usePlayerStore.getState().toggleShuffle();
      });

      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });

    it('broadcasts when repeat changes', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        repeat: 'off',
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      act(() => {
        vi.advanceTimersByTime(300);
      });

      sendPlaybackUpdate.mockClear();

      act(() => {
        usePlayerStore.getState().cycleRepeat();
      });

      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });

    it('does not broadcast when not active device', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: false,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      sendPlaybackUpdate.mockClear();

      act(() => {
        usePlayerStore.getState().play();
      });

      act(() => {
        vi.advanceTimersByTime(100);
      });

      expect(sendPlaybackUpdate).not.toHaveBeenCalled();
    });
  });

  describe('periodic position broadcast', () => {
    it('broadcasts position periodically while playing', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: true,
        currentTime: 0,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      // Clear initial calls
      sendPlaybackUpdate.mockClear();

      // Advance past the default 5 second interval
      act(() => {
        vi.advanceTimersByTime(5100);
      });

      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });

    it('respects configurable positionBroadcastInterval option', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: true,
        currentTime: 0,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() =>
        usePlaybackSync(
          createMockOptions({
            sendPlaybackUpdate,
            positionBroadcastInterval: 2000, // 2 seconds instead of default 5
          })
        )
      );

      sendPlaybackUpdate.mockClear();

      // Should not broadcast at 1.5 seconds
      act(() => {
        vi.advanceTimersByTime(1500);
      });
      expect(sendPlaybackUpdate).not.toHaveBeenCalled();

      // Should broadcast after 2 seconds total
      act(() => {
        vi.advanceTimersByTime(600);
      });
      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });

    it('does not broadcast periodically when paused', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: false,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      sendPlaybackUpdate.mockClear();

      act(() => {
        vi.advanceTimersByTime(10000);
      });

      // No periodic broadcasts when paused
      expect(sendPlaybackUpdate).not.toHaveBeenCalled();
    });

    it('does not broadcast periodically when not active device', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: true,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      sendPlaybackUpdate.mockClear();

      act(() => {
        vi.advanceTimersByTime(10000);
      });

      expect(sendPlaybackUpdate).not.toHaveBeenCalled();
    });
  });

  describe('immediate track change broadcast', () => {
    it('bypasses throttle on track change', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Song 1', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: true,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      // Trigger a broadcast to start throttle
      act(() => {
        usePlayerStore.getState().setVolume(0.9);
      });

      const initialCallCount = sendPlaybackUpdate.mock.calls.length;

      // Immediately change track (within throttle window)
      act(() => {
        usePlayerStore.setState({
          currentTrack: { id: 'track-2', title: 'Song 2', artist: 'B', albumId: '2', albumTitle: 'Y', duration: 200 },
        });
      });

      // Track changes bypass throttle and broadcast immediately
      expect(sendPlaybackUpdate.mock.calls.length).toBeGreaterThan(initialCallCount);
    });

    it('broadcasts immediately when first track is set', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: null,
        isPlaying: false,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      sendPlaybackUpdate.mockClear();

      // Set first track
      act(() => {
        usePlayerStore.setState({
          currentTrack: { id: 'track-1', title: 'Song 1', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        });
      });

      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });
  });

  describe('local seek detection', () => {
    it('broadcasts immediately on large local seek (>= 1s)', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: true,
        currentTime: 0,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      // Wait for throttle window to pass from initial renders
      act(() => {
        vi.advanceTimersByTime(300);
      });

      sendPlaybackUpdate.mockClear();

      // Large seek (> 1 second)
      act(() => {
        usePlayerStore.setState({ currentTime: 60 });
      });

      // Large local seeks trigger immediate broadcast
      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });

    it('throttles broadcasts on small position changes (does not bypass throttle like large seeks)', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: true,
        currentTime: 10,
      });

      const sendPlaybackUpdate = vi.fn();
      renderHook(() => usePlaybackSync(createMockOptions({ sendPlaybackUpdate })));

      // Trigger an initial broadcast
      act(() => {
        usePlayerStore.getState().setVolume(0.9);
      });

      const callsAfterVolume = sendPlaybackUpdate.mock.calls.length;

      // Small position change (0.5 second, simulating normal playback)
      // This should be throttled because it's within the 250ms throttle window
      act(() => {
        usePlayerStore.setState({ currentTime: 10.5 });
      });

      // Small position changes are throttled (unlike large seeks which bypass throttle)
      expect(sendPlaybackUpdate.mock.calls.length).toBe(callsAfterVolume);

      // After throttle window passes, another broadcast can go through
      act(() => {
        vi.advanceTimersByTime(250);
      });

      act(() => {
        usePlayerStore.setState({ currentTime: 11 });
      });

      // Now the broadcast should go through
      expect(sendPlaybackUpdate.mock.calls.length).toBeGreaterThan(callsAfterVolume);
    });
  });

  describe('loop prevention - remote updates do not re-broadcast', () => {
    it('does not broadcast after receiving remote playback sync', async () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: false,
      });

      const stateSourceRef: React.MutableRefObject<StateChangeSource> = { current: null };
      const sendPlaybackUpdate = vi.fn();

      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ stateSourceRef, sendPlaybackUpdate }))
      );

      sendPlaybackUpdate.mockClear();

      // Receive remote sync
      act(() => {
        result.current.handlePlaybackSync(
          createPlaybackState({ is_playing: true, volume: 0.9 })
        );
      });

      // stateSourceRef should be set to 'remote' during handling
      // (then cleared via queueMicrotask)
      expect(stateSourceRef.current).toBe('remote');

      // Process microtask queue for state source clearing
      await act(async () => {
        await Promise.resolve();
      });

      expect(stateSourceRef.current).toBeNull();
      // Should not have triggered a broadcast
      expect(sendPlaybackUpdate).not.toHaveBeenCalled();
    });

    it('does not broadcast after receiving remote seek sync', async () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        currentTime: 0,
        isPlaying: false,
      });

      const stateSourceRef: React.MutableRefObject<StateChangeSource> = { current: null };
      const sendPlaybackUpdate = vi.fn();

      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ stateSourceRef, sendPlaybackUpdate }))
      );

      sendPlaybackUpdate.mockClear();

      // Receive remote seek
      act(() => {
        result.current.handleSeekSync(30000, Date.now());
      });

      // Process microtask queue
      await act(async () => {
        await Promise.resolve();
      });

      expect(stateSourceRef.current).toBeNull();
      expect(sendPlaybackUpdate).not.toHaveBeenCalled();
    });

    it('blocks auto-broadcast when stateSourceRef is remote', () => {
      useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: false,
      });

      const stateSourceRef: React.MutableRefObject<StateChangeSource> = { current: 'remote' };
      const sendPlaybackUpdate = vi.fn();

      renderHook(() =>
        usePlaybackSync(createMockOptions({ stateSourceRef, sendPlaybackUpdate }))
      );

      act(() => {
        vi.advanceTimersByTime(300);
      });

      sendPlaybackUpdate.mockClear();

      // Change isPlaying - but stateSourceRef is 'remote', so should not broadcast
      act(() => {
        usePlayerStore.getState().play();
      });

      expect(sendPlaybackUpdate).not.toHaveBeenCalled();
    });

    it('allows broadcast after remote sync source is cleared', async () => {
      // Start as passive device
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({
        currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
        isPlaying: false,
      });

      const stateSourceRef: React.MutableRefObject<StateChangeSource> = { current: null };
      const sendPlaybackUpdate = vi.fn();

      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ stateSourceRef, sendPlaybackUpdate }))
      );

      // Receive remote sync
      act(() => {
        result.current.handlePlaybackSync(createPlaybackState());
      });

      // Process microtask queue to clear source
      await act(async () => {
        await Promise.resolve();
      });

      // Become active device
      act(() => {
        useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
      });

      sendPlaybackUpdate.mockClear();

      // Now a local action should broadcast
      act(() => {
        vi.advanceTimersByTime(300); // Past throttle window
      });

      act(() => {
        result.current.broadcastPlaybackState();
      });

      expect(sendPlaybackUpdate).toHaveBeenCalled();
    });
  });

  describe('stateSourceRef handling', () => {
    it('sets stateSourceRef to remote during handlePlaybackSync', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ isPlaying: false });

      const stateSourceRef: React.MutableRefObject<StateChangeSource> = { current: null };
      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ stateSourceRef }))
      );

      act(() => {
        result.current.handlePlaybackSync(createPlaybackState({ is_playing: true }));
      });

      // Should be 'remote' immediately after (cleared via queueMicrotask)
      expect(stateSourceRef.current).toBe('remote');
    });

    it('sets stateSourceRef to remote during handleSeekSync', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ currentTime: 0, isPlaying: false });

      const stateSourceRef: React.MutableRefObject<StateChangeSource> = { current: null };
      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ stateSourceRef }))
      );

      act(() => {
        result.current.handleSeekSync(30000, Date.now());
      });

      expect(stateSourceRef.current).toBe('remote');
    });

    it('clears stateSourceRef via queueMicrotask after handling', async () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
      usePlayerStore.setState({ isPlaying: false });

      const stateSourceRef: React.MutableRefObject<StateChangeSource> = { current: null };
      const { result } = renderHook(() =>
        usePlaybackSync(createMockOptions({ stateSourceRef }))
      );

      act(() => {
        result.current.handlePlaybackSync(createPlaybackState());
      });

      expect(stateSourceRef.current).toBe('remote');

      // Process microtask queue
      await act(async () => {
        await Promise.resolve();
      });

      expect(stateSourceRef.current).toBeNull();
    });
  });

  describe('configurable timing options', () => {
    describe('playbackThrottleMs', () => {
      it('respects custom playbackThrottleMs option for throttling', () => {
        useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
        usePlayerStore.setState({
          currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
          isPlaying: true,
        });

        const sendPlaybackUpdate = vi.fn();
        const { result } = renderHook(() =>
          usePlaybackSync(
            createMockOptions({
              sendPlaybackUpdate,
              playbackThrottleMs: 100, // 100ms instead of default 250ms
            })
          )
        );

        // First broadcast should work
        act(() => {
          result.current.broadcastPlaybackState();
        });
        expect(sendPlaybackUpdate).toHaveBeenCalledTimes(1);

        // Immediate second broadcast should be throttled
        act(() => {
          result.current.broadcastPlaybackState();
        });
        expect(sendPlaybackUpdate).toHaveBeenCalledTimes(1);

        // After 100ms (custom throttle), should work again
        act(() => {
          vi.advanceTimersByTime(100);
        });

        act(() => {
          result.current.broadcastPlaybackState();
        });
        expect(sendPlaybackUpdate).toHaveBeenCalledTimes(2);
      });

      it('uses default 250ms throttle when playbackThrottleMs not specified', () => {
        useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
        usePlayerStore.setState({
          currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
          isPlaying: true,
        });

        const sendPlaybackUpdate = vi.fn();
        const { result } = renderHook(() =>
          usePlaybackSync(createMockOptions({ sendPlaybackUpdate }))
        );

        act(() => {
          result.current.broadcastPlaybackState();
        });
        expect(sendPlaybackUpdate).toHaveBeenCalledTimes(1);

        // After 200ms (less than default 250ms), should still be throttled
        act(() => {
          vi.advanceTimersByTime(200);
        });

        act(() => {
          result.current.broadcastPlaybackState();
        });
        expect(sendPlaybackUpdate).toHaveBeenCalledTimes(1);

        // After 50ms more (total 250ms), should work
        act(() => {
          vi.advanceTimersByTime(50);
        });

        act(() => {
          result.current.broadcastPlaybackState();
        });
        expect(sendPlaybackUpdate).toHaveBeenCalledTimes(2);
      });
    });

    describe('seekThresholdMs', () => {
      it('respects custom seekThresholdMs for handleSeekSync', () => {
        useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
        usePlayerStore.setState({ currentTime: 30, isPlaying: false });

        const { result } = renderHook(() =>
          usePlaybackSync(
            createMockOptions({
              seekThresholdMs: 500, // 500ms instead of default 1000ms
            })
          )
        );

        // 400ms difference is within 500ms custom threshold - should be ignored
        act(() => {
          result.current.handleSeekSync(30400, Date.now());
        });
        expect(usePlayerStore.getState().currentTime).toBe(30);

        // 600ms difference exceeds 500ms custom threshold - should apply
        act(() => {
          result.current.handleSeekSync(30600, Date.now());
        });
        expect(usePlayerStore.getState().currentTime).toBeCloseTo(30.6, 1);
      });

      it('respects custom seekThresholdMs for handlePlaybackSync position sync', () => {
        useDeviceStore.setState({ activeDeviceId: 'other-device-id' });
        usePlayerStore.setState({ currentTime: 30, isPlaying: false });

        const { result } = renderHook(() =>
          usePlaybackSync(
            createMockOptions({
              seekThresholdMs: 2000, // 2000ms instead of default 1000ms
            })
          )
        );

        // 1500ms difference is within 2000ms custom threshold - should be ignored
        act(() => {
          result.current.handlePlaybackSync(
            createPlaybackState({ position_ms: 31500, is_playing: false })
          );
        });
        expect(usePlayerStore.getState().currentTime).toBe(30);

        // 2500ms difference exceeds 2000ms custom threshold - should apply
        act(() => {
          result.current.handlePlaybackSync(
            createPlaybackState({ position_ms: 32500, is_playing: false })
          );
        });
        expect(usePlayerStore.getState().currentTime).toBeCloseTo(32.5, 1);
      });

      it('respects custom seekThresholdMs for local seek detection broadcast', () => {
        useDeviceStore.setState({ activeDeviceId: 'mock-device-id' });
        usePlayerStore.setState({
          currentTrack: { id: 'track-1', title: 'Test', artist: 'A', albumId: '1', albumTitle: 'X', duration: 180 },
          isPlaying: true,
          currentTime: 0,
        });

        const sendPlaybackUpdate = vi.fn();
        renderHook(() =>
          usePlaybackSync(
            createMockOptions({
              sendPlaybackUpdate,
              seekThresholdMs: 3000, // 3000ms instead of default 1000ms
            })
          )
        );

        // Trigger an initial broadcast via volume change to start the throttle window
        act(() => {
          usePlayerStore.getState().setVolume(0.9);
        });

        const callsAfterVolume = sendPlaybackUpdate.mock.calls.length;

        // 2s seek from 0 to 2 is within 3000ms custom threshold
        // This should NOT bypass the throttle (only large seeks bypass throttle)
        act(() => {
          usePlayerStore.setState({ currentTime: 2 });
        });
        // Call count should be unchanged (throttled, and not a large seek)
        expect(sendPlaybackUpdate.mock.calls.length).toBe(callsAfterVolume);

        // Wait for throttle to pass
        act(() => {
          vi.advanceTimersByTime(250);
        });

        // Now a 4s seek from 2 to 6 exceeds 3000ms custom threshold - should trigger immediately
        act(() => {
          usePlayerStore.setState({ currentTime: 6 });
        });
        // Large seek should have triggered a broadcast
        expect(sendPlaybackUpdate.mock.calls.length).toBeGreaterThan(callsAfterVolume);
      });
    });
  });
});
