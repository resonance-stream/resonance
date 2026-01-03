/**
 * Sync Event Emitter System
 *
 * Provides a typed event emitter for sync-related events. This decouples
 * sync logic from UI effects (like toast notifications) by allowing
 * components to subscribe to events without the sync system knowing
 * about the UI layer.
 *
 * Usage:
 * ```tsx
 * import { syncEvents, useSyncEvents } from '@/sync';
 *
 * // Emit an event
 * syncEvents.emit('connected', { deviceId: '123', sessionId: 'abc' });
 *
 * // Subscribe in a component
 * function MyComponent() {
 *   useSyncEvents('connected', (payload) => {
 *     console.log('Connected:', payload);
 *   });
 * }
 * ```
 */

import { useEffect, useRef } from 'react';

// =============================================================================
// Event Types
// =============================================================================

/**
 * Union type of all sync event names.
 *
 * Events are emitted by the sync system at various lifecycle points:
 * - `connected`: WebSocket connection established successfully
 * - `disconnected`: WebSocket connection closed
 * - `reconnecting`: Attempting to reconnect after connection loss
 * - `error`: An error occurred in the sync system
 * - `deviceJoined`: Another device joined the sync session
 * - `deviceLeft`: A device left the sync session
 * - `transferReceived`: This device received playback control from another device
 * - `transferSent`: This device transferred playback control to another device
 */
export type SyncEventType =
  | 'connected'
  | 'disconnected'
  | 'reconnecting'
  | 'error'
  | 'deviceJoined'
  | 'deviceLeft'
  | 'transferReceived'
  | 'transferSent';

/**
 * Payload for the 'connected' event.
 * Emitted when the WebSocket connection is established successfully.
 */
export interface ConnectedEventPayload {
  /** The unique identifier of this device */
  deviceId: string;
  /** The sync session ID this device joined */
  sessionId: string;
  /** True if this is a reconnection after a previous disconnect */
  isReconnect: boolean;
}

/**
 * Payload for the 'disconnected' event.
 * Emitted when the WebSocket connection is closed.
 */
export interface DisconnectedEventPayload {
  /** Human-readable reason for the disconnect, if available */
  reason?: string;
  /** True if the disconnect was intentional (clean close) */
  wasClean: boolean;
}

/**
 * Payload for the 'reconnecting' event.
 * Emitted when the sync system is attempting to reconnect after connection loss.
 */
export interface ReconnectingEventPayload {
  /** The current reconnection attempt number (1-indexed) */
  attempt: number;
  /** The maximum number of attempts before giving up, if configured */
  maxAttempts?: number;
}

/**
 * Payload for the 'error' event.
 * Emitted when an error occurs in the sync system.
 */
export interface ErrorEventPayload {
  /** Human-readable error message */
  message: string;
  /** Machine-readable error code, if available */
  code?: string;
  /** True if the error is related to authentication (e.g., expired token) */
  isAuthError: boolean;
}

/**
 * Payload for the 'deviceJoined' event.
 * Emitted when another device joins the sync session.
 */
export interface DeviceJoinedEventPayload {
  /** The unique identifier of the device that joined */
  deviceId: string;
  /** The human-readable name of the device (e.g., "Living Room Speaker") */
  deviceName: string;
}

/**
 * Payload for the 'deviceLeft' event.
 * Emitted when a device leaves the sync session.
 */
export interface DeviceLeftEventPayload {
  /** The unique identifier of the device that left */
  deviceId: string;
}

/**
 * Payload for the 'transferReceived' event.
 * Emitted when this device receives playback control from another device.
 * This device is now the active (controlling) device.
 */
export interface TransferReceivedEventPayload {
  /** The unique identifier of the device that transferred control */
  fromDeviceId: string;
  /** The human-readable name of the device, if available */
  fromDeviceName?: string;
}

/**
 * Payload for the 'transferSent' event.
 * Emitted when this device transfers playback control to another device.
 * This device is no longer the active device.
 */
export interface TransferSentEventPayload {
  /** The unique identifier of the device receiving control */
  toDeviceId: string;
  /** The human-readable name of the device, if available */
  toDeviceName?: string;
}

/**
 * Type-safe mapping of event names to their payload interfaces.
 * Used internally by the SyncEventEmitter for type inference.
 */
export interface SyncEventPayloads {
  connected: ConnectedEventPayload;
  disconnected: DisconnectedEventPayload;
  reconnecting: ReconnectingEventPayload;
  error: ErrorEventPayload;
  deviceJoined: DeviceJoinedEventPayload;
  deviceLeft: DeviceLeftEventPayload;
  transferReceived: TransferReceivedEventPayload;
  transferSent: TransferSentEventPayload;
}

// =============================================================================
// Event Emitter Class
// =============================================================================

/** Type-safe event handler function for a specific event type */
type SyncEventHandler<T extends SyncEventType> = (payload: SyncEventPayloads[T]) => void;

/** Wildcard event handler that receives all events */
type AnyEventHandler = (event: SyncEventType, payload: unknown) => void;

/**
 * Type-safe event emitter for sync-related events.
 *
 * Provides a publish/subscribe pattern for decoupling sync logic from UI effects.
 * Components can subscribe to events (e.g., to show toast notifications) without
 * the sync system needing to know about the UI layer.
 *
 * ## Features:
 * - **Type-safe**: Events and payloads are fully typed via generics
 * - **Specific subscriptions**: Subscribe to individual event types via `on()`
 * - **Wildcard subscriptions**: Subscribe to all events via `onAny()`
 * - **One-time subscriptions**: Subscribe once via `once()`
 * - **Error isolation**: Handler errors are caught and logged, not propagated
 *
 * @example
 * ```typescript
 * const emitter = new SyncEventEmitter();
 *
 * // Subscribe to a specific event
 * const unsubscribe = emitter.on('connected', (payload) => {
 *   console.log('Connected:', payload.deviceId);
 * });
 *
 * // Emit an event
 * emitter.emit('connected', { deviceId: '123', sessionId: 'abc', isReconnect: false });
 *
 * // Cleanup
 * unsubscribe();
 * ```
 */
export class SyncEventEmitter {
  private handlers: Map<SyncEventType, Set<SyncEventHandler<SyncEventType>>> = new Map();
  private wildcardHandlers: Set<AnyEventHandler> = new Set();

  /**
   * Subscribe to a specific event type
   *
   * @param event - The event type to subscribe to
   * @param handler - Callback function invoked when the event is emitted
   * @returns Unsubscribe function
   */
  on<T extends SyncEventType>(event: T, handler: SyncEventHandler<T>): () => void {
    if (!this.handlers.has(event)) {
      this.handlers.set(event, new Set());
    }
    const handlers = this.handlers.get(event)!;
    handlers.add(handler as SyncEventHandler<SyncEventType>);

    return () => {
      handlers.delete(handler as SyncEventHandler<SyncEventType>);
      if (handlers.size === 0) {
        this.handlers.delete(event);
      }
    };
  }

  /**
   * Subscribe to all events (wildcard handler)
   *
   * @param handler - Callback function invoked for any event
   * @returns Unsubscribe function
   */
  onAny(handler: AnyEventHandler): () => void {
    this.wildcardHandlers.add(handler);
    return () => {
      this.wildcardHandlers.delete(handler);
    };
  }

  /**
   * Subscribe to an event for a single emission only
   *
   * @param event - The event type to subscribe to
   * @param handler - Callback function invoked once when the event is emitted
   * @returns Unsubscribe function
   */
  once<T extends SyncEventType>(event: T, handler: SyncEventHandler<T>): () => void {
    const wrappedHandler = (payload: SyncEventPayloads[T]) => {
      unsubscribe();
      handler(payload);
    };
    const unsubscribe = this.on(event, wrappedHandler);
    return unsubscribe;
  }

  /**
   * Emit an event with a payload
   *
   * @param event - The event type to emit
   * @param payload - The event payload
   */
  emit<T extends SyncEventType>(event: T, payload: SyncEventPayloads[T]): void {
    // Notify specific handlers
    const handlers = this.handlers.get(event);
    if (handlers) {
      handlers.forEach((handler) => {
        try {
          handler(payload);
        } catch (error) {
          console.error(`[SyncEvents] Error in handler for ${event}:`, error);
        }
      });
    }

    // Notify wildcard handlers
    this.wildcardHandlers.forEach((handler) => {
      try {
        handler(event, payload);
      } catch (error) {
        console.error(`[SyncEvents] Error in wildcard handler for ${event}:`, error);
      }
    });
  }

  /**
   * Remove all handlers for a specific event type
   *
   * @param event - The event type to clear handlers for
   */
  off<T extends SyncEventType>(event: T): void {
    this.handlers.delete(event);
  }

  /**
   * Remove all handlers for all events
   */
  clear(): void {
    this.handlers.clear();
    this.wildcardHandlers.clear();
  }

  /**
   * Get the number of handlers for a specific event type
   *
   * @param event - The event type to check
   * @returns Number of handlers registered
   */
  listenerCount<T extends SyncEventType>(event: T): number {
    return this.handlers.get(event)?.size ?? 0;
  }
}

// =============================================================================
// Singleton Instance
// =============================================================================

/**
 * Global sync event emitter instance
 *
 * Use this singleton to emit and subscribe to sync events throughout the app.
 */
export const syncEvents = new SyncEventEmitter();

// =============================================================================
// React Hook
// =============================================================================

/**
 * Hook to subscribe to sync events
 *
 * Automatically cleans up the subscription when the component unmounts.
 *
 * @param event - The event type to subscribe to
 * @param handler - Callback function invoked when the event is emitted
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   useSyncEvents('connected', (payload) => {
 *     console.log('Connected:', payload.deviceId);
 *   });
 *
 *   return <div>...</div>;
 * }
 * ```
 */
export function useSyncEvents<T extends SyncEventType>(
  event: T,
  handler: SyncEventHandler<T>
): void {
  // Use ref to avoid recreating subscription on handler changes
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    const unsubscribe = syncEvents.on(event, (payload) => {
      handlerRef.current(payload);
    });

    return unsubscribe;
  }, [event]);
}

/**
 * Hook to subscribe to all sync events
 *
 * @param handler - Callback function invoked for any event
 *
 * @example
 * ```tsx
 * function DebugComponent() {
 *   useSyncEventsAll((event, payload) => {
 *     console.log(`[Sync] ${event}:`, payload);
 *   });
 *
 *   return <div>...</div>;
 * }
 * ```
 */
export function useSyncEventsAll(handler: AnyEventHandler): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    const unsubscribe = syncEvents.onAny((event, payload) => {
      handlerRef.current(event, payload);
    });

    return unsubscribe;
  }, []);
}
