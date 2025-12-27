import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { Track } from '@resonance/shared-types'

export type { Track }

/**
 * Fisher-Yates shuffle algorithm for proper randomization
 * Returns a new shuffled array without modifying the original
 */
function fisherYatesShuffle<T>(array: T[]): T[] {
  const shuffled = [...array]
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    // Swap elements - both indices are guaranteed to be in bounds
    const temp = shuffled[i]!
    shuffled[i] = shuffled[j]!
    shuffled[j] = temp
  }
  return shuffled
}

/**
 * Select a random track index from the queue, excluding the current track
 * Uses Fisher-Yates shuffle for proper randomization
 */
function getShuffledNextIndex(queue: readonly { id: string }[], currentIndex: number): number {
  if (queue.length <= 1) return currentIndex

  // Create array of indices excluding current
  const availableIndices = queue
    .map((_, index) => index)
    .filter((index) => index !== currentIndex)

  if (availableIndices.length === 0) return currentIndex

  // Shuffle and pick the first one for true randomness
  const shuffled = fisherYatesShuffle(availableIndices)
  // Safe to assert: we already checked availableIndices has elements
  return shuffled[0]!
}

interface PlayerState {
  // Current playback
  currentTrack: Track | null
  isPlaying: boolean
  currentTime: number
  volume: number
  isMuted: boolean

  // Queue
  queue: Track[]
  queueIndex: number

  // Settings
  shuffle: boolean
  repeat: 'off' | 'track' | 'queue'

  // Actions
  setTrack: (track: Track) => void
  play: () => void
  pause: () => void
  togglePlay: () => void
  setCurrentTime: (time: number) => void
  setVolume: (volume: number) => void
  toggleMute: () => void
  setQueue: (tracks: Track[], startIndex?: number) => void
  addToQueue: (track: Track) => void
  nextTrack: () => void
  previousTrack: () => void
  toggleShuffle: () => void
  cycleRepeat: () => void
}

export const usePlayerStore = create<PlayerState>()(
  persist(
    (set, get) => ({
      // Initial state
      currentTrack: null,
      isPlaying: false,
      currentTime: 0,
      volume: 0.75,
      isMuted: false,
      queue: [],
      queueIndex: 0,
      shuffle: false,
      repeat: 'off',

      // Actions
      setTrack: (track) => set({ currentTrack: track, isPlaying: true, currentTime: 0 }),

      play: () => set({ isPlaying: true }),

      pause: () => set({ isPlaying: false }),

      togglePlay: () => set((state) => ({ isPlaying: !state.isPlaying })),

      setCurrentTime: (time) => set({ currentTime: time }),

      setVolume: (volume) => set({ volume, isMuted: false }),

      toggleMute: () => set((state) => ({ isMuted: !state.isMuted })),

      setQueue: (tracks, startIndex = 0) => set({
        queue: tracks,
        queueIndex: startIndex,
        currentTrack: tracks[startIndex] ?? null,
        isPlaying: true,
        currentTime: 0,
      }),

      addToQueue: (track) => set((state) => ({
        queue: [...state.queue, track],
      })),

      nextTrack: () => {
        const state = get()
        const { queue, queueIndex, repeat, shuffle } = state

        if (queue.length === 0) return

        let nextIndex: number

        if (shuffle) {
          // Use Fisher-Yates shuffle to select next track, excluding current track
          nextIndex = getShuffledNextIndex(queue, queueIndex)
        } else if (queueIndex < queue.length - 1) {
          nextIndex = queueIndex + 1
        } else if (repeat === 'queue') {
          nextIndex = 0
        } else {
          return // End of queue
        }

        set({
          queueIndex: nextIndex,
          currentTrack: queue[nextIndex] ?? null,
          currentTime: 0,
        })
      },

      previousTrack: () => {
        const state = get()
        const { queue, queueIndex, currentTime, repeat } = state

        if (queue.length === 0) return

        // If we're more than 3 seconds in, restart the track
        if (currentTime > 3) {
          set({ currentTime: 0 })
          return
        }

        let prevIndex: number

        if (queueIndex > 0) {
          prevIndex = queueIndex - 1
        } else if (repeat === 'queue') {
          prevIndex = queue.length - 1
        } else {
          set({ currentTime: 0 })
          return
        }

        set({
          queueIndex: prevIndex,
          currentTrack: queue[prevIndex] ?? null,
          currentTime: 0,
        })
      },

      toggleShuffle: () => set((state) => ({ shuffle: !state.shuffle })),

      cycleRepeat: () => set((state) => ({
        repeat: state.repeat === 'off' ? 'queue' : state.repeat === 'queue' ? 'track' : 'off',
      })),
    }),
    {
      name: 'resonance-player',
      partialize: (state) => ({
        volume: state.volume,
        isMuted: state.isMuted,
        shuffle: state.shuffle,
        repeat: state.repeat,
      }),
    }
  )
)
