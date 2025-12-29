import { useState, useCallback } from 'react'
import { Search as SearchIcon, AlertCircle } from 'lucide-react'
import { Input } from '../components/ui/Input'
import { MediaCard } from '../components/media'
import { SkeletonCard, Skeleton } from '../components/ui/Skeleton'
import { useDebouncedValue } from '../hooks/useDebouncedValue'
import { useSearch } from '../hooks/useSearch'
import { usePlayerStore } from '../stores/playerStore'
import { mapGqlTrackToPlayerTrack, mapAlbumToPlayerTracks } from '../lib/mappers'
import type { GqlAlbum, GqlTrack } from '../types/library'

// Genre categories for browse mode
const GENRE_CATEGORIES = [
  { name: 'Rock', color: 'from-red-500/30 to-red-700/30' },
  { name: 'Pop', color: 'from-pink-500/30 to-pink-700/30' },
  { name: 'Hip Hop', color: 'from-amber-500/30 to-amber-700/30' },
  { name: 'Electronic', color: 'from-cyan-500/30 to-cyan-700/30' },
  { name: 'Jazz', color: 'from-indigo-500/30 to-indigo-700/30' },
  { name: 'Classical', color: 'from-purple-500/30 to-purple-700/30' },
  { name: 'R&B', color: 'from-teal-500/30 to-teal-700/30' },
  { name: 'Country', color: 'from-orange-500/30 to-orange-700/30' },
  { name: 'Metal', color: 'from-slate-500/30 to-slate-700/30' },
  { name: 'Folk', color: 'from-emerald-500/30 to-emerald-700/30' },
] as const

export default function Search() {
  const [query, setQuery] = useState('')
  const debouncedQuery = useDebouncedValue(query, 300)

  const { data: results, isLoading, error } = useSearch(debouncedQuery, 12)

  // Player store actions
  const setQueue = usePlayerStore((s) => s.setQueue)
  const setTrack = usePlayerStore((s) => s.setTrack)

  // Handle playing an album
  const handlePlayAlbum = useCallback((album: GqlAlbum) => {
    if (album.tracks?.length) {
      const tracks = mapAlbumToPlayerTracks(album)
      setQueue(tracks, 0)
    }
  }, [setQueue])

  // Handle playing a track
  const handlePlayTrack = useCallback((track: GqlTrack) => {
    const playerTrack = mapGqlTrackToPlayerTrack(track)
    setTrack(playerTrack)
  }, [setTrack])

  // Handle genre click (for future implementation)
  const handleGenreClick = useCallback((genre: string) => {
    setQuery(genre)
  }, [])

  // Handle keyboard on genre cards
  const handleGenreKeyDown = useCallback((e: React.KeyboardEvent, genre: string) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      handleGenreClick(genre)
    }
  }, [handleGenreClick])

  const hasQuery = debouncedQuery.length >= 2
  const hasResults = results && (
    results.albums.length > 0 ||
    results.artists.length > 0 ||
    results.tracks.length > 0
  )

  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Header */}
      <div className="mb-8">
        <h1 className="font-display text-display text-text-primary">
          Search
        </h1>
        <p className="mt-2 text-text-secondary">
          Find your favorite music
        </p>
      </div>

      {/* Search Input */}
      <div className="max-w-2xl mb-8">
        <Input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search for songs, albums, or artists..."
          icon={<SearchIcon size={20} />}
          aria-label="Search music"
        />
      </div>

      {/* Loading State */}
      {hasQuery && isLoading && (
        <div className="space-y-8">
          <section>
            <Skeleton className="h-6 w-24 mb-4" />
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
              {Array.from({ length: 6 }).map((_, i) => (
                <SkeletonCard key={i} />
              ))}
            </div>
          </section>
        </div>
      )}

      {/* Error State */}
      {hasQuery && error && (
        <div role="alert" className="flex flex-col items-center justify-center py-12 text-center">
          <AlertCircle size={48} className="text-text-muted mb-4" aria-hidden="true" />
          <h2 className="text-xl font-semibold text-text-primary mb-2">
            Search failed
          </h2>
          <p className="text-text-secondary max-w-md">
            {error.message || 'Unable to complete the search. Please try again.'}
          </p>
        </div>
      )}

      {/* No Results State */}
      {hasQuery && !isLoading && !error && !hasResults && (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <SearchIcon size={48} className="text-text-muted mb-4" />
          <h2 className="text-xl font-semibold text-text-primary mb-2">
            No results found
          </h2>
          <p className="text-text-secondary max-w-md">
            We couldn't find anything matching "{debouncedQuery}". Try a different search term.
          </p>
        </div>
      )}

      {/* Search Results */}
      {hasQuery && !isLoading && !error && hasResults && (
        <div className="space-y-8">
          {/* Artists Section */}
          {results.artists.length > 0 && (
            <section>
              <h2 className="text-xl font-semibold text-text-primary mb-4">
                Artists
              </h2>
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                {results.artists.map((artist) => (
                  <MediaCard
                    key={artist.id}
                    title={artist.name}
                    subtitle="Artist"
                    imageUrl={artist.imageUrl}
                    href={`/artist/${artist.id}`}
                  />
                ))}
              </div>
            </section>
          )}

          {/* Albums Section */}
          {results.albums.length > 0 && (
            <section>
              <h2 className="text-xl font-semibold text-text-primary mb-4">
                Albums
              </h2>
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                {results.albums.map((album) => (
                  <MediaCard
                    key={album.id}
                    title={album.title}
                    subtitle={album.artist?.name ?? 'Unknown Artist'}
                    imageUrl={album.coverArtUrl}
                    href={`/album/${album.id}`}
                    onPlay={album.tracks?.length ? () => handlePlayAlbum(album) : undefined}
                  />
                ))}
              </div>
            </section>
          )}

          {/* Tracks Section */}
          {results.tracks.length > 0 && (
            <section>
              <h2 className="text-xl font-semibold text-text-primary mb-4">
                Songs
              </h2>
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                {results.tracks.map((track) => (
                  <MediaCard
                    key={track.id}
                    title={track.title}
                    subtitle={track.artist?.name ?? 'Unknown Artist'}
                    imageUrl={track.album?.coverArtUrl}
                    href={track.albumId ? `/album/${track.albumId}` : undefined}
                    onPlay={() => handlePlayTrack(track)}
                  />
                ))}
              </div>
            </section>
          )}
        </div>
      )}

      {/* Browse Categories (when no search query) */}
      {!hasQuery && (
        <section>
          <h2 className="text-xl font-semibold text-text-primary mb-4">
            Browse All
          </h2>
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
            {GENRE_CATEGORIES.map((genre) => (
              <div
                key={genre.name}
                role="button"
                tabIndex={0}
                onClick={() => handleGenreClick(genre.name)}
                onKeyDown={(e) => handleGenreKeyDown(e, genre.name)}
                className={`aspect-square rounded-lg bg-gradient-to-br ${genre.color} flex items-end p-4 cursor-pointer hover:scale-[1.02] transition-transform focus:outline-none focus:ring-2 focus:ring-mint/50`}
                aria-label={`Browse ${genre.name} genre`}
              >
                <span className="font-semibold text-text-primary">
                  {genre.name}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  )
}
