/**
 * NowPlaying Component Tests
 *
 * Tests for the now playing component that displays current track info.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, userEvent, waitFor, act } from '@/test/test-utils'
import { NowPlaying } from './NowPlaying'
import { usePlayerStore, Track } from '../../stores/playerStore'
import { useVisualizerStore } from '../../stores/visualizerStore'

// Mock AlbumArt component
vi.mock('../media/AlbumArt', () => ({
  AlbumArt: ({ alt }: { alt: string }) => <div data-testid="album-art">{alt}</div>,
}))

// Sample test tracks
const testTrack1: Track = {
  id: '1',
  title: 'Test Track 1',
  artist: 'Test Artist 1',
  albumId: 'album-1',
  albumTitle: 'Test Album 1',
  duration: 180,
  coverUrl: '/covers/1.jpg',
}

const testTrack2: Track = {
  id: '2',
  title: 'Test Track 2',
  artist: 'Test Artist 2',
  albumId: 'album-2',
  albumTitle: 'Test Album 2',
  duration: 240,
  coverUrl: '/covers/2.jpg',
}

// Helper to reset stores between tests
function resetStores(): void {
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
  useVisualizerStore.setState({
    showVisualizer: false,
    isFullscreen: false,
  })
}

describe('NowPlaying', () => {
  beforeEach(() => {
    resetStores()
  })

  describe('rendering', () => {
    it('renders nothing when no track is loaded', () => {
      const { container } = render(<NowPlaying />)

      expect(container.firstChild).toBeNull()
    })

    it('renders track info when a track is loaded', () => {
      usePlayerStore.setState({ currentTrack: testTrack1 })

      render(<NowPlaying />)

      expect(screen.getByText('Test Track 1')).toBeInTheDocument()
      expect(screen.getByText('Test Artist 1')).toBeInTheDocument()
    })

    it('renders album art', () => {
      usePlayerStore.setState({ currentTrack: testTrack1 })

      render(<NowPlaying />)

      expect(screen.getByTestId('album-art')).toBeInTheDocument()
      expect(screen.getByTestId('album-art')).toHaveTextContent('Test Album 1 album art')
    })
  })

  describe('fullscreen interaction', () => {
    it('opens fullscreen when clicked', async () => {
      const user = userEvent.setup()
      usePlayerStore.setState({ currentTrack: testTrack1 })

      render(<NowPlaying />)

      await user.click(screen.getByRole('button'))
      expect(useVisualizerStore.getState().isFullscreen).toBe(true)
    })

    it('has accessible label describing the action', () => {
      usePlayerStore.setState({ currentTrack: testTrack1 })

      render(<NowPlaying />)

      const button = screen.getByRole('button')
      expect(button).toHaveAttribute(
        'aria-label',
        'Now playing: Test Track 1 by Test Artist 1. Click to open fullscreen view.'
      )
    })
  })

  describe('accessibility announcements', () => {
    it('announces track changes to screen readers', async () => {
      usePlayerStore.setState({ currentTrack: testTrack1 })

      render(<NowPlaying />)

      await waitFor(() => {
        expect(screen.getByRole('status')).toHaveTextContent(
          'Now playing: Test Track 1 by Test Artist 1'
        )
      })
    })

    it('clears announcement after timeout', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true })
      usePlayerStore.setState({ currentTrack: testTrack1 })

      render(<NowPlaying />)

      // Announcement should be present initially
      await waitFor(() => {
        expect(screen.getByRole('status')).toBeInTheDocument()
      })

      // Fast-forward past the 3-second timeout
      await act(async () => {
        await vi.advanceTimersByTimeAsync(3500)
      })

      await waitFor(() => {
        expect(screen.queryByRole('status')).not.toBeInTheDocument()
      })

      vi.useRealTimers()
    })

    it('announces when track changes', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true })
      usePlayerStore.setState({ currentTrack: testTrack1 })

      const { rerender } = render(<NowPlaying />)

      // Wait for initial announcement
      await waitFor(() => {
        expect(screen.getByRole('status')).toHaveTextContent('Now playing: Test Track 1')
      })

      // Clear the first announcement
      await act(async () => {
        await vi.advanceTimersByTimeAsync(3500)
      })

      // Change track
      await act(async () => {
        usePlayerStore.setState({ currentTrack: testTrack2 })
      })
      rerender(<NowPlaying />)

      await waitFor(() => {
        expect(screen.getByRole('status')).toHaveTextContent(
          'Now playing: Test Track 2 by Test Artist 2'
        )
      })

      vi.useRealTimers()
    })
  })
})
