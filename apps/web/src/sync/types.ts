/**
 * WebSocket message types for real-time synchronization
 *
 * These types mirror the backend message definitions in:
 * apps/api/src/websocket/messages.rs
 */

// =============================================================================
// Client -> Server Messages
// =============================================================================

export type ClientMessage =
  | { type: 'PlaybackStateUpdate'; payload: PlaybackState }
  | { type: 'Seek'; payload: { position_ms: number } }
  | { type: 'QueueUpdate'; payload: QueueState }
  | { type: 'TransferPlayback'; payload: { target_device_id: string } }
  | { type: 'RequestDeviceList' }
  | { type: 'Heartbeat' }
  | { type: 'SettingsUpdate'; payload: SyncedSettings };

// =============================================================================
// Server -> Client Messages
// =============================================================================

export type ServerMessage =
  | { type: 'Connected'; payload: ConnectedPayload }
  | { type: 'Error'; payload: ErrorPayload }
  | { type: 'PlaybackSync'; payload: PlaybackState }
  | { type: 'SeekSync'; payload: { position_ms: number; timestamp: number } }
  | { type: 'QueueSync'; payload: QueueState }
  | { type: 'DeviceList'; payload: DevicePresence[] }
  | { type: 'DeviceConnected'; payload: DevicePresence }
  | { type: 'DeviceDisconnected'; payload: { device_id: string } }
  | { type: 'TransferRequested'; payload: { from_device_id: string } }
  | { type: 'TransferAccepted'; payload: { to_device_id: string } }
  | { type: 'ActiveDeviceChanged'; payload: { previous_device_id: string | null; new_device_id: string } }
  | { type: 'Pong'; payload: { server_time: number } }
  | { type: 'SettingsSync'; payload: SyncedSettings };

// =============================================================================
// Payload Types
// =============================================================================

export interface ConnectedPayload {
  device_id: string;
  session_id: string;
  active_device_id: string | null;
}

export interface ErrorPayload {
  code: string;
  message: string;
}

/** Error codes from server */
export const ErrorCodes = {
  AUTH_FAILED: 'AUTH_FAILED',
  INVALID_MESSAGE: 'INVALID_MESSAGE',
  RATE_LIMITED: 'RATE_LIMITED',
  DEVICE_NOT_FOUND: 'DEVICE_NOT_FOUND',
  NOT_ACTIVE_DEVICE: 'NOT_ACTIVE_DEVICE',
} as const;

export type ErrorCode = (typeof ErrorCodes)[keyof typeof ErrorCodes];

/** Playback state for synchronization */
export interface PlaybackState {
  /** Currently playing track ID (null if nothing playing) */
  track_id: string | null;
  /** Whether playback is active */
  is_playing: boolean;
  /** Current position in milliseconds */
  position_ms: number;
  /** Unix timestamp (ms) when this state was captured */
  timestamp: number;
  /** Volume level (0.0 - 1.0) */
  volume: number;
  /** Whether audio is muted */
  is_muted: boolean;
  /** Shuffle mode enabled */
  shuffle: boolean;
  /** Repeat mode */
  repeat: RepeatMode;
}

/** Repeat mode options */
export type RepeatMode = 'off' | 'track' | 'queue';

/** Queue state for synchronization */
export interface QueueState {
  /** Tracks in queue */
  tracks: QueueTrack[];
  /** Current position in queue (index) */
  current_index: number;
}

/** Minimal track info for queue */
export interface QueueTrack {
  id: string;
  title: string;
  artist: string;
  album_title: string;
  duration_ms: number;
  cover_url: string | null;
}

/** Device presence information */
export interface DevicePresence {
  /** Unique device identifier */
  device_id: string;
  /** Human-readable device name */
  device_name: string;
  /** Type of device */
  device_type: DeviceType;
  /** Whether this device is currently controlling playback */
  is_active: boolean;
  /** Current track (if playing) */
  current_track: TrackSummary | null;
  /** Volume level */
  volume: number;
  /** Last activity timestamp (Unix ms) */
  last_seen: number;
}

/** Device type categories */
export type DeviceType = 'web' | 'desktop' | 'mobile' | 'tablet' | 'speaker' | 'unknown';

/** Minimal track info for presence */
export interface TrackSummary {
  id: string;
  title: string;
  artist: string;
}

/** Settings that are synced across devices */
export interface SyncedSettings {
  crossfade_enabled?: boolean;
  crossfade_duration?: number;
  gapless_enabled?: boolean;
  normalize_volume?: boolean;
}

// =============================================================================
// Connection State
// =============================================================================

/** WebSocket connection state */
export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'reconnecting';

/** Device info for initial connection */
export interface DeviceInfo {
  device_name: string;
  device_type: DeviceType;
}

// =============================================================================
// Utility Functions
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

/**
 * Detect device type from user agent
 */
export function detectDeviceType(): DeviceType {
  if (typeof navigator === 'undefined') return 'unknown';

  const ua = navigator.userAgent.toLowerCase();

  if (/tablet|ipad/i.test(ua)) return 'tablet';
  if (/mobile|iphone|android.*mobile/i.test(ua)) return 'mobile';
  if (/electron/i.test(ua)) return 'desktop';

  return 'web';
}

/**
 * Generate a unique device ID (stored in localStorage)
 */
export function getOrCreateDeviceId(): string {
  const STORAGE_KEY = 'resonance-device-id';

  if (typeof localStorage === 'undefined') {
    return crypto.randomUUID();
  }

  let deviceId = localStorage.getItem(STORAGE_KEY);
  if (!deviceId) {
    deviceId = crypto.randomUUID();
    localStorage.setItem(STORAGE_KEY, deviceId);
  }

  return deviceId;
}

/**
 * Get default device name
 */
export function getDefaultDeviceName(): string {
  if (typeof navigator === 'undefined') return 'Unknown Device';

  const type = detectDeviceType();
  const browserMatch = navigator.userAgent.match(/(Chrome|Firefox|Safari|Edge|Opera)/i);
  const browser = browserMatch ? browserMatch[1] : 'Browser';

  switch (type) {
    case 'mobile':
      return `${browser} on Mobile`;
    case 'tablet':
      return `${browser} on Tablet`;
    case 'desktop':
      return 'Resonance Desktop';
    default:
      return `${browser} Web Player`;
  }
}

// =============================================================================
// State Adapters (for playerStore integration in Step 4)
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

/**
 * Local queue track that can be converted to/from sync queue
 */
export interface LocalQueueTrack {
  id: string;
  title: string;
  artist: string;
  albumTitle: string;
  duration: number; // in seconds
  coverUrl?: string;
}

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
      albumTitle: t.album_title,
      duration: t.duration_ms / 1000,
      coverUrl: t.cover_url ?? undefined,
    })),
    currentIndex: sync.current_index,
  };
}
