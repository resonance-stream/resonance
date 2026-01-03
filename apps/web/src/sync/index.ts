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
} from './types';

export { ErrorCodes } from './types';

// State adapters (separated for better modularity)
export type { LocalPlayerState, LocalQueueTrack } from './adapters';

export {
  createPlaybackState,
  adjustPositionForClockDrift,
  toSyncPlaybackState,
  fromSyncPlaybackState,
  toSyncQueueState,
  fromSyncQueueState,
} from './adapters';

// WebSocket Client
export { WebSocketClient } from './WebSocketClient';
export type { WebSocketClientConfig, WebSocketClientEvents } from './WebSocketClient';

// React Hooks
export { useSyncConnection } from './useSyncConnection';
export type { UseSyncConnectionOptions, SyncConnectionState } from './useSyncConnection';

export { useSyncState } from './useSyncState';
export type { UseSyncStateOptions, SyncStateValue } from './useSyncState';

// Sync Events
export {
  syncEvents,
  SyncEventEmitter,
  useSyncEvents,
  useSyncEventsAll,
} from './syncEvents';
export type {
  SyncEventType,
  SyncEventPayloads,
  ConnectedEventPayload,
  DisconnectedEventPayload,
  ReconnectingEventPayload,
  ErrorEventPayload,
  DeviceJoinedEventPayload,
  DeviceLeftEventPayload,
  TransferReceivedEventPayload,
  TransferSentEventPayload,
} from './syncEvents';

// Sync Notifications
export { useSyncNotifications } from './useSyncNotifications';
export type { UseSyncNotificationsOptions } from './useSyncNotifications';

// Sync Event Emitter Hook
export { useSyncEventEmitter } from './useSyncEventEmitter';
export type { UseSyncEventEmitterOptions } from './useSyncEventEmitter';

// Utilities
export {
  detectDeviceType,
  getOrCreateDeviceId,
  getDefaultDeviceName,
} from './types';

export { fetchTrackById } from './fetchTrackById';

// Test utilities (exported for use in tests across the codebase)
export {
  // Factory functions
  createPlaybackState as createTestPlaybackState,
  createQueueState as createTestQueueState,
  createSyncQueueTrack,
  createLocalQueueTrack,
  createLocalQueue,
  createTrack,
  // Store reset utilities
  resetPlayerStore,
  resetDeviceStore,
  resetAllSyncStores,
  // Mock option factories
  createMockPlaybackSyncOptions,
  createMockQueueSyncOptions,
  createMockPlaybackSyncHandlers,
  createMockQueueSyncHandlers,
  // State source ref factory
  createStateSourceRef,
  // Test assertion helpers
  getPlayerState,
  getDeviceState,
  setAsActiveDevice,
  setAsPassiveDevice,
} from './test-utils';

export type {
  LocalQueueTrack as TestLocalQueueTrack,
  PlaybackSyncOptions,
  QueueSyncOptions,
  PlaybackSyncHandlers,
  QueueSyncHandlers,
} from './test-utils';
