/**
 * Skeleton loading placeholders
 *
 * Provides visual feedback during data loading with animated pulse effect.
 */

import { forwardRef, type HTMLAttributes } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '../../lib/utils'

const skeletonVariants = cva('animate-pulse motion-reduce:animate-none bg-white/5 rounded', {
  variants: {
    variant: {
      default: 'bg-white/5',
      darker: 'bg-white/3',
      lighter: 'bg-white/10',
    },
    rounded: {
      none: 'rounded-none',
      sm: 'rounded-sm',
      md: 'rounded',
      lg: 'rounded-lg',
      xl: 'rounded-xl',
      full: 'rounded-full',
    },
  },
  defaultVariants: {
    variant: 'default',
    rounded: 'md',
  },
})

export interface SkeletonProps
  extends HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof skeletonVariants> {}

/**
 * Basic skeleton placeholder with pulse animation
 */
export const Skeleton = forwardRef<HTMLDivElement, SkeletonProps>(
  ({ className, variant, rounded, ...props }, ref) => {
    return (
      <div
        ref={ref}
        role="status"
        aria-label="Loading"
        className={cn(skeletonVariants({ variant, rounded, className }))}
        {...props}
      />
    )
  }
)

Skeleton.displayName = 'Skeleton'

/**
 * Skeleton for text lines
 */
export interface SkeletonTextProps extends SkeletonProps {
  lines?: number
}

export const SkeletonText = forwardRef<HTMLDivElement, SkeletonTextProps>(
  ({ className, lines = 1, ...props }, ref) => {
    return (
      <div
        ref={ref}
        role="status"
        aria-label="Loading text"
        className={cn('space-y-2', className)}
        {...props}
      >
        {Array.from({ length: lines }).map((_, i) => (
          <Skeleton
            key={i}
            role="presentation"
            aria-label={undefined}
            className={cn('h-4', i === lines - 1 && lines > 1 ? 'w-3/4' : 'w-full')}
          />
        ))}
      </div>
    )
  }
)

SkeletonText.displayName = 'SkeletonText'

/**
 * Skeleton for media cards (album, artist, playlist covers)
 */
export interface SkeletonCardProps extends SkeletonProps {
  aspectRatio?: 'square' | 'video' | 'portrait'
  showText?: boolean
}

export const SkeletonCard = forwardRef<HTMLDivElement, SkeletonCardProps>(
  ({ className, aspectRatio = 'square', showText = true, ...props }, ref) => {
    const aspectClass = {
      square: 'aspect-square',
      video: 'aspect-video',
      portrait: 'aspect-[3/4]',
    }[aspectRatio]

    return (
      <div ref={ref} role="status" aria-label="Loading card" className={cn('space-y-3', className)}>
        <Skeleton
          role="presentation"
          aria-label={undefined}
          className={cn('w-full', aspectClass)}
          rounded="lg"
          {...props}
        />
        {showText && (
          <div className="space-y-2">
            <Skeleton role="presentation" aria-label={undefined} className="h-4 w-3/4" />
            <Skeleton role="presentation" aria-label={undefined} className="h-3 w-1/2" />
          </div>
        )}
      </div>
    )
  }
)

SkeletonCard.displayName = 'SkeletonCard'

/**
 * Skeleton for track list rows
 */
export const SkeletonTrackRow = forwardRef<HTMLDivElement, SkeletonProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        role="status"
        aria-label="Loading track"
        className={cn('flex items-center gap-4 py-2', className)}
        {...props}
      >
        <Skeleton role="presentation" aria-label={undefined} className="h-4 w-6" />
        <Skeleton role="presentation" aria-label={undefined} className="h-10 w-10 flex-shrink-0" rounded="sm" />
        <div className="flex-1 space-y-1">
          <Skeleton role="presentation" aria-label={undefined} className="h-4 w-48" />
          <Skeleton role="presentation" aria-label={undefined} className="h-3 w-32" />
        </div>
        <Skeleton role="presentation" aria-label={undefined} className="h-4 w-12" />
      </div>
    )
  }
)

SkeletonTrackRow.displayName = 'SkeletonTrackRow'

/**
 * Skeleton for album/playlist header
 */
export const SkeletonHeader = forwardRef<HTMLDivElement, SkeletonProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        role="status"
        aria-label="Loading content"
        className={cn('flex gap-6', className)}
        {...props}
      >
        <Skeleton role="presentation" aria-label={undefined} className="h-48 w-48 flex-shrink-0" rounded="lg" />
        <div className="flex flex-col justify-end space-y-4">
          <Skeleton role="presentation" aria-label={undefined} className="h-3 w-16" />
          <Skeleton role="presentation" aria-label={undefined} className="h-10 w-64" />
          <div className="flex items-center gap-2">
            <Skeleton role="presentation" aria-label={undefined} className="h-6 w-6" rounded="full" />
            <Skeleton role="presentation" aria-label={undefined} className="h-4 w-32" />
            <Skeleton role="presentation" aria-label={undefined} className="h-4 w-24" />
          </div>
        </div>
      </div>
    )
  }
)

SkeletonHeader.displayName = 'SkeletonHeader'
