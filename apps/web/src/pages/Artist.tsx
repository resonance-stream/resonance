import { useCallback } from 'react'
import { useParams } from 'react-router-dom'
import { Play, UserPlus, MoreHorizontal, AlertCircle } from 'lucide-react'
import { Button } from '../components/ui/Button'
import { MediaCard } from '../components/media'
import { AlbumArt } from '../components/media/AlbumArt'
import { SkeletonCard, Skeleton } from '../components/ui/Skeleton'
import { useArtist } from '../hooks/useLibrary'
import { usePlayerStore } from '../stores/playerStore'
import { mapGqlTrackToPlayerTrack, mapAlbumToPlayerTracks, formatDurationMs } from '../lib/mappers'
import type { GqlTrack, GqlAlbum } from '../types/library'

/**
 * Format play count for display (e.g., 1.2M, 500K)
 */
function formatPlayCount(count: number | undefined): string {
  if (!count) return ''
  if (count >= 1_000_000_000) return `${(count / 1_000_000_000).toFixed(1)}B`
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`
  return String(count)
}

export default function Artist() {
  const { id } = useParams<{ id: string }>()
  const { data: artist, isLoading, error } = useArtist(id)

  // Granular selectors to minimize re-renders
  const setQueue = usePlayerStore((s) => s.setQueue)
  const togglePlay = usePlayerStore((s) => s.togglePlay)
  const currentTrackId = usePlayerStore((s) => s.currentTrack?.id)
  const isPlaying = usePlayerStore((s) => s.isPlaying)

  // Handle playing a track
  const handlePlayTrack = useCallback((track: GqlTrack, index: number, allTracks: GqlTrack[]) => {
    // If clicking the currently playing track, toggle play/pause
    if (currentTrackId === track.id) {
      togglePlay()
      return
    }

    // Set queue from clicked track
    const playerTracks = allTracks.map((t) => mapGqlTrackToPlayerTrack(t))
    setQueue(playerTracks, index)
  }, [currentTrackId, togglePlay, setQueue])

  // Handle keyboard navigation on track rows
  const handleTrackKeyDown = useCallback((
    e: React.KeyboardEvent,
    track: GqlTrack,
    index: number,
    allTracks: GqlTrack[]
  ) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      handlePlayTrack(track, index, allTracks)
    }
  }, [handlePlayTrack])

  // Handle playing an album
  const handlePlayAlbum = useCallback((album: GqlAlbum) => {
    if (album.tracks?.length) {
      const tracks = mapAlbumToPlayerTracks(album)
      setQueue(tracks, 0)
    }
  }, [setQueue])

  // Handle playing all top tracks
  const handlePlayAll = useCallback(() => {
    if (!artist?.topTracks?.length) return
    const tracks = artist.topTracks.map((t) => mapGqlTrackToPlayerTrack(t))
    setQueue(tracks, 0)
  }, [artist, setQueue])

  // Handle missing artist ID
  if (!id) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center p-6 animate-fade-in">
        <AlertCircle size={48} className="text-text-muted mb-4" />
        <h2 className="text-xl font-semibold text-text-primary mb-2">
          Invalid Artist URL
        </h2>
        <p className="text-text-secondary">
          No artist ID was provided in the URL.
        </p>
      </div>
    )
  }

  // Loading state
  if (isLoading) {
    return (
      <div className="flex flex-1 flex-col animate-fade-in">
        {/* Hero Section Skeleton */}
        <div className="relative h-80 bg-gradient-to-b from-accent-dark/50 to-background flex items-end p-6">
          <div className="space-y-3">
            <Skeleton className="h-4 w-16" />
            <Skeleton className="h-12 w-64" />
            <Skeleton className="h-5 w-40" />
          </div>
        </div>

        <div className="p-6">
          {/* Action buttons skeleton */}
          <div className="flex items-center gap-4 mb-8">
            <Skeleton className="h-10 w-28" rounded="full" />
            <Skeleton className="h-10 w-24" />
            <Skeleton className="h-10 w-10" rounded="full" />
          </div>

          {/* Popular section skeleton */}
          <section className="mb-8">
            <Skeleton className="h-6 w-24 mb-4" />
            <div className="space-y-2">
              {Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="flex items-center gap-4 py-2">
                  <Skeleton className="h-4 w-6" />
                  <Skeleton className="h-10 w-10" rounded="sm" />
                  <div className="flex-1 space-y-1">
                    <Skeleton className="h-4 w-48" />
                    <Skeleton className="h-3 w-32" />
                  </div>
                  <Skeleton className="h-4 w-12" />
                </div>
              ))}
            </div>
          </section>

          {/* Discography skeleton */}
          <section>
            <Skeleton className="h-6 w-32 mb-4" />
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
              {Array.from({ length: 6 }).map((_, i) => (
                <SkeletonCard key={i} />
              ))}
            </div>
          </section>
        </div>
      </div>
    )
  }

  // Error state
  if (error || !artist) {
    return (
      <div role="alert" className="flex flex-1 flex-col items-center justify-center p-6 animate-fade-in">
        <AlertCircle size={48} className="text-text-muted mb-4" aria-hidden="true" />
        <h2 className="text-xl font-semibold text-text-primary mb-2">
          Artist not found
        </h2>
        <p className="text-text-secondary">
          {error?.message || 'The artist you requested could not be loaded.'}
        </p>
      </div>
    )
  }

  const topTracks = artist.topTracks ?? []
  const albums = artist.albums ?? []

  return (
    <div className="flex flex-1 flex-col animate-fade-in">
      {/* Hero Section */}
      <div className="relative h-80 bg-gradient-to-b from-accent-dark/50 to-background flex items-end p-6">
        {artist.imageUrl && (
          <div className="absolute inset-0 overflow-hidden">
            <img
              src={artist.imageUrl}
              alt=""
              className="w-full h-full object-cover opacity-20 blur-sm"
            />
            <div className="absolute inset-0 bg-gradient-to-b from-transparent to-background" />
          </div>
        )}
        <div className="relative z-10">
          <span className="text-overline text-text-muted uppercase tracking-wider">
            Artist
          </span>
          <h1 className="font-display text-display-xl text-text-primary mt-2">
            {artist.name}
          </h1>
          {artist.albumCount !== undefined && artist.albumCount > 0 && (
            <p className="text-text-secondary mt-2">
              {artist.albumCount} {artist.albumCount === 1 ? 'album' : 'albums'}
            </p>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="p-6">
        {/* Action Buttons */}
        <div className="flex items-center gap-4 mb-8">
          <Button
            variant="accent"
            className="gap-2 rounded-full px-8"
            onClick={handlePlayAll}
            disabled={topTracks.length === 0}
          >
            <Play size={20} fill="currentColor" className="ml-0.5" />
            Play
          </Button>
          <Button variant="secondary" className="gap-2">
            <UserPlus size={18} />
            Follow
          </Button>
          <Button variant="ghost" size="icon" aria-label="More options">
            <MoreHorizontal size={24} />
          </Button>
        </div>

        {/* Popular Tracks */}
        {topTracks.length > 0 && (
          <section className="mb-8">
            <h2 className="text-xl font-semibold text-text-primary mb-4">
              Popular
            </h2>
            <ol className="space-y-1" role="list" aria-label="Popular tracks">
              {topTracks.slice(0, 5).map((track, index) => {
                const isCurrentTrack = currentTrackId === track.id
                const isPlayingTrack = isCurrentTrack && isPlaying

                return (
                  <li key={track.id}>
                    <div
                      role="button"
                      tabIndex={0}
                      onClick={() => handlePlayTrack(track, index, topTracks.slice(0, 5))}
                      onKeyDown={(e) => handleTrackKeyDown(e, track, index, topTracks.slice(0, 5))}
                      aria-label={`Play ${track.title}${isPlayingTrack ? ' (currently playing)' : ''}`}
                      aria-current={isCurrentTrack ? 'true' : undefined}
                      className={`flex items-center gap-4 px-4 py-3 hover:bg-background-tertiary/50 rounded-lg cursor-pointer group transition-colors focus:outline-none focus:ring-2 focus:ring-mint/50 ${
                        isCurrentTrack ? 'bg-accent-dark/10' : ''
                      }`}
                    >
                      <span className={`w-6 text-center ${isCurrentTrack ? 'text-mint' : 'text-text-muted'} group-hover:hidden`}>
                        {isPlayingTrack ? (
                          <span className="inline-flex gap-0.5">
                            <span className="w-0.5 h-3 bg-mint animate-pulse" />
                            <span className="w-0.5 h-3 bg-mint animate-pulse delay-75" />
                            <span className="w-0.5 h-3 bg-mint animate-pulse delay-150" />
                          </span>
                        ) : (
                          index + 1
                        )}
                      </span>
                      <span className="w-6 hidden group-hover:flex items-center justify-center">
                        <Play size={14} className="text-navy" fill="currentColor" />
                      </span>
                      <AlbumArt
                        src={track.album?.coverArtUrl}
                        alt={track.album?.title ?? 'Album cover'}
                        size="sm"
                        showPlayButton={false}
                        className="flex-shrink-0"
                      />
                      <div className="flex-1 min-w-0">
                        <p className={`truncate ${isCurrentTrack ? 'text-mint' : 'text-text-primary'}`}>
                          {track.title}
                        </p>
                        <p className="text-sm text-text-muted truncate">
                          {track.album?.title ?? 'Unknown Album'}
                        </p>
                      </div>
                      <span className="text-text-muted text-sm">
                        {track.formattedDuration || formatDurationMs(track.durationMs)}
                      </span>
                      {track.playCount && (
                        <span className="text-text-muted text-sm w-16 text-right">
                          {formatPlayCount(track.playCount)}
                        </span>
                      )}
                    </div>
                  </li>
                )
              })}
            </ol>
          </section>
        )}

        {/* Discography */}
        {albums.length > 0 && (
          <section>
            <h2 className="text-xl font-semibold text-text-primary mb-4">
              Discography
            </h2>
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
              {albums.map((album) => (
                <MediaCard
                  key={album.id}
                  title={album.title}
                  subtitle={album.releaseYear ? String(album.releaseYear) : 'Unknown year'}
                  imageUrl={album.coverArtUrl}
                  href={`/album/${album.id}`}
                  onPlay={album.tracks?.length ? () => handlePlayAlbum(album) : undefined}
                />
              ))}
            </div>
          </section>
        )}

        {/* Empty state for artist with no content */}
        {topTracks.length === 0 && albums.length === 0 && (
          <div className="py-12 text-center text-text-muted">
            No tracks or albums available for this artist
          </div>
        )}
      </div>
    </div>
  )
}
