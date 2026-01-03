/**
 * Sync Event Emitter Hook
 *
 * Handles emission of sync-related events based on connection state changes.
 * This hook is extracted from SyncProvider to improve separation of concerns
 * and testability. It monitors connection state and errors from the device store
 * and emits appropriate events via the syncEvents emitter.
 *
 * Events emitted:
 * - `connected`: When WebSocket connection is established (with isReconnect flag)
 * - `disconnected`: When WebSocket connection drops
 * - `reconnecting`: When attempting to reconnect
 * - `error`: When a connection error occurs (with isAuthError detection)
 *
 * @example
 * ```tsx
 * function SyncProvider({ children, enabled }: SyncProviderProps) {
 *   // Use the hook to emit events based on connection state
 *   useSyncEventEmitter({ enabled });
 *
 *   return <SyncContext.Provider value={...}>{children}</SyncContext.Provider>;
 * }
 * ```
 */

import { useEffect, useRef } from 'react';
import { useDeviceStore } from '../stores/deviceStore';
import { syncEvents } from './syncEvents';
import type { ConnectionState } from './types';

/**
 * Options for the useSyncEventEmitter hook.
 */
export interface UseSyncEventEmitterOptions {
  /** Whether sync event emission is enabled (default: true) */
  enabled?: boolean;
}

/**
 * Hook that emits sync events based on connection state changes.
 *
 * Monitors the device store for connection state and error changes,
 * and emits appropriate events via the syncEvents emitter.
 *
 * @param options - Configuration options
 */
export function useSyncEventEmitter(options: UseSyncEventEmitterOptions = {}): void {
  const { enabled = true } = options;

  // Get connection state and error from device store
  const connectionState = useDeviceStore((s) => s.connectionState);
  const connectionError = useDeviceStore((s) => s.lastError);
  const deviceId = useDeviceStore((s) => s.deviceId);
  const sessionId = useDeviceStore((s) => s.sessionId);

  // Track previous connection state for event emission
  const prevConnectionStateRef = useRef<ConnectionState>(connectionState);
  const hasConnectedRef = useRef(false);

  // Emit sync events for connection state changes
  useEffect(() => {
    // Skip if sync is disabled
    if (!enabled) return;

    const prevState = prevConnectionStateRef.current;
    prevConnectionStateRef.current = connectionState;

    // Emit disconnected event when connection drops
    if (connectionState === 'disconnected' && prevState === 'connected') {
      syncEvents.emit('disconnected', {
        reason: undefined,
        wasClean: false,
      });
    }

    // Emit reconnecting event
    if (connectionState === 'reconnecting' && prevState !== 'reconnecting') {
      syncEvents.emit('reconnecting', {
        attempt: 1,
      });
    }

    // Emit connected event when connection established
    if (connectionState === 'connected' && prevState !== 'connected') {
      const isReconnect = hasConnectedRef.current;
      hasConnectedRef.current = true;

      syncEvents.emit('connected', {
        deviceId,
        sessionId: sessionId ?? '',
        isReconnect,
      });
    }
  }, [connectionState, enabled, deviceId, sessionId]);

  // Emit error event for connection errors
  useEffect(() => {
    if (!enabled || !connectionError) return;

    // Check for auth-related errors
    const isAuthError = detectAuthError(connectionError);

    syncEvents.emit('error', {
      message: connectionError,
      isAuthError,
    });
  }, [connectionError, enabled]);
}

/**
 * Detects if an error message indicates an authentication failure.
 *
 * @param errorMessage - The error message to check
 * @returns True if the error appears to be authentication-related
 */
function detectAuthError(errorMessage: string): boolean {
  const lowerCaseError = errorMessage.toLowerCase();
  return (
    lowerCaseError.includes('auth') ||
    lowerCaseError.includes('unauthorized') ||
    lowerCaseError.includes('token') ||
    lowerCaseError.includes('401')
  );
}
