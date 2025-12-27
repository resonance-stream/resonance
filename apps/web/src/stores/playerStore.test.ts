/**
 * Player Store Tests
 *
 * Tests for the Zustand player store.
 */

import { describe, it, expect, beforeEach } from 'vitest'
import { usePlayerStore, Track } from './playerStore'

// Helper to reset the store between tests
function resetStore(): void {
  usePlayerStore.setState({
    currentTrack: null,
    isPlaying: false,
    currentTime: 0,
    volume: 0.75,
    isMuted: false,
    queue: [],
    queueIndex: 0,
    shuffle: false,
    repeat: 'off',
  })
}

// Sample test tracks
const testTrack1: Track = {
  id: '1',
  title: 'Test Track 1',
  artist: 'Test Artist',
  albumId: 'album-1',
  albumTitle: 'Test Album',
  duration: 180,
  coverUrl: '/covers/1.jpg',
}

const testTrack2: Track = {
  id: '2',
  title: 'Test Track 2',
  artist: 'Test Artist',
  albumId: 'album-1',
  albumTitle: 'Test Album',
  duration: 240,
  coverUrl: '/covers/2.jpg',
}

const testTrack3: Track = {
  id: '3',
  title: 'Test Track 3',
  artist: 'Test Artist',
  albumId: 'album-1',
  albumTitle: 'Test Album',
  duration: 200,
}

describe('playerStore', () => {
  beforeEach(() => {
    resetStore()
  })

  describe('initial state', () => {
    it('has null current track', () => {
      expect(usePlayerStore.getState().currentTrack).toBeNull()
    })

    it('is not playing', () => {
      expect(usePlayerStore.getState().isPlaying).toBe(false)
    })

    it('has default volume of 0.75', () => {
      expect(usePlayerStore.getState().volume).toBe(0.75)
    })

    it('is not muted', () => {
      expect(usePlayerStore.getState().isMuted).toBe(false)
    })

    it('has empty queue', () => {
      expect(usePlayerStore.getState().queue).toEqual([])
    })

    it('has shuffle off', () => {
      expect(usePlayerStore.getState().shuffle).toBe(false)
    })

    it('has repeat off', () => {
      expect(usePlayerStore.getState().repeat).toBe('off')
    })
  })

  describe('setTrack', () => {
    it('sets the current track', () => {
      usePlayerStore.getState().setTrack(testTrack1)

      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack1)
    })

    it('starts playing', () => {
      usePlayerStore.getState().setTrack(testTrack1)

      expect(usePlayerStore.getState().isPlaying).toBe(true)
    })

    it('resets current time to 0', () => {
      usePlayerStore.setState({ currentTime: 50 })
      usePlayerStore.getState().setTrack(testTrack1)

      expect(usePlayerStore.getState().currentTime).toBe(0)
    })
  })

  describe('play/pause', () => {
    it('play sets isPlaying to true', () => {
      usePlayerStore.getState().play()

      expect(usePlayerStore.getState().isPlaying).toBe(true)
    })

    it('pause sets isPlaying to false', () => {
      usePlayerStore.setState({ isPlaying: true })
      usePlayerStore.getState().pause()

      expect(usePlayerStore.getState().isPlaying).toBe(false)
    })

    it('togglePlay toggles isPlaying', () => {
      expect(usePlayerStore.getState().isPlaying).toBe(false)

      usePlayerStore.getState().togglePlay()
      expect(usePlayerStore.getState().isPlaying).toBe(true)

      usePlayerStore.getState().togglePlay()
      expect(usePlayerStore.getState().isPlaying).toBe(false)
    })
  })

  describe('volume', () => {
    it('setVolume updates volume', () => {
      usePlayerStore.getState().setVolume(0.5)

      expect(usePlayerStore.getState().volume).toBe(0.5)
    })

    it('setVolume unmutes', () => {
      usePlayerStore.setState({ isMuted: true })
      usePlayerStore.getState().setVolume(0.5)

      expect(usePlayerStore.getState().isMuted).toBe(false)
    })

    it('toggleMute toggles muted state', () => {
      expect(usePlayerStore.getState().isMuted).toBe(false)

      usePlayerStore.getState().toggleMute()
      expect(usePlayerStore.getState().isMuted).toBe(true)

      usePlayerStore.getState().toggleMute()
      expect(usePlayerStore.getState().isMuted).toBe(false)
    })
  })

  describe('queue management', () => {
    it('setQueue sets the queue and starts playing', () => {
      const tracks = [testTrack1, testTrack2, testTrack3]
      usePlayerStore.getState().setQueue(tracks)

      expect(usePlayerStore.getState().queue).toEqual(tracks)
      expect(usePlayerStore.getState().queueIndex).toBe(0)
      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack1)
      expect(usePlayerStore.getState().isPlaying).toBe(true)
    })

    it('setQueue with startIndex sets correct track', () => {
      const tracks = [testTrack1, testTrack2, testTrack3]
      usePlayerStore.getState().setQueue(tracks, 1)

      expect(usePlayerStore.getState().queueIndex).toBe(1)
      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack2)
    })

    it('addToQueue appends track to queue', () => {
      usePlayerStore.setState({ queue: [testTrack1] })
      usePlayerStore.getState().addToQueue(testTrack2)

      expect(usePlayerStore.getState().queue).toEqual([testTrack1, testTrack2])
    })
  })

  describe('nextTrack', () => {
    it('advances to next track', () => {
      const tracks = [testTrack1, testTrack2, testTrack3]
      usePlayerStore.getState().setQueue(tracks)
      usePlayerStore.getState().nextTrack()

      expect(usePlayerStore.getState().queueIndex).toBe(1)
      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack2)
    })

    it('does nothing at end of queue when repeat is off', () => {
      const tracks = [testTrack1, testTrack2]
      usePlayerStore.getState().setQueue(tracks, 1) // Start at last track

      usePlayerStore.getState().nextTrack()

      expect(usePlayerStore.getState().queueIndex).toBe(1)
      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack2)
    })

    it('loops to start when repeat is queue', () => {
      const tracks = [testTrack1, testTrack2]
      usePlayerStore.getState().setQueue(tracks, 1)
      usePlayerStore.setState({ repeat: 'queue' })

      usePlayerStore.getState().nextTrack()

      expect(usePlayerStore.getState().queueIndex).toBe(0)
      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack1)
    })

    it('does nothing on empty queue', () => {
      usePlayerStore.getState().nextTrack()

      expect(usePlayerStore.getState().queueIndex).toBe(0)
      expect(usePlayerStore.getState().currentTrack).toBeNull()
    })
  })

  describe('previousTrack', () => {
    it('goes to previous track when within 3 seconds', () => {
      const tracks = [testTrack1, testTrack2, testTrack3]
      usePlayerStore.getState().setQueue(tracks, 1)
      usePlayerStore.setState({ currentTime: 2 })

      usePlayerStore.getState().previousTrack()

      expect(usePlayerStore.getState().queueIndex).toBe(0)
      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack1)
    })

    it('restarts current track when past 3 seconds', () => {
      const tracks = [testTrack1, testTrack2, testTrack3]
      usePlayerStore.getState().setQueue(tracks, 1)
      usePlayerStore.setState({ currentTime: 5 })

      usePlayerStore.getState().previousTrack()

      expect(usePlayerStore.getState().queueIndex).toBe(1)
      expect(usePlayerStore.getState().currentTime).toBe(0)
    })

    it('loops to end when repeat is queue and at start', () => {
      const tracks = [testTrack1, testTrack2]
      usePlayerStore.getState().setQueue(tracks, 0)
      usePlayerStore.setState({ currentTime: 2, repeat: 'queue' })

      usePlayerStore.getState().previousTrack()

      expect(usePlayerStore.getState().queueIndex).toBe(1)
      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack2)
    })
  })

  describe('shuffle', () => {
    it('toggleShuffle toggles shuffle state', () => {
      expect(usePlayerStore.getState().shuffle).toBe(false)

      usePlayerStore.getState().toggleShuffle()
      expect(usePlayerStore.getState().shuffle).toBe(true)

      usePlayerStore.getState().toggleShuffle()
      expect(usePlayerStore.getState().shuffle).toBe(false)
    })
  })

  describe('repeat', () => {
    it('cycleRepeat cycles through repeat modes', () => {
      expect(usePlayerStore.getState().repeat).toBe('off')

      usePlayerStore.getState().cycleRepeat()
      expect(usePlayerStore.getState().repeat).toBe('queue')

      usePlayerStore.getState().cycleRepeat()
      expect(usePlayerStore.getState().repeat).toBe('track')

      usePlayerStore.getState().cycleRepeat()
      expect(usePlayerStore.getState().repeat).toBe('off')
    })
  })
})
