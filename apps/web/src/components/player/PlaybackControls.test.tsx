/**
 * PlaybackControls Component Tests
 *
 * Tests for the playback controls (play/pause, next, previous, shuffle, repeat).
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, userEvent } from '@/test/test-utils'
import { PlaybackControls } from './PlaybackControls'
import { usePlayerStore, Track } from '../../stores/playerStore'

// Mock ProgressBar to isolate PlaybackControls testing
vi.mock('./ProgressBar', () => ({
  ProgressBar: () => <div data-testid="progress-bar">Progress Bar Mock</div>,
}))

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

describe('PlaybackControls', () => {
  beforeEach(() => {
    resetStore()
  })

  describe('rendering', () => {
    it('renders all control buttons', () => {
      render(<PlaybackControls />)

      expect(screen.getByRole('button', { name: /play/i })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /previous track/i })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /next track/i })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /shuffle/i })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /repeat/i })).toBeInTheDocument()
    })

    it('renders the progress bar', () => {
      render(<PlaybackControls />)

      expect(screen.getByTestId('progress-bar')).toBeInTheDocument()
    })
  })

  describe('play/pause button', () => {
    it('shows play button when paused', () => {
      usePlayerStore.setState({ isPlaying: false })

      render(<PlaybackControls />)

      expect(screen.getByRole('button', { name: 'Play' })).toBeInTheDocument()
    })

    it('shows pause button when playing', () => {
      usePlayerStore.setState({ isPlaying: true })

      render(<PlaybackControls />)

      expect(screen.getByRole('button', { name: 'Pause' })).toBeInTheDocument()
    })

    it('toggles play state when clicked', async () => {
      const user = userEvent.setup()
      usePlayerStore.setState({ isPlaying: false })

      render(<PlaybackControls />)

      await user.click(screen.getByRole('button', { name: 'Play' }))
      expect(usePlayerStore.getState().isPlaying).toBe(true)

      await user.click(screen.getByRole('button', { name: 'Pause' }))
      expect(usePlayerStore.getState().isPlaying).toBe(false)
    })
  })

  describe('previous/next buttons', () => {
    it('disables previous/next when queue is empty', () => {
      usePlayerStore.setState({ queue: [] })

      render(<PlaybackControls />)

      expect(screen.getByRole('button', { name: /previous track/i })).toBeDisabled()
      expect(screen.getByRole('button', { name: /next track/i })).toBeDisabled()
    })

    it('enables previous/next when queue has tracks', () => {
      usePlayerStore.setState({
        queue: [testTrack1, testTrack2],
        queueIndex: 0,
        currentTrack: testTrack1,
      })

      render(<PlaybackControls />)

      expect(screen.getByRole('button', { name: /previous track/i })).not.toBeDisabled()
      expect(screen.getByRole('button', { name: /next track/i })).not.toBeDisabled()
    })

    it('calls nextTrack when next button is clicked', async () => {
      const user = userEvent.setup()
      usePlayerStore.setState({
        queue: [testTrack1, testTrack2],
        queueIndex: 0,
        currentTrack: testTrack1,
      })

      render(<PlaybackControls />)

      await user.click(screen.getByRole('button', { name: /next track/i }))
      expect(usePlayerStore.getState().queueIndex).toBe(1)
      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack2)
    })

    it('calls previousTrack when previous button is clicked', async () => {
      const user = userEvent.setup()
      usePlayerStore.setState({
        queue: [testTrack1, testTrack2],
        queueIndex: 1,
        currentTrack: testTrack2,
        currentTime: 1, // Less than 3 seconds
      })

      render(<PlaybackControls />)

      await user.click(screen.getByRole('button', { name: /previous track/i }))
      expect(usePlayerStore.getState().queueIndex).toBe(0)
      expect(usePlayerStore.getState().currentTrack).toEqual(testTrack1)
    })
  })

  describe('shuffle button', () => {
    it('has aria-pressed false when shuffle is off', () => {
      usePlayerStore.setState({ shuffle: false })

      render(<PlaybackControls />)

      const shuffleButton = screen.getByRole('button', { name: /enable shuffle/i })
      expect(shuffleButton).toHaveAttribute('aria-pressed', 'false')
    })

    it('has aria-pressed true when shuffle is on', () => {
      usePlayerStore.setState({ shuffle: true })

      render(<PlaybackControls />)

      const shuffleButton = screen.getByRole('button', { name: /disable shuffle/i })
      expect(shuffleButton).toHaveAttribute('aria-pressed', 'true')
    })

    it('toggles shuffle when clicked', async () => {
      const user = userEvent.setup()
      usePlayerStore.setState({ shuffle: false })

      render(<PlaybackControls />)

      await user.click(screen.getByRole('button', { name: /enable shuffle/i }))
      expect(usePlayerStore.getState().shuffle).toBe(true)

      await user.click(screen.getByRole('button', { name: /disable shuffle/i }))
      expect(usePlayerStore.getState().shuffle).toBe(false)
    })
  })

  describe('repeat button', () => {
    it('shows correct label for repeat off', () => {
      usePlayerStore.setState({ repeat: 'off' })

      render(<PlaybackControls />)

      expect(screen.getByRole('button', { name: /enable repeat all/i })).toBeInTheDocument()
    })

    it('shows correct label for repeat queue', () => {
      usePlayerStore.setState({ repeat: 'queue' })

      render(<PlaybackControls />)

      expect(screen.getByRole('button', { name: /enable repeat one/i })).toBeInTheDocument()
    })

    it('shows correct label for repeat track', () => {
      usePlayerStore.setState({ repeat: 'track' })

      render(<PlaybackControls />)

      expect(screen.getByRole('button', { name: /disable repeat/i })).toBeInTheDocument()
    })

    it('cycles through repeat modes when clicked', async () => {
      const user = userEvent.setup()
      usePlayerStore.setState({ repeat: 'off' })

      render(<PlaybackControls />)

      // off -> queue
      await user.click(screen.getByRole('button', { name: /enable repeat all/i }))
      expect(usePlayerStore.getState().repeat).toBe('queue')

      // queue -> track
      await user.click(screen.getByRole('button', { name: /enable repeat one/i }))
      expect(usePlayerStore.getState().repeat).toBe('track')

      // track -> off
      await user.click(screen.getByRole('button', { name: /disable repeat/i }))
      expect(usePlayerStore.getState().repeat).toBe('off')
    })
  })
})
