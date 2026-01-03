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
 * - Shows toast notifications for connection events
 */

import { createContext, useContext, useCallback, useMemo, useEffect, useRef, type ReactNode } from 'react';
import { useSyncState, type SyncStateValue } from '../sync/useSyncState';
import { usePlayerStore } from '../stores/playerStore';
import { useDeviceStore } from '../stores/deviceStore';
import { useToastStore } from '../stores/toastStore';
import { fetchTrackById } from '../sync/fetchTrackById';

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
  const addToast = useToastStore((s) => s.addToast);

  // Track previous connection state for toast notifications
  const prevConnectionStateRef = useRef(connectionState);
  const hasShownDisconnectToastRef = useRef(false);

  // Show toast notifications for connection state changes
  useEffect(() => {
    // Skip if sync is disabled
    if (!enabled) return;

    const prevState = prevConnectionStateRef.current;
    prevConnectionStateRef.current = connectionState;

    // Show warning when disconnecting or reconnecting
    if (
      (connectionState === 'disconnected' || connectionState === 'reconnecting') &&
      prevState === 'connected' &&
      !hasShownDisconnectToastRef.current
    ) {
      hasShownDisconnectToastRef.current = true;
      addToast({
        type: 'warning',
        title: 'Sync disconnected',
        description: 'Reconnecting...',
      });
    }

    // Show success when reconnected
    if (connectionState === 'connected' && prevState !== 'connected') {
      // Only show "restored" if we previously disconnected (not initial connect)
      if (hasShownDisconnectToastRef.current) {
        addToast({
          type: 'success',
          title: 'Sync restored',
          description: 'Cross-device sync is active',
        });
      }
      hasShownDisconnectToastRef.current = false;
    }
  }, [connectionState, enabled, addToast]);

  // Show error toast for auth failures and other errors
  useEffect(() => {
    if (!enabled || !connectionError) return;

    // Check for auth-related errors
    const isAuthError =
      connectionError.toLowerCase().includes('auth') ||
      connectionError.toLowerCase().includes('unauthorized') ||
      connectionError.toLowerCase().includes('token') ||
      connectionError.toLowerCase().includes('401');

    if (isAuthError) {
      addToast({
        type: 'error',
        title: 'Sync authentication failed',
        description: 'Please sign in again to enable sync',
      });
    } else {
      // Generic connection error
      addToast({
        type: 'error',
        title: 'Sync connection error',
        description: connectionError,
      });
    }
  }, [connectionError, enabled, addToast]);

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
