import { useState, useCallback, useMemo } from 'react'
import { LayoutGrid, List, AlertCircle } from 'lucide-react'
import { Button } from '../components/ui/Button'
import { MediaCard } from '../components/media'
import { SkeletonCard } from '../components/ui/Skeleton'
import { useAlbums, useMyPlaylists, useArtists } from '../hooks/useLibrary'
import { usePlayerStore } from '../stores/playerStore'
import { mapAlbumToPlayerTracks } from '../lib/mappers'
import { cn } from '../lib/utils'
import type { GqlAlbum } from '../types/library'

type ViewMode = 'grid' | 'list'
type FilterType = 'all' | 'albums' | 'playlists' | 'artists'

const FILTERS: { key: FilterType; label: string }[] = [
  { key: 'all', label: 'All' },
  { key: 'albums', label: 'Albums' },
  { key: 'playlists', label: 'Playlists' },
  { key: 'artists', label: 'Artists' },
]

export default function Library() {
  const [viewMode, setViewMode] = useState<ViewMode>('grid')
  const [filter, setFilter] = useState<FilterType>('all')

  // Fetch data based on filter
  const shouldFetchAlbums = filter === 'all' || filter === 'albums'
  const shouldFetchPlaylists = filter === 'all' || filter === 'playlists'
  const shouldFetchArtists = filter === 'all' || filter === 'artists'

  const {
    data: albums,
    isLoading: loadingAlbums,
    error: albumsError
  } = useAlbums({ limit: 50 }, { enabled: shouldFetchAlbums })

  const {
    data: playlists,
    isLoading: loadingPlaylists,
    error: playlistsError
  } = useMyPlaylists({ limit: 50 }, { enabled: shouldFetchPlaylists })

  const {
    data: artists,
    isLoading: loadingArtists,
    error: artistsError
  } = useArtists({ limit: 50 }, { enabled: shouldFetchArtists })

  // Player store actions
  const setQueue = usePlayerStore((s) => s.setQueue)

  // Handle playing an album
  const handlePlayAlbum = useCallback((album: GqlAlbum) => {
    if (album.tracks?.length) {
      const tracks = mapAlbumToPlayerTracks(album)
      setQueue(tracks, 0)
    }
  }, [setQueue])

  // Determine loading state
  const isLoading = (
    (shouldFetchAlbums && loadingAlbums) ||
    (shouldFetchPlaylists && loadingPlaylists) ||
    (shouldFetchArtists && loadingArtists)
  )

  // Check for errors
  const error = albumsError || playlistsError || artistsError

  // Combine items for "all" view
  const items = useMemo(() => {
    const result: Array<{
      type: 'album' | 'playlist' | 'artist'
      id: string
      title: string
      subtitle: string
      imageUrl?: string
      href: string
      onPlay?: () => void
    }> = []

    if (shouldFetchAlbums && albums) {
      albums.forEach((album) => {
        const canPlay = Boolean(album.tracks?.length)
        result.push({
          type: 'album',
          id: album.id,
          title: album.title,
          subtitle: album.artist?.name ?? 'Unknown Artist',
          imageUrl: album.coverArtUrl,
          href: `/album/${album.id}`,
          onPlay: canPlay ? () => handlePlayAlbum(album) : undefined,
        })
      })
    }

    if (shouldFetchPlaylists && playlists) {
      playlists.forEach((playlist) => {
        result.push({
          type: 'playlist',
          id: playlist.id,
          title: playlist.name,
          subtitle: playlist.description || `${playlist.trackCount ?? 0} tracks`,
          imageUrl: playlist.imageUrl,
          href: `/playlist/${playlist.id}`,
        })
      })
    }

    if (shouldFetchArtists && artists) {
      artists.forEach((artist) => {
        result.push({
          type: 'artist',
          id: artist.id,
          title: artist.name,
          subtitle: 'Artist',
          imageUrl: artist.imageUrl,
          href: `/artist/${artist.id}`,
        })
      })
    }

    return result
  }, [albums, playlists, artists, shouldFetchAlbums, shouldFetchPlaylists, shouldFetchArtists, handlePlayAlbum])

  // Empty state check
  const isEmpty = !isLoading && items.length === 0

  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="font-display text-display text-text-primary">
            Your Library
          </h1>
          <p className="mt-1 text-text-secondary">
            Your saved albums, playlists, and artists
          </p>
        </div>

        {/* View Toggle */}
        <div className="flex items-center gap-2">
          <Button
            variant={viewMode === 'grid' ? 'secondary' : 'ghost'}
            size="icon"
            onClick={() => setViewMode('grid')}
            aria-label="Grid view"
            aria-pressed={viewMode === 'grid'}
          >
            <LayoutGrid size={18} />
          </Button>
          <Button
            variant={viewMode === 'list' ? 'secondary' : 'ghost'}
            size="icon"
            onClick={() => setViewMode('list')}
            aria-label="List view"
            aria-pressed={viewMode === 'list'}
          >
            <List size={18} />
          </Button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-2 mb-6" role="tablist" aria-label="Library filter">
        {FILTERS.map((f) => (
          <Button
            key={f.key}
            variant={filter === f.key ? 'secondary' : 'ghost'}
            size="sm"
            onClick={() => setFilter(f.key)}
            role="tab"
            aria-selected={filter === f.key}
            aria-controls="library-content"
          >
            {f.label}
          </Button>
        ))}
      </div>

      {/* Error State */}
      {error && (
        <div role="alert" className="flex flex-col items-center justify-center py-12 text-center">
          <AlertCircle size={48} className="text-text-muted mb-4" aria-hidden="true" />
          <h2 className="text-xl font-semibold text-text-primary mb-2">
            Failed to load library
          </h2>
          <p className="text-text-secondary max-w-md">
            {error.message || 'Unable to load your library. Please try again.'}
          </p>
        </div>
      )}

      {/* Loading State */}
      {isLoading && !error && (
        <div
          aria-busy="true"
          aria-label="Loading library content"
          className={cn(
            'grid gap-4',
            viewMode === 'grid'
              ? 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'
              : 'grid-cols-1'
          )}
        >
          {Array.from({ length: 12 }).map((_, i) => (
            <SkeletonCard key={i} />
          ))}
        </div>
      )}

      {/* Empty State */}
      {isEmpty && !error && (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <div className="w-16 h-16 rounded-full bg-background-tertiary flex items-center justify-center mb-4">
            <LayoutGrid size={32} className="text-text-muted" />
          </div>
          <h2 className="text-xl font-semibold text-text-primary mb-2">
            Your library is empty
          </h2>
          <p className="text-text-secondary max-w-md">
            {filter === 'all'
              ? 'Start building your library by saving albums, creating playlists, or following artists.'
              : filter === 'albums'
              ? 'You haven\'t saved any albums yet.'
              : filter === 'playlists'
              ? 'You haven\'t created any playlists yet.'
              : 'You haven\'t followed any artists yet.'}
          </p>
        </div>
      )}

      {/* Content */}
      {!isLoading && !error && items.length > 0 && (
        <div
          id="library-content"
          className={cn(
            'grid gap-4',
            viewMode === 'grid'
              ? 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'
              : 'grid-cols-1'
          )}
        >
          {items.map((item) => (
            <MediaCard
              key={`${item.type}-${item.id}`}
              title={item.title}
              subtitle={item.subtitle}
              imageUrl={item.imageUrl}
              href={item.href}
              onPlay={item.onPlay}
            />
          ))}
        </div>
      )}
    </div>
  )
}
