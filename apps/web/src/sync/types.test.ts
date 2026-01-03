/**
 * Tests for sync type utilities
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  createPlaybackState,
  adjustPositionForClockDrift,
  toSyncPlaybackState,
  fromSyncPlaybackState,
  toSyncQueueState,
  fromSyncQueueState,
  detectDeviceType,
  getOrCreateDeviceId,
  getDefaultDeviceName,
  type PlaybackState,
  type LocalPlayerState,
  type LocalQueueTrack,
  type QueueState,
} from './types';

describe('createPlaybackState', () => {
  it('creates state with defaults', () => {
    const state = createPlaybackState({});

    expect(state.track_id).toBeNull();
    expect(state.is_playing).toBe(false);
    expect(state.position_ms).toBe(0);
    expect(state.timestamp).toBeGreaterThan(0);
    expect(state.volume).toBe(1.0);
    expect(state.is_muted).toBe(false);
    expect(state.shuffle).toBe(false);
    expect(state.repeat).toBe('off');
  });

  it('creates state with partial values', () => {
    const state = createPlaybackState({
      track_id: 'track-123',
      is_playing: true,
      position_ms: 30000,
      volume: 0.5,
    });

    expect(state.track_id).toBe('track-123');
    expect(state.is_playing).toBe(true);
    expect(state.position_ms).toBe(30000);
    expect(state.volume).toBe(0.5);
    // Defaults still applied
    expect(state.is_muted).toBe(false);
    expect(state.shuffle).toBe(false);
  });
});

describe('adjustPositionForClockDrift', () => {
  it('returns raw position when not playing', () => {
    const state: PlaybackState = {
      track_id: 'track-123',
      is_playing: false,
      position_ms: 30000,
      timestamp: Date.now() - 5000,
      volume: 1.0,
      is_muted: false,
      shuffle: false,
      repeat: 'off',
    };

    const adjusted = adjustPositionForClockDrift(state);
    expect(adjusted).toBe(30000);
  });

  it('adjusts position when playing', () => {
    const now = Date.now();
    const state: PlaybackState = {
      track_id: 'track-123',
      is_playing: true,
      position_ms: 30000,
      timestamp: now - 1000, // 1 second ago
      volume: 1.0,
      is_muted: false,
      shuffle: false,
      repeat: 'off',
    };

    const adjusted = adjustPositionForClockDrift(state, now);
    expect(adjusted).toBe(31000); // 30000 + 1000
  });

  it('clamps to zero for negative result', () => {
    const now = Date.now();
    const state: PlaybackState = {
      track_id: 'track-123',
      is_playing: true,
      position_ms: 500,
      timestamp: now + 2000, // Future timestamp (clock desync simulation)
      volume: 1.0,
      is_muted: false,
      shuffle: false,
      repeat: 'off',
    };

    // With future timestamp within bounds, elapsed is negative
    // But MAX_CLOCK_DRIFT bounds should catch extreme cases
    const adjusted = adjustPositionForClockDrift(state, now);
    expect(adjusted).toBeGreaterThanOrEqual(0);
  });

  it('returns raw position when drift exceeds bounds', () => {
    const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const now = Date.now();

    const state: PlaybackState = {
      track_id: 'track-123',
      is_playing: true,
      position_ms: 30000,
      timestamp: now - 10000, // 10 seconds ago - exceeds 5s bounds
      volume: 1.0,
      is_muted: false,
      shuffle: false,
      repeat: 'off',
    };

    const adjusted = adjustPositionForClockDrift(state, now);
    expect(adjusted).toBe(30000); // Raw position returned
    expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('Clock drift out of bounds'));

    consoleSpy.mockRestore();
  });
});

describe('toSyncPlaybackState', () => {
  it('converts local state to sync format', () => {
    const local: LocalPlayerState = {
      trackId: 'track-123',
      isPlaying: true,
      currentTime: 30.5, // seconds
      volume: 0.75,
      isMuted: false,
      shuffle: true,
      repeat: 'queue',
    };

    const sync = toSyncPlaybackState(local);

    expect(sync.track_id).toBe('track-123');
    expect(sync.is_playing).toBe(true);
    expect(sync.position_ms).toBe(30500); // Converted to ms
    expect(sync.timestamp).toBeGreaterThan(0);
    expect(sync.volume).toBe(0.75);
    expect(sync.is_muted).toBe(false);
    expect(sync.shuffle).toBe(true);
    expect(sync.repeat).toBe('queue');
  });

  it('handles null trackId', () => {
    const local: LocalPlayerState = {
      trackId: null,
      isPlaying: false,
      currentTime: 0,
      volume: 1.0,
      isMuted: false,
      shuffle: false,
      repeat: 'off',
    };

    const sync = toSyncPlaybackState(local);
    expect(sync.track_id).toBeNull();
  });
});

describe('fromSyncPlaybackState', () => {
  it('converts sync state to local format', () => {
    const sync: PlaybackState = {
      track_id: 'track-123',
      is_playing: true,
      position_ms: 30500,
      timestamp: Date.now(),
      volume: 0.75,
      is_muted: true,
      shuffle: false,
      repeat: 'track',
    };

    const local = fromSyncPlaybackState(sync);

    expect(local.trackId).toBe('track-123');
    expect(local.isPlaying).toBe(true);
    expect(local.currentTime).toBe(30.5); // Converted to seconds
    expect(local.volume).toBe(0.75);
    expect(local.isMuted).toBe(true);
    expect(local.shuffle).toBe(false);
    expect(local.repeat).toBe('track');
  });
});

describe('toSyncQueueState', () => {
  it('converts local queue to sync format', () => {
    const tracks: LocalQueueTrack[] = [
      {
        id: 'track-1',
        title: 'Song One',
        artist: 'Artist A',
        albumTitle: 'Album X',
        duration: 180.5, // seconds
        coverUrl: 'https://example.com/cover1.jpg',
      },
      {
        id: 'track-2',
        title: 'Song Two',
        artist: 'Artist B',
        albumTitle: 'Album Y',
        duration: 240,
        coverUrl: undefined,
      },
    ];

    const sync = toSyncQueueState(tracks, 1);

    expect(sync.current_index).toBe(1);
    expect(sync.tracks).toHaveLength(2);
    expect(sync.tracks[0]).toEqual({
      id: 'track-1',
      title: 'Song One',
      artist: 'Artist A',
      album_id: null,
      album_title: 'Album X',
      duration_ms: 180500,
      cover_url: 'https://example.com/cover1.jpg',
    });
    expect(sync.tracks[1]?.cover_url).toBeNull();
  });

  it('handles empty queue', () => {
    const sync = toSyncQueueState([], 0);

    expect(sync.current_index).toBe(0);
    expect(sync.tracks).toHaveLength(0);
  });
});

describe('fromSyncQueueState', () => {
  it('converts sync queue to local format', () => {
    const sync: QueueState = {
      tracks: [
        {
          id: 'track-1',
          title: 'Song One',
          artist: 'Artist A',
          album_id: 'album-1',
          album_title: 'Album X',
          duration_ms: 180500,
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

    const result = fromSyncQueueState(sync);

    expect(result.currentIndex).toBe(0);
    expect(result.tracks).toHaveLength(2);
    expect(result.tracks[0]).toEqual({
      id: 'track-1',
      title: 'Song One',
      artist: 'Artist A',
      albumId: 'album-1',
      albumTitle: 'Album X',
      duration: 180.5, // Converted to seconds
      coverUrl: 'https://example.com/cover1.jpg',
    });
    expect(result.tracks[1]?.coverUrl).toBeUndefined();
  });
});

describe('detectDeviceType', () => {
  const originalNavigator = global.navigator;

  afterEach(() => {
    Object.defineProperty(global, 'navigator', {
      value: originalNavigator,
      writable: true,
    });
  });

  it('returns unknown when navigator is undefined', () => {
    Object.defineProperty(global, 'navigator', {
      value: undefined,
      writable: true,
    });
    expect(detectDeviceType()).toBe('unknown');
  });

  it('detects tablet', () => {
    Object.defineProperty(global, 'navigator', {
      value: { userAgent: 'Mozilla/5.0 (iPad; CPU OS 14_0 like Mac OS X)' },
      writable: true,
    });
    expect(detectDeviceType()).toBe('tablet');
  });

  it('detects mobile', () => {
    Object.defineProperty(global, 'navigator', {
      value: { userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X)' },
      writable: true,
    });
    expect(detectDeviceType()).toBe('mobile');
  });

  it('detects desktop (electron)', () => {
    Object.defineProperty(global, 'navigator', {
      value: { userAgent: 'Mozilla/5.0 Electron/10.0.0' },
      writable: true,
    });
    expect(detectDeviceType()).toBe('desktop');
  });

  it('defaults to web', () => {
    Object.defineProperty(global, 'navigator', {
      value: { userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/90.0' },
      writable: true,
    });
    expect(detectDeviceType()).toBe('web');
  });
});

describe('getOrCreateDeviceId', () => {
  let mockStorage: Record<string, string>;

  beforeEach(() => {
    mockStorage = {};
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => mockStorage[key] ?? null,
      setItem: (key: string, value: string) => {
        mockStorage[key] = value;
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('creates new device ID when none exists', () => {
    const id = getOrCreateDeviceId();
    expect(id).toMatch(/^[0-9a-f-]{36}$/); // UUID format
    expect(mockStorage['resonance-device-id']).toBe(id);
  });

  it('returns existing device ID', () => {
    mockStorage['resonance-device-id'] = 'existing-id-123';
    const id = getOrCreateDeviceId();
    expect(id).toBe('existing-id-123');
  });
});

describe('getDefaultDeviceName', () => {
  const originalNavigator = global.navigator;

  afterEach(() => {
    Object.defineProperty(global, 'navigator', {
      value: originalNavigator,
      writable: true,
    });
  });

  it('returns Unknown Device when navigator is undefined', () => {
    Object.defineProperty(global, 'navigator', {
      value: undefined,
      writable: true,
    });
    expect(getDefaultDeviceName()).toBe('Unknown Device');
  });

  it('returns browser-based name for web', () => {
    Object.defineProperty(global, 'navigator', {
      value: { userAgent: 'Mozilla/5.0 Chrome/90.0' },
      writable: true,
    });
    expect(getDefaultDeviceName()).toBe('Chrome Web Player');
  });

  it('returns mobile name for mobile devices', () => {
    Object.defineProperty(global, 'navigator', {
      value: { userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 14_0) Safari/604.1' },
      writable: true,
    });
    expect(getDefaultDeviceName()).toBe('Safari on Mobile');
  });
});
