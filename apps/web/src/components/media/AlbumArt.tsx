import { Play } from 'lucide-react';
import { cn } from '../../lib/utils';
import { Button } from '../ui/Button';

export interface AlbumArtProps {
  src?: string;
  alt: string;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  showPlayButton?: boolean;
  onPlay?: () => void;
  className?: string;
}

const sizeClasses = {
  sm: 'w-12 h-12',
  md: 'w-24 h-24',
  lg: 'w-40 h-40',
  xl: 'w-60 h-60',
};

const playButtonSizes = {
  sm: 'w-6 h-6',
  md: 'w-10 h-10',
  lg: 'w-12 h-12',
  xl: 'w-14 h-14',
};

const playIconSizes = {
  sm: 12,
  md: 18,
  lg: 24,
  xl: 28,
};

export function AlbumArt({
  src,
  alt,
  size = 'md',
  showPlayButton = true,
  onPlay,
  className,
}: AlbumArtProps): JSX.Element {
  return (
    <div
      className={cn(
        'relative aspect-square rounded-lg overflow-hidden bg-background-tertiary group',
        sizeClasses[size],
        className
      )}
    >
      {src ? (
        <img
          src={src}
          alt={alt}
          className="w-full h-full object-cover"
          loading="lazy"
        />
      ) : (
        <div className="w-full h-full flex items-center justify-center">
          <div className="text-text-muted">
            <svg
              className={cn('opacity-50', sizeClasses[size])}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
            >
              <path d="M9 18V5l12-2v13" />
              <circle cx="6" cy="18" r="3" />
              <circle cx="18" cy="16" r="3" />
            </svg>
          </div>
        </div>
      )}

      {/* Hover overlay with play button */}
      {showPlayButton && onPlay && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-150">
          <Button
            variant="accent"
            size="icon"
            onClick={(e) => {
              e.stopPropagation();
              onPlay();
            }}
            className={cn(
              'rounded-full',
              playButtonSizes[size]
            )}
            aria-label={`Play ${alt}`}
          >
            <Play size={playIconSizes[size]} className="ml-0.5" fill="currentColor" />
          </Button>
        </div>
      )}

      {/* Hover glow effect */}
      <div className="absolute inset-0 pointer-events-none opacity-0 group-hover:opacity-100 transition-opacity duration-150 shadow-[inset_0_0_0_1px_rgba(37,99,235,0.3)]" />
    </div>
  );
}
