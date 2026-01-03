import { Disc3 } from 'lucide-react'
import { memo, useCallback, useState, type MouseEvent } from 'react'
import { cn } from '../../lib/utils'
import { Button } from '../ui/Button'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '../ui/Dialog'
import { SimilarTracksPanel } from './SimilarTracksPanel'

export interface FindSimilarButtonProps {
  /** UUID of the track to find similar tracks for */
  trackId: string
  /** Title of the track (displayed in dialog header) */
  trackTitle: string
  /** Additional CSS classes */
  className?: string
}

/**
 * Icon button that opens a dialog showing similar tracks
 *
 * Opens a modal with SimilarTracksPanel when clicked. The modal closes
 * automatically when a track is played from the similar tracks list.
 */
export const FindSimilarButton = memo(function FindSimilarButton({
  trackId,
  trackTitle,
  className,
}: FindSimilarButtonProps): JSX.Element {
  const [isOpen, setIsOpen] = useState(false)

  const handleClick = useCallback((e: MouseEvent<HTMLButtonElement>) => {
    // Stop propagation to prevent parent click handlers (e.g., row selection)
    e.stopPropagation()
    setIsOpen(true)
  }, [])

  const handleTrackPlay = useCallback(() => {
    // Close the dialog when a track is played
    setIsOpen(false)
  }, [])

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <Button
        variant="icon"
        size="icon"
        className={cn(
          'text-text-muted hover:text-text-primary',
          'transition-colors',
          className
        )}
        onClick={handleClick}
        aria-label={`Find tracks similar to ${trackTitle}`}
        title="Find similar tracks"
      >
        <Disc3 className="h-4 w-4" />
      </Button>

      <DialogContent className="max-w-lg max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Similar Tracks</DialogTitle>
          <DialogDescription>
            Tracks similar to &ldquo;{trackTitle}&rdquo;
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto mt-4 -mx-2 px-2">
          <SimilarTracksPanel
            trackId={trackId}
            limit={15}
            onTrackPlay={handleTrackPlay}
          />
        </div>
      </DialogContent>
    </Dialog>
  )
})
