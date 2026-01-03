import { memo, useCallback, useMemo } from 'react'
import { AlertTriangle, Music } from 'lucide-react'
import { useSimilarTracks, type ScoredTrack } from '../../hooks/useSimilarTracks'
import { usePlayerStore } from '../../stores/playerStore'
import { mapScoredTrackToPlayerTrack } from '../../lib/mappers'
import { AlbumArt } from '../media/AlbumArt'
import { Skeleton } from '../ui/Skeleton'
import { cn } from '../../lib/utils'

export interface SimilarTracksPanelProps {
  /** UUID of the track to find similar tracks for */
  trackId: string
  /** Maximum number of similar tracks to display */
  limit?: number
  /** Callback when a track is played */
  onTrackPlay?: (track: ScoredTrack) => void
  /** Additional CSS classes */
  className?: string
}

/**
 * Panel component displaying similar tracks in a list format
 *
 * Designed for use in modal dialogs, showing tracks similar to a given track
 * with album art thumbnails, track info, and similarity scores.
 */
export const SimilarTracksPanel = memo(function SimilarTracksPanel({
  trackId,
  limit = 10,
  onTrackPlay,
  className,
}: SimilarTracksPanelProps): JSX.Element {
  const { data: similarTracks, isLoading, error } = useSimilarTracks(trackId, limit)
  const setQueue = usePlayerStore((s) => s.setQueue)

  // Pre-compute player tracks array for efficient queue updates
  const playerTracks = useMemo(
    () => similarTracks?.map(mapScoredTrackToPlayerTrack) ?? [],
    [similarTracks]
  )

  const handlePlayTrack = useCallback(
    (similarTrack: ScoredTrack, index: number) => {
      if (playerTracks.length === 0) return

      // Set queue starting from the clicked track
      setQueue(playerTracks, index)

      // Trigger callback if provided
      onTrackPlay?.(similarTrack)
    },
    [playerTracks, setQueue, onTrackPlay]
  )

  // Loading state with skeleton
  if (isLoading) {
    return (
      <div className={cn('space-y-2', className)} role="status" aria-label="Loading similar tracks">
        {Array.from({ length: Math.min(limit, 5) }).map((_, i) => (
          <SimilarTrackSkeleton key={i} />
        ))}
      </div>
    )
  }

  // Error state
  if (error) {
    return (
      <div
        className={cn(
          'flex flex-col items-center justify-center py-8 text-center',
          className
        )}
        role="alert"
      >
        <AlertTriangle
          className="w-12 h-12 text-text-muted mb-3"
          strokeWidth={1.5}
        />
        <p className="text-text-muted text-sm">Could not load similar tracks</p>
        <p className="text-text-muted/60 text-xs mt-1">
          {error instanceof Error ? error.message : 'Please try again later'}
        </p>
      </div>
    )
  }

  // Empty state
  if (!similarTracks || similarTracks.length === 0) {
    return (
      <div
        className={cn(
          'flex flex-col items-center justify-center py-8 text-center',
          className
        )}
      >
        <Music
          className="w-12 h-12 text-text-muted mb-3"
          strokeWidth={1.5}
        />
        <p className="text-text-muted text-sm">No similar tracks found</p>
        <p className="text-text-muted/60 text-xs mt-1">
          Try a different track or check back later
        </p>
      </div>
    )
  }

  return (
    <div
      className={cn('space-y-1', className)}
      role="list"
      aria-label="Similar tracks"
    >
      {similarTracks.map((track, index) => (
        <SimilarTrackRow
          key={track.trackId}
          track={track}
          onPlay={() => handlePlayTrack(track, index)}
        />
      ))}
    </div>
  )
})

// ============================================================================
// Sub-components
// ============================================================================

interface SimilarTrackRowProps {
  track: ScoredTrack
  onPlay: () => void
}

/**
 * Individual track row in the similar tracks list
 */
const SimilarTrackRow = memo(function SimilarTrackRow({
  track,
  onPlay,
}: SimilarTrackRowProps): JSX.Element {
  // Format similarity score as percentage
  const similarityPercent = Math.round(track.score * 100)

  // Get similarity type from track (SimilarTrack has it, ScoredTrack doesn't)
  // Default to 'Combined' for basic similarity results
  const similarityType = 'similarityType' in track ? (track as { similarityType: string }).similarityType : 'Combined'

  // Get similarity type badge color
  const getBadgeColor = (type: string): string => {
    switch (type) {
      case 'Semantic':
        return 'bg-purple-500/20 text-purple-300'
      case 'Acoustic':
        return 'bg-blue-500/20 text-blue-300'
      case 'Categorical':
        return 'bg-green-500/20 text-green-300'
      case 'Combined':
      default:
        return 'bg-accent/20 text-accent'
    }
  }

  return (
    <div
      role="listitem"
      className={cn(
        'flex items-center gap-3 p-2 rounded-lg',
        'hover:bg-background-tertiary transition-colors cursor-pointer',
        'focus-within:ring-2 focus-within:ring-accent-glow focus-within:ring-offset-1 focus-within:ring-offset-background-primary'
      )}
      onClick={onPlay}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onPlay()
        }
      }}
      tabIndex={0}
      aria-label={`${track.title} by ${track.artistName ?? 'Unknown Artist'}. ${similarityPercent}% similar. Press Enter to play.`}
    >
      {/* Album Art Thumbnail */}
      <AlbumArt
        src={track.track?.album?.coverArtUrl}
        alt={track.albumTitle ?? track.title}
        size="sm"
        showPlayButton={false}
        className="flex-shrink-0"
      />

      {/* Track Info */}
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-text-primary truncate">
          {track.title}
        </p>
        <p className="text-xs text-text-muted truncate">
          {track.artistName ?? 'Unknown Artist'}
          {track.albumTitle && (
            <span className="text-text-muted/60"> &middot; {track.albumTitle}</span>
          )}
        </p>
      </div>

      {/* Similarity Badge */}
      <div className="flex items-center gap-2 flex-shrink-0">
        <span
          className={cn(
            'px-2 py-0.5 rounded-full text-xs font-medium',
            getBadgeColor(similarityType)
          )}
          title={`${similarityType} similarity`}
        >
          {similarityPercent}%
        </span>
      </div>
    </div>
  )
})

/**
 * Skeleton placeholder for loading state
 */
function SimilarTrackSkeleton(): JSX.Element {
  return (
    <div className="flex items-center gap-3 p-2" role="presentation" aria-hidden="true">
      <Skeleton className="w-12 h-12 flex-shrink-0" rounded="lg" />
      <div className="flex-1 space-y-1.5">
        <Skeleton className="h-4 w-3/4" />
        <Skeleton className="h-3 w-1/2" />
      </div>
      <Skeleton className="h-5 w-10" rounded="full" />
    </div>
  )
}
