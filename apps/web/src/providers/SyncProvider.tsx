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
 */

import { createContext, useContext, useCallback, useMemo, type ReactNode } from 'react';
import { useSyncState, type SyncStateValue } from '../sync/useSyncState';
import { usePlayerStore } from '../stores/playerStore';

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
  const handleRemoteTrackChange = useCallback((trackId: string) => {
    // Get current queue at execution time to avoid stale closures
    const currentQueue = usePlayerStore.getState().queue;
    const trackIndex = currentQueue.findIndex((t) => t.id === trackId);
    if (trackIndex !== -1) {
      // Track is in queue, jump to it
      jumpToIndex(trackIndex);
    } else {
      // Track not in queue - would need to fetch and load
      // For now, log a warning. Full implementation would:
      // 1. Fetch track metadata from API
      // 2. Add to queue or replace queue
      // 3. Start playback
      console.warn('[SyncProvider] Remote track not in queue:', trackId);
    }
  }, [jumpToIndex]);

  // Initialize sync state
  const syncState = useSyncState({
    onRemoteTrackChange: handleRemoteTrackChange,
    autoConnect: enabled,
  });

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
