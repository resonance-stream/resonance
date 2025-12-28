/**
 * Home Page Tests
 *
 * Tests for the Home page component with real data hooks.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen } from '@/test/test-utils'
import Home from './Home'
import * as useLibraryModule from '../hooks/useLibrary'

// Mock the hooks to control loading/data states
vi.mock('../hooks/useLibrary', () => ({
  useRecentAlbums: vi.fn(() => ({
    data: undefined,
    isLoading: true,
    error: null,
  })),
  useMyPlaylists: vi.fn(() => ({
    data: undefined,
    isLoading: true,
    error: null,
  })),
  useTopTracks: vi.fn(() => ({
    data: undefined,
    isLoading: true,
    error: null,
  })),
}))

describe('Home', () => {
  let dateSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    if (dateSpy) {
      dateSpy.mockRestore()
    }
  })

  it('renders the greeting heading based on time of day', () => {
    // Mock Date.prototype.getHours to return evening time (8 PM)
    dateSpy = vi.spyOn(Date.prototype, 'getHours').mockReturnValue(20)

    render(<Home />)

    expect(
      screen.getByRole('heading', { name: /good evening/i })
    ).toBeInTheDocument()
  })

  it('renders morning greeting before noon', () => {
    dateSpy = vi.spyOn(Date.prototype, 'getHours').mockReturnValue(9)

    render(<Home />)

    expect(
      screen.getByRole('heading', { name: /good morning/i })
    ).toBeInTheDocument()
  })

  it('renders afternoon greeting between noon and 6 PM', () => {
    dateSpy = vi.spyOn(Date.prototype, 'getHours').mockReturnValue(14)

    render(<Home />)

    expect(
      screen.getByRole('heading', { name: /good afternoon/i })
    ).toBeInTheDocument()
  })

  it('renders the welcome message', () => {
    render(<Home />)

    expect(
      screen.getByText(/welcome back to resonance/i)
    ).toBeInTheDocument()
  })

  it('renders the Recently Added section', () => {
    render(<Home />)

    expect(
      screen.getByRole('heading', { name: /recently added/i })
    ).toBeInTheDocument()
  })

  it('renders the Your Playlists section', () => {
    render(<Home />)

    expect(
      screen.getByRole('heading', { name: /your playlists/i })
    ).toBeInTheDocument()
  })

  it('renders the Top Tracks section', () => {
    render(<Home />)

    expect(
      screen.getByRole('heading', { name: /top tracks/i })
    ).toBeInTheDocument()
  })

  it('renders loading skeletons while data is loading', () => {
    render(<Home />)

    // Should show skeleton cards during loading
    const loadingCards = screen.getAllByRole('status', { name: /loading card/i })
    expect(loadingCards.length).toBeGreaterThan(0)
  })

  describe('error states', () => {
    it('renders error message when albums fail to load', () => {
      vi.mocked(useLibraryModule.useRecentAlbums).mockReturnValue({
        data: undefined,
        isLoading: false,
        error: new Error('Failed to fetch'),
      } as ReturnType<typeof useLibraryModule.useRecentAlbums>)

      render(<Home />)

      expect(screen.getByText(/failed to load albums/i)).toBeInTheDocument()
    })

    it('renders error message when playlists fail to load', () => {
      vi.mocked(useLibraryModule.useMyPlaylists).mockReturnValue({
        data: undefined,
        isLoading: false,
        error: new Error('Failed to fetch'),
      } as ReturnType<typeof useLibraryModule.useMyPlaylists>)

      render(<Home />)

      expect(screen.getByText(/failed to load playlists/i)).toBeInTheDocument()
    })

    it('renders error message when top tracks fail to load', () => {
      vi.mocked(useLibraryModule.useTopTracks).mockReturnValue({
        data: undefined,
        isLoading: false,
        error: new Error('Failed to fetch'),
      } as ReturnType<typeof useLibraryModule.useTopTracks>)

      render(<Home />)

      expect(screen.getByText(/failed to load top tracks/i)).toBeInTheDocument()
    })
  })

  describe('data loaded states', () => {
    it('renders albums when data is loaded', () => {
      vi.mocked(useLibraryModule.useRecentAlbums).mockReturnValue({
        data: [
          {
            id: '1',
            title: 'Test Album',
            artist: { id: '1', name: 'Test Artist' },
            coverArtUrl: 'https://example.com/cover.jpg',
            tracks: [],
          },
        ],
        isLoading: false,
        error: null,
      } as unknown as ReturnType<typeof useLibraryModule.useRecentAlbums>)

      render(<Home />)

      expect(screen.getByText('Test Album')).toBeInTheDocument()
      expect(screen.getByText('Test Artist')).toBeInTheDocument()
    })

    it('renders playlists when data is loaded', () => {
      vi.mocked(useLibraryModule.useMyPlaylists).mockReturnValue({
        data: [
          {
            id: '1',
            name: 'My Playlist',
            description: 'A great playlist',
            trackCount: 10,
          },
        ],
        isLoading: false,
        error: null,
      } as ReturnType<typeof useLibraryModule.useMyPlaylists>)

      render(<Home />)

      expect(screen.getByText('My Playlist')).toBeInTheDocument()
      expect(screen.getByText('A great playlist')).toBeInTheDocument()
    })

    it('renders top tracks when data is loaded', () => {
      vi.mocked(useLibraryModule.useTopTracks).mockReturnValue({
        data: [
          {
            id: '1',
            title: 'Top Track',
            artist: { id: '1', name: 'Top Artist' },
            album: { id: '1', coverArtUrl: 'https://example.com/cover.jpg' },
            albumId: '1',
          },
        ],
        isLoading: false,
        error: null,
      } as ReturnType<typeof useLibraryModule.useTopTracks>)

      render(<Home />)

      expect(screen.getByText('Top Track')).toBeInTheDocument()
      expect(screen.getByText('Top Artist')).toBeInTheDocument()
    })

    it('renders empty state when no albums exist', () => {
      vi.mocked(useLibraryModule.useRecentAlbums).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
      } as unknown as ReturnType<typeof useLibraryModule.useRecentAlbums>)

      render(<Home />)

      expect(screen.getByText(/no albums yet/i)).toBeInTheDocument()
    })

    it('renders empty state when no playlists exist', () => {
      vi.mocked(useLibraryModule.useMyPlaylists).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
      } as unknown as ReturnType<typeof useLibraryModule.useMyPlaylists>)

      render(<Home />)

      expect(screen.getByText(/no playlists yet/i)).toBeInTheDocument()
    })

    it('renders empty state when no top tracks exist', () => {
      vi.mocked(useLibraryModule.useTopTracks).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
      } as unknown as ReturnType<typeof useLibraryModule.useTopTracks>)

      render(<Home />)

      expect(screen.getByText(/no top tracks yet/i)).toBeInTheDocument()
    })
  })
})
