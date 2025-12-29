/**
 * Real-time cross-device synchronization module
 *
 * This module provides WebSocket-based synchronization for playback state,
 * queue management, and device presence across multiple devices.
 *
 * @example
 * ```tsx
 * import { useSyncConnection } from '@/sync';
 *
 * function MyComponent() {
 *   const { isConnected, sendPlaybackUpdate } = useSyncConnection({
 *     onPlaybackSync: (state) => {
 *       // Handle incoming playback state from other devices
 *     },
 *   });
 *
 *   return <div>Connected: {isConnected ? 'Yes' : 'No'}</div>;
 * }
 * ```
 */

// Types
export type {
  ClientMessage,
  ServerMessage,
  ConnectedPayload,
  ErrorPayload,
  PlaybackState,
  QueueState,
  QueueTrack,
  DevicePresence,
  DeviceType,
  RepeatMode,
  TrackSummary,
  SyncedSettings,
  ConnectionState,
  DeviceInfo,
  // State adapters
  LocalPlayerState,
  LocalQueueTrack,
} from './types';

export {
  ErrorCodes,
  createPlaybackState,
  adjustPositionForClockDrift,
  // State adapters
  toSyncPlaybackState,
  fromSyncPlaybackState,
  toSyncQueueState,
  fromSyncQueueState,
} from './types';

// WebSocket Client
export { WebSocketClient } from './WebSocketClient';
export type { WebSocketClientConfig, WebSocketClientEvents } from './WebSocketClient';

// React Hooks
export { useSyncConnection } from './useSyncConnection';
export type { UseSyncConnectionOptions, SyncConnectionState } from './useSyncConnection';

export { useSyncState } from './useSyncState';
export type { UseSyncStateOptions, SyncStateValue } from './useSyncState';

// Utilities
export {
  detectDeviceType,
  getOrCreateDeviceId,
  getDefaultDeviceName,
} from './types';
