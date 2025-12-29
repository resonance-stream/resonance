import { create } from 'zustand'
import { persist } from 'zustand/middleware'

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

export interface Track {
  id: string
  title: string
  artist: string
  albumId: string
  albumTitle: string
  duration: number
  coverUrl?: string
}

interface PlayerState {
  // Current playback
  currentTrack: Track | null
  isPlaying: boolean
  currentTime: number
  volume: number
  isMuted: boolean

  // Loading states
  isLoading: boolean
  isBuffering: boolean

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
  playNext: (track: Track) => void
  removeFromQueue: (index: number) => void
  reorderQueue: (fromIndex: number, toIndex: number) => void
  clearQueue: () => void
  jumpToIndex: (index: number) => void
  nextTrack: () => void
  previousTrack: () => void
  toggleShuffle: () => void
  cycleRepeat: () => void
  setLoading: (loading: boolean) => void
  setBuffering: (buffering: boolean) => void
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
      isLoading: false,
      isBuffering: false,
      queue: [],
      queueIndex: 0,
      shuffle: false,
      repeat: 'off',

      // Actions
      setTrack: (track) => set({ currentTrack: track, isPlaying: true, currentTime: 0, isLoading: true }),

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
        isLoading: true,
      }),

      addToQueue: (track) => set((state) => ({
        queue: [...state.queue, track],
      })),

      playNext: (track) => set((state) => {
        // Insert after current track position
        const newQueue = [...state.queue]
        newQueue.splice(state.queueIndex + 1, 0, track)
        return { queue: newQueue }
      }),

      removeFromQueue: (index) => set((state) => {
        if (index < 0 || index >= state.queue.length) return state

        const newQueue = state.queue.filter((_, i) => i !== index)

        // Adjust queueIndex if needed
        let newQueueIndex = state.queueIndex
        if (index < state.queueIndex) {
          // Removed track before current, shift index back
          newQueueIndex = state.queueIndex - 1
        } else if (index === state.queueIndex) {
          // Removed current track - play next or stop
          if (newQueue.length === 0) {
            return {
              queue: newQueue,
              queueIndex: 0,
              currentTrack: null,
              isPlaying: false,
            }
          }
          // Keep same index (next track slides in), or go to last if at end
          newQueueIndex = Math.min(state.queueIndex, newQueue.length - 1)
          return {
            queue: newQueue,
            queueIndex: newQueueIndex,
            currentTrack: newQueue[newQueueIndex] ?? null,
            isLoading: true,
          }
        }

        return { queue: newQueue, queueIndex: newQueueIndex }
      }),

      reorderQueue: (fromIndex, toIndex) => set((state) => {
        if (
          fromIndex < 0 ||
          fromIndex >= state.queue.length ||
          toIndex < 0 ||
          toIndex >= state.queue.length ||
          fromIndex === toIndex
        ) {
          return state
        }

        const newQueue = [...state.queue]
        const [movedTrack] = newQueue.splice(fromIndex, 1)
        if (!movedTrack) return state
        newQueue.splice(toIndex, 0, movedTrack)

        // Adjust queueIndex to follow the current track
        let newQueueIndex = state.queueIndex
        if (fromIndex === state.queueIndex) {
          // Moving current track
          newQueueIndex = toIndex
        } else if (fromIndex < state.queueIndex && toIndex >= state.queueIndex) {
          // Moving track from before to after current
          newQueueIndex = state.queueIndex - 1
        } else if (fromIndex > state.queueIndex && toIndex <= state.queueIndex) {
          // Moving track from after to before current
          newQueueIndex = state.queueIndex + 1
        }

        return { queue: newQueue, queueIndex: newQueueIndex }
      }),

      clearQueue: () => set({
        queue: [],
        queueIndex: 0,
        currentTrack: null,
        isPlaying: false,
        currentTime: 0,
      }),

      jumpToIndex: (index) => {
        const state = get()
        if (index < 0 || index >= state.queue.length) return

        set({
          queueIndex: index,
          currentTrack: state.queue[index] ?? null,
          currentTime: 0,
          isLoading: true,
          isPlaying: true,
        })
      },

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
          isLoading: true,
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
          isLoading: true,
        })
      },

      toggleShuffle: () => set((state) => ({ shuffle: !state.shuffle })),

      cycleRepeat: () => set((state) => ({
        repeat: state.repeat === 'off' ? 'queue' : state.repeat === 'queue' ? 'track' : 'off',
      })),

      setLoading: (loading) => set({ isLoading: loading }),

      setBuffering: (buffering) => set({ isBuffering: buffering }),
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
