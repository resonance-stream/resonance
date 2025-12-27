import { create } from 'zustand'
import { persist } from 'zustand/middleware'

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
          nextIndex = Math.floor(Math.random() * queue.length)
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
