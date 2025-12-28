import { Link } from 'react-router-dom';
import { Play } from 'lucide-react';
import { cn } from '../../lib/utils';
import { Button } from '../ui/Button';

export interface MediaCardProps {
  title: string;
  subtitle?: string;
  imageUrl?: string;
  href?: string;
  onPlay?: () => void;
  className?: string;
}

export function MediaCard({
  title,
  subtitle,
  imageUrl,
  href,
  onPlay,
  className,
}: MediaCardProps): JSX.Element {
  const content = (
    <>
      {/* Album Art */}
      <div className="relative aspect-square rounded-lg overflow-hidden bg-background-tertiary mb-3 group-hover:shadow-[0_0_20px_rgba(37,99,235,0.25)] transition-shadow duration-150">
        {imageUrl ? (
          <img
            src={imageUrl}
            alt={title}
            className="w-full h-full object-cover"
            loading="lazy"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center">
            <svg
              className="w-12 h-12 text-text-muted opacity-50"
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
        )}

        {/* Play button overlay */}
        {onPlay && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-150">
            <Button
              variant="accent"
              size="icon"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onPlay();
              }}
              className="rounded-full w-12 h-12"
              aria-label={`Play ${title}`}
            >
              <Play size={20} className="ml-0.5" fill="currentColor" />
            </Button>
          </div>
        )}
      </div>

      {/* Text content */}
      <div className="px-1">
        <h3 className="font-medium text-text-primary truncate text-sm">
          {title}
        </h3>
        {subtitle && (
          <p className="text-text-secondary text-sm truncate mt-0.5">
            {subtitle}
          </p>
        )}
      </div>
    </>
  );

  const wrapperClasses = cn(
    'group block p-3 rounded-lg',
    'bg-background-secondary/50',
    'border border-transparent',
    'hover:bg-background-tertiary hover:border-accent-dark',
    'transition-all duration-150',
    className
  );

  if (href) {
    return (
      <Link to={href} className={wrapperClasses}>
        {content}
      </Link>
    );
  }

  return <div className={wrapperClasses}>{content}</div>;
}
