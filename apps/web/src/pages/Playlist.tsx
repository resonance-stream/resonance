import { useCallback } from 'react'
import { useParams } from 'react-router-dom'
import { Play, Heart, MoreHorizontal, Clock, Shuffle, AlertCircle } from 'lucide-react'
import { Button } from '../components/ui/Button'
import { AlbumArt } from '../components/media/AlbumArt'
import { SkeletonHeader, Skeleton } from '../components/ui/Skeleton'
import { usePlaylist } from '../hooks/useLibrary'
import { usePlayerStore } from '../stores/playerStore'
import { mapGqlTrackToPlayerTrack, formatDurationMs } from '../lib/mappers'
import type { GqlPlaylistTrackEntry } from '../types/library'

/**
 * Fisher-Yates shuffle algorithm for unbiased random ordering
 */
function fisherYatesShuffle<T>(array: T[]): T[] {
  const shuffled = [...array]
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[shuffled[i], shuffled[j]] = [shuffled[j]!, shuffled[i]!]
  }
  return shuffled
}

/**
 * Format relative time for "added at" display
 */
function formatAddedAt(dateStr: string | undefined): string {
  if (!dateStr) return ''
  try {
    const date = new Date(dateStr)
    const now = new Date()
    const diffDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24))

    if (diffDays === 0) return 'Today'
    if (diffDays === 1) return 'Yesterday'
    if (diffDays < 7) return `${diffDays} days ago`
    if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`
    if (diffDays < 365) return `${Math.floor(diffDays / 30)} months ago`
    return `${Math.floor(diffDays / 365)} years ago`
  } catch {
    return ''
  }
}

export default function Playlist() {
  const { id } = useParams<{ id: string }>()
  const { data: playlist, isLoading, error } = usePlaylist(id)

  // Granular selectors to minimize re-renders
  const setQueue = usePlayerStore((s) => s.setQueue)
  const togglePlay = usePlayerStore((s) => s.togglePlay)
  const currentTrackId = usePlayerStore((s) => s.currentTrack?.id)
  const isPlaying = usePlayerStore((s) => s.isPlaying)

  // Extract tracks from playlist entries
  const getTracksFromEntries = useCallback((entries: GqlPlaylistTrackEntry[]) => {
    return entries.map((entry) => mapGqlTrackToPlayerTrack(entry.track))
  }, [])

  // Handle playing all tracks
  const handlePlayAll = useCallback(() => {
    if (!playlist?.tracks?.length) return
    const tracks = getTracksFromEntries(playlist.tracks)
    setQueue(tracks, 0)
  }, [playlist, setQueue, getTracksFromEntries])

  // Handle shuffle play with Fisher-Yates for unbiased randomness
  const handleShufflePlay = useCallback(() => {
    if (!playlist?.tracks?.length) return
    const tracks = getTracksFromEntries(playlist.tracks)
    const shuffled = fisherYatesShuffle(tracks)
    setQueue(shuffled, 0)
  }, [playlist, setQueue, getTracksFromEntries])

  // Handle playing a track
  const handlePlayTrack = useCallback((entry: GqlPlaylistTrackEntry, index: number) => {
    if (!playlist?.tracks) return

    // If clicking the currently playing track, toggle play/pause
    if (currentTrackId === entry.track.id) {
      togglePlay()
      return
    }

    // Set queue from clicked track
    const tracks = getTracksFromEntries(playlist.tracks)
    setQueue(tracks, index)
  }, [playlist, currentTrackId, togglePlay, setQueue, getTracksFromEntries])

  // Handle keyboard navigation on track rows
  const handleTrackKeyDown = useCallback((
    e: React.KeyboardEvent,
    entry: GqlPlaylistTrackEntry,
    index: number
  ) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      handlePlayTrack(entry, index)
    }
  }, [handlePlayTrack])

  // Handle missing playlist ID
  if (!id) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center p-6 animate-fade-in">
        <AlertCircle size={48} className="text-text-muted mb-4" />
        <h2 className="text-xl font-semibold text-text-primary mb-2">
          Invalid Playlist URL
        </h2>
        <p className="text-text-secondary">
          No playlist ID was provided in the URL.
        </p>
      </div>
    )
  }

  // Loading state
  if (isLoading) {
    return (
      <div className="flex flex-1 flex-col p-6 animate-fade-in">
        <SkeletonHeader className="mb-8" />

        {/* Action buttons skeleton */}
        <div className="flex items-center gap-4 mb-8">
          <Skeleton className="h-10 w-28" rounded="full" />
          <Skeleton className="h-10 w-10" rounded="full" />
          <Skeleton className="h-10 w-10" rounded="full" />
          <Skeleton className="h-10 w-10" rounded="full" />
        </div>

        {/* Track list header */}
        <div className="grid grid-cols-[auto_1fr_1fr_auto_auto] gap-4 px-4 py-2 text-sm text-text-muted border-b border-white/5">
          <span className="w-8 text-center">#</span>
          <span>Title</span>
          <span>Album</span>
          <span className="w-24 text-right">Added</span>
          <span className="w-16 text-right flex items-center justify-end">
            <Clock size={16} />
          </span>
        </div>

        {/* Track skeletons */}
        <div className="divide-y divide-white/5">
          {Array.from({ length: 10 }).map((_, i) => (
            <div key={i} className="flex items-center gap-4 py-3 px-4">
              <Skeleton className="h-4 w-6" />
              <div className="flex-1 space-y-1">
                <Skeleton className="h-4 w-48" />
                <Skeleton className="h-3 w-32" />
              </div>
              <Skeleton className="h-4 w-32" />
              <Skeleton className="h-4 w-20" />
              <Skeleton className="h-4 w-12" />
            </div>
          ))}
        </div>
      </div>
    )
  }

  // Error state
  if (error || !playlist) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center p-6 animate-fade-in">
        <AlertCircle size={48} className="text-text-muted mb-4" />
        <h2 className="text-xl font-semibold text-text-primary mb-2">
          Playlist not found
        </h2>
        <p className="text-text-secondary">
          {error?.message || 'The playlist you requested could not be loaded.'}
        </p>
      </div>
    )
  }

  const tracks = playlist.tracks ?? []

  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Playlist Header */}
      <div className="flex flex-col md:flex-row gap-8 mb-8">
        <AlbumArt
          src={playlist.imageUrl}
          alt={playlist.name}
          size="xl"
          showPlayButton={false}
          className="flex-shrink-0 self-center md:self-start"
        />

        <div className="flex flex-col justify-end">
          <span className="text-overline text-text-muted uppercase tracking-wider">
            {playlist.isPublic ? 'Public Playlist' : 'Playlist'}
          </span>
          <h1 className="font-display text-display-xl text-text-primary mt-2">
            {playlist.name}
          </h1>
          {playlist.description && (
            <p className="text-text-secondary mt-2 max-w-lg">
              {playlist.description}
            </p>
          )}
          <p className="text-sm text-text-muted mt-2">
            {playlist.trackCount ?? tracks.length} songs
            {playlist.formattedDuration && ` - ${playlist.formattedDuration}`}
          </p>

          {/* Action Buttons */}
          <div className="flex items-center gap-4 mt-6">
            <Button
              variant="accent"
              className="gap-2 rounded-full px-8"
              onClick={handlePlayAll}
              disabled={tracks.length === 0}
            >
              <Play size={20} fill="currentColor" className="ml-0.5" />
              Play
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Shuffle"
              onClick={handleShufflePlay}
              disabled={tracks.length === 0}
            >
              <Shuffle size={22} />
            </Button>
            <Button variant="ghost" size="icon" aria-label="Like playlist" className="hover:text-mint">
              <Heart size={24} />
            </Button>
            <Button variant="ghost" size="icon" aria-label="More options">
              <MoreHorizontal size={24} />
            </Button>
          </div>
        </div>
      </div>

      {/* Track List */}
      <div className="mt-4">
        {/* Header */}
        <div className="grid grid-cols-[auto_1fr_1fr_auto_auto] gap-4 px-4 py-2 text-sm text-text-muted border-b border-white/5">
          <span className="w-8 text-center">#</span>
          <span>Title</span>
          <span>Album</span>
          <span className="w-24 text-right">Added</span>
          <span className="w-16 text-right flex items-center justify-end">
            <Clock size={16} />
          </span>
        </div>

        {/* Empty state */}
        {tracks.length === 0 && (
          <div className="py-12 text-center text-text-muted">
            This playlist is empty
          </div>
        )}

        {/* Tracks */}
        <ol className="divide-y divide-white/5" role="list" aria-label="Playlist tracks">
          {tracks.map((entry, index) => {
            const track = entry.track
            const isCurrentTrack = currentTrackId === track.id
            const isPlayingTrack = isCurrentTrack && isPlaying

            return (
              <li key={`${entry.position}-${track.id}`}>
                <div
                  role="button"
                  tabIndex={0}
                  onClick={() => handlePlayTrack(entry, index)}
                  onKeyDown={(e) => handleTrackKeyDown(e, entry, index)}
                  aria-label={`Play ${track.title}${isPlayingTrack ? ' (currently playing)' : ''}`}
                  aria-current={isCurrentTrack ? 'true' : undefined}
                  className={`grid grid-cols-[auto_1fr_1fr_auto_auto] gap-4 px-4 py-3 text-sm hover:bg-background-tertiary/50 rounded-lg cursor-pointer group transition-colors focus:outline-none focus:ring-2 focus:ring-mint/50 ${
                    isCurrentTrack ? 'bg-accent-dark/10' : ''
                  }`}
                >
                  <span className={`w-8 text-center ${isCurrentTrack ? 'text-mint' : 'text-text-muted'} group-hover:hidden`}>
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
                  <span className="w-8 text-center hidden group-hover:flex items-center justify-center">
                    <Play size={14} className="text-navy" fill="currentColor" />
                  </span>
                  <div className="min-w-0">
                    <p className={`truncate ${isCurrentTrack ? 'text-mint' : 'text-text-primary'}`}>
                      {track.title}
                    </p>
                    <p className="text-text-muted text-xs truncate">
                      {track.artist?.name ?? 'Unknown Artist'}
                    </p>
                  </div>
                  <span className="text-text-muted truncate">
                    {track.album?.title ?? 'Unknown Album'}
                  </span>
                  <span className="w-24 text-right text-text-muted">
                    {formatAddedAt(entry.addedAt)}
                  </span>
                  <span className="w-16 text-right text-text-muted">
                    {track.formattedDuration || formatDurationMs(track.durationMs)}
                  </span>
                </div>
              </li>
            )
          })}
        </ol>
      </div>
    </div>
  )
}
