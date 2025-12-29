import { useCallback } from 'react'
import { useParams } from 'react-router-dom'
import { Play, Heart, MoreHorizontal, Clock, AlertCircle } from 'lucide-react'
import { Button } from '../components/ui/Button'
import { AlbumArt } from '../components/media/AlbumArt'
import { QualityBadge, type QualityBadgeProps } from '../components/ui/Badge'
import { SkeletonHeader, SkeletonTrackRow } from '../components/ui/Skeleton'
import { useAlbum } from '../hooks/useLibrary'
import { usePlayerStore } from '../stores/playerStore'
import { mapAlbumToPlayerTracks, formatDurationMs } from '../lib/mappers'
import type { GqlTrack, AudioFormat } from '../types/library'

/**
 * Map GraphQL AudioFormat to QualityBadge format
 * Handles all format variants and prioritizes hi-res display
 */
function mapAudioFormatToBadgeFormat(
  format: AudioFormat,
  isHires: boolean,
  isLossless: boolean
): QualityBadgeProps['format'] {
  if (isHires) return 'hires'
  if (isLossless) return 'lossless'

  const lowerFormat = format.toLowerCase()
  if (lowerFormat === 'flac' || lowerFormat === 'alac' || lowerFormat === 'wav') return 'flac'
  if (lowerFormat === 'mp3') return 'mp3'
  if (lowerFormat === 'aac' || lowerFormat === 'opus' || lowerFormat === 'ogg') return 'aac'
  return 'mp3' // fallback for unknown formats
}

export default function Album() {
  const { id } = useParams<{ id: string }>()
  const { data: album, isLoading, error } = useAlbum(id)

  // Use granular selectors to minimize re-renders
  const setQueue = usePlayerStore((s) => s.setQueue)
  const togglePlay = usePlayerStore((s) => s.togglePlay)
  const currentTrackId = usePlayerStore((s) => s.currentTrack?.id)
  const isPlaying = usePlayerStore((s) => s.isPlaying)

  // Handle play all button
  const handlePlayAll = useCallback(() => {
    if (!album?.tracks?.length) return
    const tracks = mapAlbumToPlayerTracks(album)
    setQueue(tracks, 0)
  }, [album, setQueue])

  // Handle individual track play
  const handlePlayTrack = useCallback((track: GqlTrack, index: number) => {
    if (!album?.tracks) return

    // If clicking the currently playing track, toggle play/pause
    if (currentTrackId === track.id) {
      togglePlay()
      return
    }

    // Set entire album as queue starting from clicked track
    const tracks = mapAlbumToPlayerTracks(album)
    setQueue(tracks, index)
  }, [album, currentTrackId, togglePlay, setQueue])

  // Handle keyboard navigation on track rows
  const handleTrackKeyDown = useCallback((
    e: React.KeyboardEvent,
    track: GqlTrack,
    index: number
  ) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      handlePlayTrack(track, index)
    }
  }, [handlePlayTrack])

  // Handle missing album ID
  if (!id) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center p-6 animate-fade-in">
        <AlertCircle size={48} className="text-text-muted mb-4" />
        <h2 className="text-xl font-semibold text-text-primary mb-2">
          Invalid Album URL
        </h2>
        <p className="text-text-secondary">
          No album ID was provided in the URL.
        </p>
      </div>
    )
  }

  // Loading state
  if (isLoading) {
    return (
      <div className="flex flex-1 flex-col p-6 animate-fade-in">
        <SkeletonHeader className="mb-8" />
        <div className="mt-4">
          <div className="grid grid-cols-[auto_1fr_auto_auto] gap-4 px-4 py-2 text-sm text-text-muted border-b border-white/5">
            <span className="w-8 text-center">#</span>
            <span>Title</span>
            <span className="w-20 text-center">Quality</span>
            <span className="w-16 text-right flex items-center justify-end">
              <Clock size={16} />
            </span>
          </div>
          <div className="divide-y divide-white/5">
            {Array.from({ length: 8 }).map((_, i) => (
              <SkeletonTrackRow key={i} />
            ))}
          </div>
        </div>
      </div>
    )
  }

  // Error state
  if (error || !album) {
    return (
      <div role="alert" className="flex flex-1 flex-col items-center justify-center p-6 animate-fade-in">
        <AlertCircle size={48} className="text-text-muted mb-4" aria-hidden="true" />
        <h2 className="text-xl font-semibold text-text-primary mb-2">
          Album not found
        </h2>
        <p className="text-text-secondary">
          {error?.message || 'The album you requested could not be loaded.'}
        </p>
      </div>
    )
  }

  // Get album artist name
  const artistName = album.artist?.name ?? 'Unknown Artist'
  const tracks = album.tracks ?? []

  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Album Header */}
      <div className="flex flex-col md:flex-row gap-8 mb-8">
        <AlbumArt
          src={album.coverArtUrl}
          alt={album.title}
          size="xl"
          showPlayButton={false}
          className="flex-shrink-0 self-center md:self-start"
        />

        <div className="flex flex-col justify-end">
          <span className="text-overline text-text-muted uppercase tracking-wider">
            {album.albumType}
          </span>
          <h1 className="font-display text-display-xl text-text-primary mt-2">
            {album.title}
          </h1>
          <p className="text-lg text-text-secondary mt-2">
            {artistName}
          </p>
          <p className="text-sm text-text-muted mt-1">
            {album.releaseYear ?? 'Unknown year'} - {album.totalTracks ?? tracks.length} songs
            {album.formattedDuration && `, ${album.formattedDuration}`}
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
            <Button variant="ghost" size="icon" aria-label="Like album" className="hover:text-mint">
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
        <div className="grid grid-cols-[auto_1fr_auto_auto] gap-4 px-4 py-2 text-sm text-text-muted border-b border-white/5">
          <span className="w-8 text-center">#</span>
          <span>Title</span>
          <span className="w-20 text-center">Quality</span>
          <span className="w-16 text-right flex items-center justify-end">
            <Clock size={16} />
          </span>
        </div>

        {/* Empty state */}
        {tracks.length === 0 && (
          <div className="py-12 text-center text-text-muted">
            No tracks available
          </div>
        )}

        {/* Tracks */}
        <ol className="divide-y divide-white/5" role="list" aria-label="Album tracks">
          {tracks.map((track, index) => {
            const isCurrentTrack = currentTrackId === track.id
            const isPlayingTrack = isCurrentTrack && isPlaying

            return (
              <li key={track.id}>
                <div
                  role="button"
                  tabIndex={0}
                  onClick={() => handlePlayTrack(track, index)}
                  onKeyDown={(e) => handleTrackKeyDown(e, track, index)}
                  aria-label={`Play ${track.title}${isPlayingTrack ? ' (currently playing)' : ''}`}
                  aria-current={isCurrentTrack ? 'true' : undefined}
                  className={`grid grid-cols-[auto_1fr_auto_auto] gap-4 px-4 py-3 text-sm hover:bg-background-tertiary/50 rounded-lg cursor-pointer group transition-colors focus:outline-none focus:ring-2 focus:ring-mint/50 ${
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
                      track.trackNumber ?? index + 1
                    )}
                  </span>
                  <span className="w-8 text-center hidden group-hover:flex items-center justify-center">
                    <Play size={14} className="text-navy" fill="currentColor" />
                  </span>
                  <div className="flex flex-col min-w-0">
                    <span className={`truncate ${isCurrentTrack ? 'text-mint' : 'text-text-primary'}`}>
                      {track.title}
                    </span>
                    {track.artist && track.artist.name !== artistName && (
                      <span className="text-xs text-text-muted truncate">
                        {track.artist.name}
                      </span>
                    )}
                  </div>
                  <span className="w-20 flex items-center justify-center">
                    <QualityBadge
                      format={mapAudioFormatToBadgeFormat(track.fileFormat, track.isHires, track.isLossless)}
                    />
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
