/**
 * Sync Provider
 *
 * Provides cross-device playback synchronization throughout the app.
 * Should be placed inside AudioProvider to have access to audio context.
 *
 * This provider:
 * - Manages WebSocket connection lifecycle
 * - Syncs playback state between devices
 * - Handles incoming sync commands
 * - Provides sync context to child components
 * - Emits sync events for UI effects (via syncEvents)
 */

import { createContext, useContext, useCallback, useMemo, useEffect, useRef, type ReactNode } from 'react';
import { useSyncState, type SyncStateValue } from '../sync/useSyncState';
import { usePlayerStore } from '../stores/playerStore';
import { useDeviceStore } from '../stores/deviceStore';
import { fetchTrackById } from '../sync/fetchTrackById';
import { syncEvents } from '../sync/syncEvents';

interface SyncProviderProps {
  children: ReactNode;
  /** Whether sync is enabled (default: true) */
  enabled?: boolean;
}

interface SyncContextValue extends SyncStateValue {
  /** Whether sync feature is enabled */
  enabled: boolean;
}

const SyncContext = createContext<SyncContextValue | null>(null);

/**
 * Hook to access sync context
 *
 * @throws Error if used outside SyncProvider
 */
export function useSync(): SyncContextValue {
  const context = useContext(SyncContext);
  if (!context) {
    throw new Error('useSync must be used within a SyncProvider');
  }
  return context;
}

/**
 * Hook to access sync context (nullable version)
 *
 * Returns null if sync is not available (outside provider or disabled)
 */
export function useSyncOptional(): SyncContextValue | null {
  return useContext(SyncContext);
}

export function SyncProvider({ children, enabled = true }: SyncProviderProps): JSX.Element {
  // Get queue management from playerStore
  const jumpToIndex = usePlayerStore((s) => s.jumpToIndex);

  // Handle remote track changes
  // When another device changes the track, we need to load it locally
  // Uses getState() to avoid stale closure issues with queue
  const handleRemoteTrackChange = useCallback(async (trackId: string) => {
    // Get current queue at execution time to avoid stale closures
    const currentQueue = usePlayerStore.getState().queue;
    const trackIndex = currentQueue.findIndex((t) => t.id === trackId);
    if (trackIndex !== -1) {
      // Track is in queue, jump to it
      jumpToIndex(trackIndex);
    } else {
      // Track not in queue - fetch from API and play it
      const track = await fetchTrackById(trackId);
      if (track) {
        usePlayerStore.getState().setTrack(track);
      } else {
        console.warn('[SyncProvider] Failed to fetch remote track:', trackId);
      }
    }
  }, [jumpToIndex]);

  // Initialize sync state
  const syncState = useSyncState({
    onRemoteTrackChange: handleRemoteTrackChange,
    autoConnect: enabled,
  });

  // Get connection state and error from device store
  const connectionState = useDeviceStore((s) => s.connectionState);
  const connectionError = useDeviceStore((s) => s.lastError);
  const deviceId = useDeviceStore((s) => s.deviceId);

  // Track previous connection state for event emission
  const prevConnectionStateRef = useRef(connectionState);
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
        sessionId: '', // Session ID is managed by useSyncState
        isReconnect,
      });
    }
  }, [connectionState, enabled, deviceId]);

  // Emit error event for connection errors
  useEffect(() => {
    if (!enabled || !connectionError) return;

    // Check for auth-related errors
    const isAuthError =
      connectionError.toLowerCase().includes('auth') ||
      connectionError.toLowerCase().includes('unauthorized') ||
      connectionError.toLowerCase().includes('token') ||
      connectionError.toLowerCase().includes('401');

    syncEvents.emit('error', {
      message: connectionError,
      isAuthError,
    });
  }, [connectionError, enabled]);

  // Memoize context value
  const contextValue = useMemo<SyncContextValue>(() => ({
    enabled,
    ...syncState,
  }), [enabled, syncState]);

  return (
    <SyncContext.Provider value={contextValue}>
      {children}
    </SyncContext.Provider>
  );
}
