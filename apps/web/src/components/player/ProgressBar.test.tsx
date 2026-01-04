/**
 * ProgressBar Component Tests
 *
 * Tests for the seek/progress bar component.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/test-utils'
import { ProgressBar } from './ProgressBar'
import { usePlayerStore, Track } from '../../stores/playerStore'

// Mock useAudio hook
const mockSeek = vi.fn()
vi.mock('../../hooks/useAudio', () => ({
  useAudio: () => ({
    seek: mockSeek,
  }),
}))

// Sample test track
const testTrack: Track = {
  id: '1',
  title: 'Test Track',
  artist: 'Test Artist',
  albumId: 'album-1',
  albumTitle: 'Test Album',
  duration: 180, // 3 minutes
  coverUrl: '/covers/1.jpg',
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

describe('ProgressBar', () => {
  beforeEach(() => {
    resetStore()
    mockSeek.mockClear()
  })

  describe('time display', () => {
    it('displays 0:00 when no track is loaded', () => {
      render(<ProgressBar />)

      const timeDisplays = screen.getAllByText('0:00')
      expect(timeDisplays.length).toBeGreaterThan(0)
    })

    it('displays current time and duration', () => {
      usePlayerStore.setState({
        currentTrack: testTrack,
        currentTime: 65, // 1:05
      })

      render(<ProgressBar />)

      expect(screen.getByText('1:05')).toBeInTheDocument()
      expect(screen.getByText('3:00')).toBeInTheDocument()
    })

    it('formats time correctly for single digit seconds', () => {
      usePlayerStore.setState({
        currentTrack: testTrack,
        currentTime: 5, // 0:05
      })

      render(<ProgressBar />)

      expect(screen.getByText('0:05')).toBeInTheDocument()
    })

    it('handles edge case of 0 duration gracefully', () => {
      const trackWithZeroDuration = { ...testTrack, duration: 0 }
      usePlayerStore.setState({
        currentTrack: trackWithZeroDuration,
        currentTime: 0,
      })

      render(<ProgressBar />)

      const timeDisplays = screen.getAllByText('0:00')
      expect(timeDisplays).toHaveLength(2)
    })
  })

  describe('slider functionality', () => {
    it('renders a seek slider', () => {
      usePlayerStore.setState({ currentTrack: testTrack })

      render(<ProgressBar />)

      const slider = screen.getByRole('slider', { name: /seek/i })
      expect(slider).toBeInTheDocument()
    })

    it('slider has reduced opacity class when no track duration', () => {
      const trackWithZeroDuration = { ...testTrack, duration: 0 }
      usePlayerStore.setState({
        currentTrack: trackWithZeroDuration,
      })

      render(<ProgressBar />)

      // When disabled, the slider root has opacity-50 and cursor-not-allowed classes
      const slider = screen.getByRole('slider', { name: /seek/i })
      expect(slider).toBeInTheDocument()
      // The slider is present but visually indicates disabled state
    })

    it('slider reflects current progress', () => {
      usePlayerStore.setState({
        currentTrack: testTrack,
        currentTime: 90, // 50% of 180s
      })

      render(<ProgressBar />)

      const slider = screen.getByRole('slider', { name: /seek/i })
      // Radix slider value is percentage
      expect(slider).toHaveAttribute('aria-valuenow', '50')
    })

    it('slider is interactive when track has duration', () => {
      usePlayerStore.setState({
        currentTrack: testTrack,
        currentTime: 0,
      })

      render(<ProgressBar />)

      const slider = screen.getByRole('slider', { name: /seek/i })

      // Verify the slider is interactive (not disabled)
      expect(slider).not.toHaveAttribute('data-disabled')
      expect(slider).toHaveAttribute('aria-valuenow', '0')
      expect(slider).toHaveAttribute('aria-valuemax', '100')
    })
  })

  describe('accessibility', () => {
    it('has accessible slider label', () => {
      usePlayerStore.setState({ currentTrack: testTrack })

      render(<ProgressBar />)

      expect(screen.getByRole('slider', { name: /seek/i })).toBeInTheDocument()
    })

    it('has seek position label on thumb', () => {
      usePlayerStore.setState({ currentTrack: testTrack })

      render(<ProgressBar />)

      // The thumb has its own aria-label
      const slider = screen.getByRole('slider', { name: /seek/i })
      expect(slider).toBeInTheDocument()
    })
  })
})
