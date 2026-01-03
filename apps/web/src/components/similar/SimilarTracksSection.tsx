/**
 * SimilarTracksSection - Collapsible grid section for displaying similar tracks
 *
 * Shows tracks similar to a given track using AI-powered similarity matching.
 * Designed for use on Album and Artist detail pages.
 *
 * @example
 * ```tsx
 * <SimilarTracksSection
 *   trackId="uuid-of-track"
 *   title="Similar Tracks"
 *   limit={8}
 *   defaultCollapsed={false}
 * />
 * ```
 */

import { useState, useCallback } from 'react'
import { ChevronDown, ChevronRight } from 'lucide-react'
import { MediaCard } from '../media'
import { SkeletonCard } from '../ui/Skeleton'
import { useSimilarTracks } from '../../hooks/useSimilarTracks'
import { usePlayerStore } from '../../stores/playerStore'
import { mapGqlTrackToPlayerTrack } from '../../lib/mappers'
import { cn } from '../../lib/utils'
import type { SimilarTrack } from '../../types/similarity'

export interface SimilarTracksSectionProps {
  /** UUID of the track to find similar tracks for */
  trackId: string
  /** Section title (default: 'Similar Tracks') */
  title?: string
  /** Maximum number of similar tracks to display (default: 8) */
  limit?: number
  /** Whether the section starts collapsed (default: false) */
  defaultCollapsed?: boolean
  /** Additional CSS classes */
  className?: string
}

export function SimilarTracksSection({
  trackId,
  title = 'Similar Tracks',
  limit = 8,
  defaultCollapsed = false,
  className,
}: SimilarTracksSectionProps): JSX.Element | null {
  const [isCollapsed, setIsCollapsed] = useState(defaultCollapsed)

  const {
    data: similarTracks,
    isLoading,
    error,
  } = useSimilarTracks(trackId, limit, {
    // Only fetch when section is expanded or on initial load if not collapsed
    enabled: !!trackId && (!defaultCollapsed || !isCollapsed),
  })

  const setTrack = usePlayerStore((s) => s.setTrack)

  const handlePlayTrack = useCallback(
    (similarTrack: SimilarTrack) => {
      // Similar tracks include embedded track data when available
      if (similarTrack.track) {
        const playerTrack = mapGqlTrackToPlayerTrack(similarTrack.track)
        setTrack(playerTrack)
      }
    },
    [setTrack]
  )

  const toggleCollapsed = useCallback(() => {
    setIsCollapsed((prev) => !prev)
  }, [])

  // Don't render if no trackId provided
  if (!trackId) {
    return null
  }

  const ChevronIcon = isCollapsed ? ChevronRight : ChevronDown

  return (
    <section className={cn('', className)}>
      {/* Collapsible Header */}
      <button
        type="button"
        onClick={toggleCollapsed}
        className={cn(
          'flex w-full items-center gap-2 text-left',
          'group cursor-pointer',
          'mb-4'
        )}
        aria-expanded={!isCollapsed}
        aria-controls="similar-tracks-grid"
      >
        <ChevronIcon
          size={20}
          className="text-text-secondary transition-transform duration-200 group-hover:text-text-primary"
        />
        <h2 className="text-xl font-semibold text-text-primary group-hover:text-accent-primary transition-colors duration-150">
          {title}
        </h2>
        {!isLoading && similarTracks && similarTracks.length > 0 && (
          <span className="text-sm text-text-muted">
            ({similarTracks.length})
          </span>
        )}
      </button>

      {/* Content Grid */}
      {!isCollapsed && (
        <div
          id="similar-tracks-grid"
          className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6"
        >
          {isLoading ? (
            // Loading skeletons
            Array.from({ length: Math.min(limit, 6) }).map((_, i) => (
              <SkeletonCard key={i} />
            ))
          ) : error ? (
            // Error state
            <p role="alert" className="text-text-muted col-span-full">
              Failed to load similar tracks
            </p>
          ) : similarTracks && similarTracks.length > 0 ? (
            // Similar tracks grid
            similarTracks.map((similarTrack) => (
              <MediaCard
                key={similarTrack.trackId}
                title={similarTrack.title}
                subtitle={similarTrack.artistName ?? 'Unknown Artist'}
                imageUrl={similarTrack.track?.album?.coverArtUrl}
                href={
                  similarTrack.track?.albumId
                    ? `/album/${similarTrack.track.albumId}`
                    : undefined
                }
                onPlay={
                  similarTrack.track ? () => handlePlayTrack(similarTrack) : undefined
                }
              />
            ))
          ) : (
            // Empty state
            <p className="text-text-muted col-span-full">
              No similar tracks found
            </p>
          )}
        </div>
      )}
    </section>
  )
}
