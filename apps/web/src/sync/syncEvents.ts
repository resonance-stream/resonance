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

/** Sync event types */
export type SyncEventType =
  | 'connected'
  | 'disconnected'
  | 'reconnecting'
  | 'error'
  | 'deviceJoined'
  | 'deviceLeft'
  | 'transferReceived'
  | 'transferSent';

/** Payload for 'connected' event */
export interface ConnectedEventPayload {
  deviceId: string;
  sessionId: string;
  isReconnect: boolean;
}

/** Payload for 'disconnected' event */
export interface DisconnectedEventPayload {
  reason?: string;
  wasClean: boolean;
}

/** Payload for 'reconnecting' event */
export interface ReconnectingEventPayload {
  attempt: number;
  maxAttempts?: number;
}

/** Payload for 'error' event */
export interface ErrorEventPayload {
  message: string;
  code?: string;
  isAuthError: boolean;
}

/** Payload for 'deviceJoined' event */
export interface DeviceJoinedEventPayload {
  deviceId: string;
  deviceName: string;
}

/** Payload for 'deviceLeft' event */
export interface DeviceLeftEventPayload {
  deviceId: string;
}

/** Payload for 'transferReceived' event */
export interface TransferReceivedEventPayload {
  fromDeviceId: string;
  fromDeviceName?: string;
}

/** Payload for 'transferSent' event */
export interface TransferSentEventPayload {
  toDeviceId: string;
  toDeviceName?: string;
}

/** Map of event types to their payloads */
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

type SyncEventHandler<T extends SyncEventType> = (payload: SyncEventPayloads[T]) => void;
type AnyEventHandler = (event: SyncEventType, payload: unknown) => void;

/**
 * Typed event emitter for sync events
 *
 * Provides type-safe event emission and subscription for sync-related events.
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
