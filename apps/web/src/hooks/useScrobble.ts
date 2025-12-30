/**
 * Scrobble tracking hook for ListenBrainz integration
 *
 * Monitors playback progress and submits scrobbles to ListenBrainz when:
 * - Track has played for 50% of duration OR 4 minutes (whichever comes first)
 * - Track is at least 30 seconds long
 * - ListenBrainz scrobbling is enabled
 *
 * Prevents duplicate scrobbles for the same track play session.
 */

import { useEffect, useRef, useCallback, useState } from 'react'
import { usePlayerStore } from '../stores/playerStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useSubmitScrobble } from './useIntegrations'
import type { ScrobbleInput } from '../types/integrations'

// Scrobble thresholds (following Last.fm/ListenBrainz standard rules)
const SCROBBLE_PERCENTAGE = 0.5 // 50% of track duration
const SCROBBLE_MAX_SECONDS = 240 // 4 minutes
const MIN_TRACK_DURATION_MS = 30000 // 30 seconds minimum

/**
 * Calculate the scrobble threshold for a track in milliseconds
 * Returns the earlier of: 50% of duration or 4 minutes
 */
function calculateScrobbleThreshold(durationMs: number): number {
  const halfDuration = durationMs * SCROBBLE_PERCENTAGE
  const maxThreshold = SCROBBLE_MAX_SECONDS * 1000
  return Math.min(halfDuration, maxThreshold)
}

/**
 * Generate a unique key for a playback session
 * Combines track ID with the timestamp when playback started
 */
function getPlaybackSessionKey(trackId: string, startTime: Date): string {
  return `${trackId}-${startTime.getTime()}`
}

export interface UseScrobbleOptions {
  /** Enable debug logging */
  debug?: boolean
}

export interface UseScrobbleReturn {
  /** Whether a scrobble was submitted for the current track */
  hasScrobbled: boolean
  /** Whether a scrobble submission is in progress */
  isSubmitting: boolean
  /** Manually trigger a scrobble (for testing) */
  forceScrobble: () => void
}

/**
 * Hook to track playback and submit scrobbles to ListenBrainz
 *
 * @param options - Configuration options
 * @returns Scrobble state and control functions
 */
export function useScrobble(options: UseScrobbleOptions = {}): UseScrobbleReturn {
  const { debug = false } = options

  // Track state from player store
  const currentTrack = usePlayerStore((s) => s.currentTrack)
  const isPlaying = usePlayerStore((s) => s.isPlaying)
  const currentTime = usePlayerStore((s) => s.currentTime)

  // Integration settings
  const listenbrainzEnabled = useSettingsStore((s) => s.integrations.listenbrainzEnabled)

  // Use mutation hook for scrobble submission
  const submitScrobbleMutation = useSubmitScrobble({
    onSuccess: (result) => {
      if (result.success) {
        if (debug) {
          console.log('[useScrobble] Scrobble submitted successfully')
        }
      } else {
        console.warn('[useScrobble] Scrobble failed:', result.error)
        // Allow retry by resetting scrobbled state
        setHasScrobbled(false)
        if (playbackStartRef.current && currentTrack) {
          const sessionKey = getPlaybackSessionKey(currentTrack.id, playbackStartRef.current)
          scrobbledSessionsRef.current.delete(sessionKey)
        }
      }
    },
    onError: (error) => {
      console.error('[useScrobble] Failed to submit scrobble:', error)
      // Allow retry by resetting scrobbled state
      setHasScrobbled(false)
      if (playbackStartRef.current && currentTrack) {
        const sessionKey = getPlaybackSessionKey(currentTrack.id, playbackStartRef.current)
        scrobbledSessionsRef.current.delete(sessionKey)
      }
    },
  })

  // Reactive state for UI updates
  const [hasScrobbled, setHasScrobbled] = useState(false)

  // Refs to track scrobble state across renders without triggering re-renders
  const scrobbledSessionsRef = useRef<Set<string>>(new Set())
  const playbackStartRef = useRef<Date | null>(null)
  const accumulatedTimeRef = useRef<number>(0)
  const lastUpdateTimeRef = useRef<number>(0)

  // Reset tracking when track changes
  // We intentionally only depend on currentTrack?.id to avoid resetting on other property changes
  const trackId = currentTrack?.id
  useEffect(() => {
    if (trackId) {
      // New track started - reset all tracking state
      playbackStartRef.current = new Date()
      accumulatedTimeRef.current = 0
      lastUpdateTimeRef.current = 0 // Start fresh from position 0
      setHasScrobbled(false)

      if (debug) {
        console.log('[useScrobble] New track started:', trackId)
      }
    } else {
      // Track cleared
      playbackStartRef.current = null
      accumulatedTimeRef.current = 0
      setHasScrobbled(false)
    }
  }, [trackId, debug])

  // Submit scrobble function
  const submitScrobble = useCallback(() => {
    if (!currentTrack || !playbackStartRef.current) {
      return
    }

    if (!listenbrainzEnabled) {
      if (debug) {
        console.log('[useScrobble] Scrobbling disabled, skipping')
      }
      return
    }

    const sessionKey = getPlaybackSessionKey(currentTrack.id, playbackStartRef.current)

    // Check if already scrobbled this session
    if (scrobbledSessionsRef.current.has(sessionKey)) {
      if (debug) {
        console.log('[useScrobble] Already scrobbled this session')
      }
      return
    }

    // Mark as scrobbled immediately to prevent race conditions
    scrobbledSessionsRef.current.add(sessionKey)
    setHasScrobbled(true)

    const input: ScrobbleInput = {
      trackId: currentTrack.id,
      playedAt: playbackStartRef.current.toISOString(),
      durationPlayed: Math.floor(accumulatedTimeRef.current),
    }

    if (debug) {
      console.log('[useScrobble] Submitting scrobble:', input)
    }

    // Submit via mutation hook
    submitScrobbleMutation.mutate(input)
  }, [currentTrack, listenbrainzEnabled, submitScrobbleMutation, debug])

  // Track accumulated playback time and check threshold
  useEffect(() => {
    if (!currentTrack || !isPlaying || !listenbrainzEnabled) {
      return
    }

    // Skip tracks shorter than minimum duration
    if (currentTrack.duration < MIN_TRACK_DURATION_MS) {
      if (debug) {
        console.log('[useScrobble] Track too short for scrobbling:', currentTrack.duration, 'ms')
      }
      return
    }

    // Skip if already scrobbled this session
    if (hasScrobbled) {
      return
    }

    // Calculate time delta since last update
    const timeDelta = currentTime - lastUpdateTimeRef.current
    lastUpdateTimeRef.current = currentTime

    // Only accumulate positive deltas (ignore seeks backwards)
    // Cap at 5 seconds to ignore large jumps (seeks forward)
    if (timeDelta > 0 && timeDelta < 5) {
      accumulatedTimeRef.current += timeDelta
    }

    // Check if we've reached the scrobble threshold
    const thresholdMs = calculateScrobbleThreshold(currentTrack.duration)
    const accumulatedMs = accumulatedTimeRef.current * 1000

    if (accumulatedMs >= thresholdMs) {
      if (debug) {
        console.log('[useScrobble] Threshold reached:', {
          accumulated: accumulatedMs,
          threshold: thresholdMs,
          trackDuration: currentTrack.duration,
        })
      }
      submitScrobble()
    }
  }, [currentTrack, isPlaying, currentTime, listenbrainzEnabled, hasScrobbled, submitScrobble, debug])

  // Cleanup old sessions periodically (keep last 100)
  useEffect(() => {
    const sessions = scrobbledSessionsRef.current
    if (sessions.size > 100) {
      const sessionsArray = Array.from(sessions)
      const toRemove = sessionsArray.slice(0, sessions.size - 100)
      toRemove.forEach((key) => sessions.delete(key))
    }
  }, [currentTrack?.id])

  return {
    hasScrobbled,
    isSubmitting: submitScrobbleMutation.isPending,
    forceScrobble: submitScrobble,
  }
}
