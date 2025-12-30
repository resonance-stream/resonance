/**
 * Discord Rich Presence hook
 *
 * Monitors playback state and updates Discord presence accordingly.
 * Only functional in desktop app environments (Electron/Tauri).
 *
 * Updates presence on:
 * - Track change
 * - Play/pause state change
 * - Setting toggle (enable/disable)
 */

import { useEffect, useRef, useCallback, useMemo } from 'react'
import { usePlayerStore } from '../stores/playerStore'
import { useIntegrations } from './useIntegrations'
import {
  getDiscordRpcService,
  type PresenceState,
  type RpcAvailability,
} from '../services/discordRpc'

export interface UseDiscordRpcOptions {
  /** Enable debug logging */
  debug?: boolean
}

export interface UseDiscordRpcReturn {
  /** Whether Discord RPC is available in this environment */
  availability: RpcAvailability
  /** Whether Discord RPC is currently enabled */
  isEnabled: boolean
  /**
   * Manually trigger a presence update, bypassing debounce.
   * Primarily intended for testing or debugging.
   */
  forceUpdate: () => void
}

/**
 * Hook to manage Discord Rich Presence based on playback state
 *
 * @param options - Configuration options
 * @returns Discord RPC state and control functions
 */
export function useDiscordRpc(options: UseDiscordRpcOptions = {}): UseDiscordRpcReturn {
  const { debug = false } = options

  // Get playback state (excluding currentTime to avoid excessive re-renders)
  const currentTrack = usePlayerStore((s) => s.currentTrack)
  const isPlaying = usePlayerStore((s) => s.isPlaying)

  // Get integration settings from server (TanStack Query as single source of truth)
  const { data: integrations } = useIntegrations()
  const discordRpcEnabled = integrations?.discordRpcEnabled ?? false

  // Memoize service and availability to prevent re-creation on every render
  const service = useMemo(() => getDiscordRpcService(), [])
  const availability = useMemo(() => service.checkAvailability(), [service])

  // Keep track of last update to debounce
  const lastUpdateRef = useRef<number>(0)
  const updateTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Refs for cleanup to avoid stale closures
  const discordRpcEnabledRef = useRef(discordRpcEnabled)
  const availabilityRef = useRef(availability)

  // Keep refs in sync
  useEffect(() => {
    discordRpcEnabledRef.current = discordRpcEnabled
    availabilityRef.current = availability
  }, [discordRpcEnabled, availability])

  // Build current presence state (reads currentTime at call time)
  const buildPresenceState = useCallback((): PresenceState => {
    if (!currentTrack) {
      return { track: null, isPlaying: false }
    }

    // Get currentTime at call time to avoid subscription overhead
    const currentTime = usePlayerStore.getState().currentTime

    return {
      track: {
        title: currentTrack.title,
        artist: currentTrack.artist,
        album: currentTrack.albumTitle,
        duration: currentTrack.duration / 1000, // Convert ms to seconds
        coverUrl: currentTrack.coverUrl || undefined,
      },
      isPlaying,
      position: currentTime,
    }
  }, [currentTrack, isPlaying])

  // Update presence with debouncing
  const updatePresence = useCallback(() => {
    if (!discordRpcEnabled || !availability.available) {
      return
    }

    const now = Date.now()
    const timeSinceLastUpdate = now - lastUpdateRef.current

    // Debounce rapid updates (min 1 second between updates)
    if (timeSinceLastUpdate < 1000) {
      if (updateTimeoutRef.current) {
        clearTimeout(updateTimeoutRef.current)
      }
      // Build state now to capture current values
      const stateToSend = buildPresenceState()
      updateTimeoutRef.current = setTimeout(() => {
        // Check current enabled state via refs to avoid stale closures
        if (!discordRpcEnabledRef.current || !availabilityRef.current.available) return
        if (!service.isEnabled()) return
        lastUpdateRef.current = Date.now()
        if (debug) {
          console.log('[useDiscordRpc] Updating presence (debounced):', stateToSend)
        }
        service.setPresence(stateToSend).catch((error) => {
          console.error('[useDiscordRpc] Failed to update presence:', error)
        })
      }, 1000 - timeSinceLastUpdate)
      return
    }

    lastUpdateRef.current = now
    const state = buildPresenceState()

    if (debug) {
      console.log('[useDiscordRpc] Updating presence:', state)
    }

    service.setPresence(state).catch((error) => {
      console.error('[useDiscordRpc] Failed to update presence:', error)
    })
  }, [discordRpcEnabled, availability.available, buildPresenceState, service, debug])

  // Force update (for testing)
  const forceUpdate = useCallback(() => {
    lastUpdateRef.current = 0
    updatePresence()
  }, [updatePresence])

  // Sync enabled state with service
  useEffect(() => {
    service.setEnabled(discordRpcEnabled)

    if (debug) {
      console.log('[useDiscordRpc] Enabled:', discordRpcEnabled)
    }
  }, [discordRpcEnabled, service, debug])

  // Consolidated effect for track and play state changes
  const trackId = currentTrack?.id
  useEffect(() => {
    if (!discordRpcEnabled || !availability.available) {
      return
    }

    if (trackId) {
      updatePresence()
    } else {
      // Clear presence when no track
      service.clearPresence().catch((error) => {
        console.error('[useDiscordRpc] Failed to clear presence:', error)
      })
    }
  }, [trackId, isPlaying, discordRpcEnabled, availability.available, updatePresence, service])

  // Cleanup on unmount - use refs to avoid stale closures
  useEffect(() => {
    return () => {
      if (updateTimeoutRef.current) {
        clearTimeout(updateTimeoutRef.current)
      }
      if (discordRpcEnabledRef.current && availabilityRef.current.available) {
        service.clearPresence().catch(() => {
          // Ignore errors during cleanup
        })
      }
    }
  }, [service])

  return {
    availability,
    isEnabled: discordRpcEnabled,
    forceUpdate,
  }
}
