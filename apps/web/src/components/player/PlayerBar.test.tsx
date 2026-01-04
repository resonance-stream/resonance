/**
 * PlayerBar Component Tests
 *
 * Tests for the main PlayerBar component that contains all player controls.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, userEvent } from '@/test/test-utils'
import { PlayerBar } from './PlayerBar'
import { usePlayerStore } from '../../stores/playerStore'
import { useEqualizerStore } from '../../stores/equalizerStore'

// Mock child components to isolate PlayerBar testing
vi.mock('./NowPlaying', () => ({
  NowPlaying: () => <div data-testid="now-playing">Now Playing Mock</div>,
}))

vi.mock('./PlaybackControls', () => ({
  PlaybackControls: () => <div data-testid="playback-controls">Playback Controls Mock</div>,
}))

vi.mock('./VolumeControl', () => ({
  VolumeControl: () => <div data-testid="volume-control">Volume Control Mock</div>,
}))

vi.mock('../equalizer', () => ({
  EqualizerPanel: ({ onClose }: { onClose: () => void }) => (
    <div data-testid="equalizer-panel">
      Equalizer Panel Mock
      <button onClick={onClose}>Close EQ</button>
    </div>
  ),
}))

vi.mock('../queue', () => ({
  QueuePanel: ({ onClose }: { onClose: () => void }) => (
    <div data-testid="queue-panel">
      Queue Panel Mock
      <button onClick={onClose}>Close Queue</button>
    </div>
  ),
}))

vi.mock('../visualizer', () => ({
  FullscreenPlayer: () => <div data-testid="fullscreen-player">Fullscreen Player Mock</div>,
}))

// Sample test track
const testTrack = {
  id: '1',
  title: 'Test Track',
  artist: 'Test Artist',
  albumId: 'album-1',
  albumTitle: 'Test Album',
  duration: 180,
  coverUrl: '/covers/1.jpg',
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
  useEqualizerStore.setState({
    settings: { enabled: false, preamp: 0, bands: {} as never },
    activePreset: 'flat',
    customPresets: [],
  })
}

describe('PlayerBar', () => {
  beforeEach(() => {
    resetStores()
  })

  it('renders nothing when no track is loaded', () => {
    const { container } = render(<PlayerBar />)

    // Should not render the player bar
    expect(screen.queryByRole('region', { name: /audio player/i })).not.toBeInTheDocument()
    expect(container.firstChild).toBeNull()
  })

  it('renders the player bar when a track is loaded', () => {
    usePlayerStore.setState({ currentTrack: testTrack })

    render(<PlayerBar />)

    expect(screen.getByRole('region', { name: /audio player/i })).toBeInTheDocument()
    expect(screen.getByTestId('now-playing')).toBeInTheDocument()
    expect(screen.getByTestId('playback-controls')).toBeInTheDocument()
    expect(screen.getByTestId('volume-control')).toBeInTheDocument()
  })

  it('has accessible region label', () => {
    usePlayerStore.setState({ currentTrack: testTrack })

    render(<PlayerBar />)

    const region = screen.getByRole('region', { name: /audio player/i })
    expect(region).toHaveAttribute('aria-label', 'Audio player')
  })

  it('toggles equalizer panel when EQ button is clicked', async () => {
    const user = userEvent.setup()
    usePlayerStore.setState({ currentTrack: testTrack })

    render(<PlayerBar />)

    const eqButton = screen.getByRole('button', { name: /show equalizer/i })
    expect(screen.queryByTestId('equalizer-panel')).not.toBeInTheDocument()

    await user.click(eqButton)
    expect(screen.getByTestId('equalizer-panel')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /hide equalizer/i })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /hide equalizer/i }))
    expect(screen.queryByTestId('equalizer-panel')).not.toBeInTheDocument()
  })

  it('toggles queue panel when queue button is clicked', async () => {
    const user = userEvent.setup()
    usePlayerStore.setState({ currentTrack: testTrack })

    render(<PlayerBar />)

    const queueButton = screen.getByRole('button', { name: /show queue/i })
    expect(screen.queryByTestId('queue-panel')).not.toBeInTheDocument()

    await user.click(queueButton)
    expect(screen.getByTestId('queue-panel')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /hide queue/i })).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /hide queue/i }))
    expect(screen.queryByTestId('queue-panel')).not.toBeInTheDocument()
  })

  it('closes equalizer when queue is opened (only one panel at a time)', async () => {
    const user = userEvent.setup()
    usePlayerStore.setState({ currentTrack: testTrack })

    render(<PlayerBar />)

    // Open equalizer
    await user.click(screen.getByRole('button', { name: /show equalizer/i }))
    expect(screen.getByTestId('equalizer-panel')).toBeInTheDocument()

    // Open queue - should close equalizer
    await user.click(screen.getByRole('button', { name: /show queue/i }))
    expect(screen.getByTestId('queue-panel')).toBeInTheDocument()
    expect(screen.queryByTestId('equalizer-panel')).not.toBeInTheDocument()
  })

  it('shows accent color on queue button when queue has tracks', () => {
    usePlayerStore.setState({
      currentTrack: testTrack,
      queue: [testTrack],
    })

    render(<PlayerBar />)

    const queueButton = screen.getByRole('button', { name: /show queue/i })
    expect(queueButton).toHaveClass('text-accent')
  })
})
