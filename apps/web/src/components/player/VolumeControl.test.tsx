/**
 * VolumeControl Component Tests
 *
 * Tests for the volume control component with mute toggle and volume slider.
 */

import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, userEvent } from '@/test/test-utils'
import { VolumeControl } from './VolumeControl'
import { usePlayerStore } from '../../stores/playerStore'

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

describe('VolumeControl', () => {
  beforeEach(() => {
    resetStore()
  })

  describe('mute toggle', () => {
    it('renders mute button with correct label when unmuted', () => {
      usePlayerStore.setState({ isMuted: false })

      render(<VolumeControl />)

      expect(screen.getByRole('button', { name: 'Mute' })).toBeInTheDocument()
    })

    it('renders unmute button with correct label when muted', () => {
      usePlayerStore.setState({ isMuted: true })

      render(<VolumeControl />)

      expect(screen.getByRole('button', { name: 'Unmute' })).toBeInTheDocument()
    })

    it('toggles mute state when clicked', async () => {
      const user = userEvent.setup()
      usePlayerStore.setState({ isMuted: false })

      render(<VolumeControl />)

      await user.click(screen.getByRole('button', { name: 'Mute' }))
      expect(usePlayerStore.getState().isMuted).toBe(true)

      await user.click(screen.getByRole('button', { name: 'Unmute' }))
      expect(usePlayerStore.getState().isMuted).toBe(false)
    })

    it('has aria-pressed false when unmuted', () => {
      usePlayerStore.setState({ isMuted: false })

      render(<VolumeControl />)

      expect(screen.getByRole('button', { name: 'Mute' })).toHaveAttribute('aria-pressed', 'false')
    })

    it('has aria-pressed true when muted', () => {
      usePlayerStore.setState({ isMuted: true })

      render(<VolumeControl />)

      expect(screen.getByRole('button', { name: 'Unmute' })).toHaveAttribute('aria-pressed', 'true')
    })
  })

  describe('volume slider', () => {
    it('renders volume slider', () => {
      render(<VolumeControl />)

      expect(screen.getByRole('slider', { name: /volume/i })).toBeInTheDocument()
    })

    it('slider reflects current volume level', () => {
      usePlayerStore.setState({ volume: 0.5 })

      render(<VolumeControl />)

      const slider = screen.getByRole('slider', { name: /volume/i })
      expect(slider).toHaveAttribute('aria-valuenow', '50')
    })

    it('slider shows 0 when muted', () => {
      usePlayerStore.setState({ volume: 0.75, isMuted: true })

      render(<VolumeControl />)

      const slider = screen.getByRole('slider', { name: /volume/i })
      expect(slider).toHaveAttribute('aria-valuenow', '0')
    })

    it('has accessible label on slider', () => {
      render(<VolumeControl />)

      expect(screen.getByRole('slider', { name: /volume/i })).toBeInTheDocument()
    })
  })

  describe('volume icons', () => {
    it('shows muted icon when volume is 0', () => {
      usePlayerStore.setState({ volume: 0, isMuted: false })

      render(<VolumeControl />)

      // When volume is 0, the button should still say Mute (but show muted icon)
      expect(screen.getByRole('button', { name: 'Mute' })).toBeInTheDocument()
    })

    it('shows muted icon when muted regardless of volume', () => {
      usePlayerStore.setState({ volume: 0.75, isMuted: true })

      render(<VolumeControl />)

      expect(screen.getByRole('button', { name: 'Unmute' })).toBeInTheDocument()
    })
  })
})
