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
  | { type: 'SettingsUpdate'; payload: SyncedSettings }
  | { type: 'ChatSend'; payload: ChatSendPayload }
  | { type: 'ChatHistory'; payload: ChatHistoryPayload };

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
  | { type: 'ActiveDeviceChanged'; payload: { previous_device_id: string | null; new_device_id: string | null } }
  | { type: 'Pong'; payload: { server_time: number } }
  | { type: 'SettingsSync'; payload: SyncedSettings }
  | { type: 'ChatToken'; payload: ChatTokenPayload }
  | { type: 'ChatComplete'; payload: ChatCompletePayload }
  | { type: 'ChatError'; payload: ChatErrorPayload }
  | { type: 'ChatHistorySync'; payload: ChatHistorySyncPayload };

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
  album_id: string | null;
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
// Chat Payload Types
// =============================================================================

/** Role of a chat message sender */
export type ChatRole = 'user' | 'assistant' | 'system' | 'tool';

/** Payload for sending a chat message */
export interface ChatSendPayload {
  /** Optional conversation ID (null for new conversation) */
  conversation_id: string | null;
  /** Message content */
  message: string;
}

/** Payload for requesting chat history */
export interface ChatHistoryPayload {
  /** Conversation ID to fetch */
  conversation_id: string;
  /** Maximum messages to retrieve */
  limit?: number;
}

/** Streaming token from AI response */
export interface ChatTokenPayload {
  /** Conversation ID */
  conversation_id: string;
  /** Token content */
  token: string;
  /** Whether this is the final token */
  is_final: boolean;
}

/** Tool call from AI */
export interface ChatToolCall {
  /** Tool call ID */
  id: string;
  /** Tool name */
  name: string;
  /** Tool arguments as JSON string */
  arguments: string;
}

/** Tool result for execution */
export interface ChatToolResult {
  /** Tool call ID this result corresponds to */
  tool_call_id: string;
  /** Result content */
  content: string;
  /** Whether the tool execution succeeded */
  success: boolean;
}

/** Action the UI should execute */
export interface ChatAction {
  /** Action type */
  type: 'play_track' | 'add_to_queue' | 'create_playlist' | 'search_library' | 'get_recommendations';
  /** Action payload (varies by type) */
  payload: Record<string, unknown>;
}

/** Complete AI response */
export interface ChatCompletePayload {
  /** Conversation ID */
  conversation_id: string;
  /** Message ID of the saved response */
  message_id: string;
  /** Full response text */
  full_response: string;
  /** Actions for the UI to execute */
  actions: ChatAction[];
  /** Tool calls that were made */
  tool_calls?: ChatToolCall[];
  /** Server timestamp when message was created (ISO 8601) */
  created_at: string;
}

/** Chat error */
export interface ChatErrorPayload {
  /** Conversation ID (null if error occurred before conversation) */
  conversation_id: string | null;
  /** Error message */
  error: string;
  /** Error code */
  code?: string;
}

/** Chat history sync */
export interface ChatHistorySyncPayload {
  /** Conversation ID */
  conversation_id: string;
  /** Messages in the conversation */
  messages: ChatMessageData[];
}

/** Chat message data from server */
export interface ChatMessageData {
  /** Message ID */
  id: string;
  /** Conversation ID */
  conversation_id: string;
  /** Message role */
  role: ChatRole;
  /** Message content */
  content: string | null;
  /** Model used (for assistant messages) */
  model_used?: string;
  /** Token count */
  token_count?: number;
  /** Creation timestamp */
  created_at: string;
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
 * Generate a UUID with fallback for environments without crypto.randomUUID
 */
function generateUUID(): string {
  // Use native crypto.randomUUID if available (modern browsers)
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }

  // Fallback: use crypto.getRandomValues if available
  if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);
    // Set version (4) and variant (RFC 4122)
    // Using non-null assertion since we know the array has 16 elements
    bytes[6] = (bytes[6]! & 0x0f) | 0x40;
    bytes[8] = (bytes[8]! & 0x3f) | 0x80;
    const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
  }

  // Last resort fallback (not cryptographically secure, but functional)
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/** Canonical storage key for device identity (matches Zustand persist) */
const DEVICE_STORAGE_KEY = 'resonance-device';
/** Legacy key for backwards compatibility */
const LEGACY_DEVICE_ID_KEY = 'resonance-device-id';

/**
 * Generate a unique device ID (stored in localStorage)
 *
 * Uses the same storage key as Zustand persist to ensure consistency.
 * Also checks legacy key for backwards compatibility.
 */
export function getOrCreateDeviceId(): string {
  if (typeof localStorage === 'undefined') {
    return generateUUID();
  }

  // First, check the canonical Zustand persist storage
  const zustandData = localStorage.getItem(DEVICE_STORAGE_KEY);
  if (zustandData) {
    try {
      const parsed = JSON.parse(zustandData);
      if (parsed?.state?.deviceId) {
        return parsed.state.deviceId;
      }
    } catch {
      // JSON parse failed, continue to fallback
    }
  }

  // Check legacy key for backwards compatibility
  const legacyId = localStorage.getItem(LEGACY_DEVICE_ID_KEY);
  if (legacyId) {
    return legacyId;
  }

  // Generate new device ID and store in legacy key
  // (Zustand persist will pick it up and store in canonical location)
  const deviceId = generateUUID();
  localStorage.setItem(LEGACY_DEVICE_ID_KEY, deviceId);
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
  albumId?: string;
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
