/**
 * WebSocket message types for real-time sync in Resonance
 */

import type { PlaybackState, Device, QueueAction } from './player.js';

// ============================================================================
// Base Message Types
// ============================================================================

/**
 * Base WebSocket message structure
 */
export interface WebSocketMessage<T extends string = string, P = unknown> {
  /** Message type identifier */
  type: T;
  /** Message payload */
  payload: P;
  /** Message ID for request/response correlation */
  id?: string;
  /** Timestamp when message was created */
  timestamp: string;
}

/**
 * Error response for WebSocket messages
 */
export interface WebSocketError {
  /** Error code */
  code: string;
  /** Error message */
  message: string;
  /** Original message ID if applicable */
  requestId?: string;
}

// ============================================================================
// Sync Message Types
// ============================================================================

/**
 * All possible sync message types (union)
 */
export type SyncMessage =
  | PlaybackSyncMessage
  | QueueSyncMessage
  | VolumeSyncMessage
  | SeekSyncMessage
  | DeviceSyncMessage
  | TransferSyncMessage;

/**
 * Playback state sync (play/pause/track change)
 */
export interface PlaybackSyncMessage extends WebSocketMessage<'sync:playback', {
  /** Updated playback state */
  state: Pick<PlaybackState, 'trackId' | 'isPlaying' | 'position'>;
  /** Device that initiated the change */
  sourceDeviceId: string;
}> {}

/**
 * Queue update sync
 */
export interface QueueSyncMessage extends WebSocketMessage<'sync:queue', {
  /** Queue modification action */
  action: QueueAction;
  /** Resulting queue state */
  queue: {
    ids: string[];
    index: number;
  };
  /** Device that initiated the change */
  sourceDeviceId: string;
}> {}

/**
 * Volume change sync
 */
export interface VolumeSyncMessage extends WebSocketMessage<'sync:volume', {
  /** New volume level (0-1) */
  volume: number;
  /** Whether muted */
  isMuted: boolean;
  /** Device that initiated the change */
  sourceDeviceId: string;
}> {}

/**
 * Seek position sync
 */
export interface SeekSyncMessage extends WebSocketMessage<'sync:seek', {
  /** New position in seconds */
  position: number;
  /** Device that initiated the change */
  sourceDeviceId: string;
}> {}

/**
 * Device status update
 */
export interface DeviceSyncMessage extends WebSocketMessage<'sync:device', {
  /** Updated device information */
  device: Device;
  /** Event type */
  event: 'connected' | 'disconnected' | 'updated';
}> {}

/**
 * Transfer playback to another device
 */
export interface TransferSyncMessage extends WebSocketMessage<'sync:transfer', {
  /** Target device ID */
  targetDeviceId: string;
  /** Whether to start playing immediately */
  startPlaying: boolean;
  /** Position to resume at (seconds) */
  resumePosition?: number;
}> {}

// ============================================================================
// Presence Message Types
// ============================================================================

/**
 * All possible presence message types
 */
export type PresenceMessage =
  | UserPresenceMessage
  | ListeningActivityMessage
  | TypingIndicatorMessage;

/**
 * User online/offline presence
 */
export interface UserPresenceMessage extends WebSocketMessage<'presence:user', {
  /** User ID */
  userId: string;
  /** Presence status */
  status: 'online' | 'away' | 'offline';
  /** Last seen timestamp (for offline) */
  lastSeen?: string;
  /** Active device info */
  activeDevice?: {
    id: string;
    name: string;
    type: Device['type'];
  };
}> {}

/**
 * Real-time listening activity (what someone is playing)
 */
export interface ListeningActivityMessage extends WebSocketMessage<'presence:listening', {
  /** User ID */
  userId: string;
  /** Currently playing track info (null if nothing playing) */
  nowPlaying: {
    trackId: string;
    trackTitle: string;
    artistName: string;
    albumTitle: string;
    coverUrl?: string;
    position: number;
    duration: number;
    isPlaying: boolean;
  } | null;
}> {}

/**
 * Typing indicator for chat/collaborative playlists
 */
export interface TypingIndicatorMessage extends WebSocketMessage<'presence:typing', {
  /** User ID */
  userId: string;
  /** Context where user is typing */
  context: {
    type: 'chat' | 'playlist_comment' | 'playlist_edit';
    id: string;
  };
  /** Whether user is currently typing */
  isTyping: boolean;
}> {}

// ============================================================================
// Notification Message Types
// ============================================================================

/**
 * Real-time notification messages
 */
export type NotificationMessage =
  | NewFollowerNotification
  | PlaylistUpdateNotification
  | NewReleaseNotification
  | SystemNotification;

/**
 * New follower notification
 */
export interface NewFollowerNotification extends WebSocketMessage<'notification:follower', {
  /** User who followed */
  follower: {
    id: string;
    username: string;
    displayName?: string;
    avatarUrl?: string;
  };
}> {}

/**
 * Playlist update notification
 */
export interface PlaylistUpdateNotification extends WebSocketMessage<'notification:playlist', {
  /** Playlist that was updated */
  playlist: {
    id: string;
    name: string;
    coverUrl?: string;
  };
  /** Update type */
  updateType: 'tracks_added' | 'tracks_removed' | 'metadata_changed';
  /** Number of tracks affected */
  trackCount?: number;
}> {}

/**
 * New release notification
 */
export interface NewReleaseNotification extends WebSocketMessage<'notification:release', {
  /** Artist info */
  artist: {
    id: string;
    name: string;
    imageUrl?: string;
  };
  /** New release info */
  release: {
    id: string;
    title: string;
    type: 'album' | 'single' | 'ep';
    coverUrl?: string;
    releaseDate: string;
  };
}> {}

/**
 * System notification
 */
export interface SystemNotification extends WebSocketMessage<'notification:system', {
  /** Notification level */
  level: 'info' | 'warning' | 'error';
  /** Title */
  title: string;
  /** Message body */
  message: string;
  /** Optional action */
  action?: {
    label: string;
    url: string;
  };
}> {}

// ============================================================================
// Command Message Types (Client → Server)
// ============================================================================

/**
 * Commands that clients can send to the server
 */
export type ClientCommand =
  | SubscribeCommand
  | UnsubscribeCommand
  | PlaybackCommand
  | QueueCommand
  | PresenceCommand;

/**
 * Subscribe to a channel
 */
export interface SubscribeCommand extends WebSocketMessage<'command:subscribe', {
  /** Channel to subscribe to */
  channel: ChannelType;
  /** Channel-specific ID (e.g., playlist ID for collaborative editing) */
  channelId?: string;
}> {}

/**
 * Unsubscribe from a channel
 */
export interface UnsubscribeCommand extends WebSocketMessage<'command:unsubscribe', {
  /** Channel to unsubscribe from */
  channel: ChannelType;
  /** Channel-specific ID */
  channelId?: string;
}> {}

/**
 * Playback control command
 */
export interface PlaybackCommand extends WebSocketMessage<'command:playback', {
  /** Action to perform */
  action: 'play' | 'pause' | 'next' | 'previous' | 'seek' | 'shuffle' | 'repeat';
  /** Action-specific data */
  data?: {
    position?: number;
    shuffle?: boolean;
    repeat?: 'off' | 'track' | 'queue';
  };
}> {}

/**
 * Queue manipulation command
 */
export interface QueueCommand extends WebSocketMessage<'command:queue', {
  /** Queue action to perform */
  action: QueueAction;
}> {}

/**
 * Presence update command
 */
export interface PresenceCommand extends WebSocketMessage<'command:presence', {
  /** Presence status to set */
  status: 'online' | 'away';
}> {}

/**
 * Available channel types for subscription
 */
export type ChannelType =
  | 'sync' // Playback sync across devices
  | 'presence' // User presence updates
  | 'notifications' // Personal notifications
  | 'playlist' // Collaborative playlist updates
  | 'activity' // Friend activity feed
  | 'library'; // Library updates (new imports, etc.)

// ============================================================================
// Connection Types
// ============================================================================

/**
 * WebSocket connection state
 */
export interface ConnectionState {
  /** Current connection status */
  status: 'connecting' | 'connected' | 'disconnected' | 'reconnecting';
  /** Latency in milliseconds */
  latency?: number;
  /** Number of reconnection attempts */
  reconnectAttempts: number;
  /** Last successful connection time */
  lastConnectedAt?: string;
  /** Subscribed channels */
  subscribedChannels: ChannelType[];
}

/**
 * Connection established response
 */
export interface ConnectionEstablished extends WebSocketMessage<'connection:established', {
  /** Session ID for this connection */
  sessionId: string;
  /** Server timestamp for clock sync */
  serverTime: string;
  /** Heartbeat interval in milliseconds */
  heartbeatInterval: number;
}> {}

/**
 * Heartbeat/ping message
 */
export interface HeartbeatMessage extends WebSocketMessage<'heartbeat', {
  /** Client timestamp for latency calculation */
  clientTime: string;
}> {}

/**
 * Heartbeat response/pong
 */
export interface HeartbeatResponse extends WebSocketMessage<'heartbeat:ack', {
  /** Original client timestamp */
  clientTime: string;
  /** Server timestamp */
  serverTime: string;
}> {}
