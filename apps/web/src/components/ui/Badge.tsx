import { type HTMLAttributes } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../../lib/utils';

const badgeVariants = cva(
  'inline-flex items-center font-semibold uppercase tracking-wider border',
  {
    variants: {
      variant: {
        default: 'bg-accent-dark/50 text-text-secondary border-white/5',
        quality: 'bg-accent-dark/50 text-text-secondary border-white/5',
        'quality-hires':
          'bg-mint-soft text-mint border-mint/30',
        success: 'bg-mint-soft text-mint border-mint/30',
        warning: 'bg-warning/20 text-warning-text border-warning/30',
        error: 'bg-error/20 text-error-text border-error/30',
        info: 'bg-navy-soft text-navy border-navy/30',
      },
      size: {
        sm: 'px-1.5 py-0.5 text-[10px] rounded',
        md: 'px-2 py-1 text-xs rounded',
        lg: 'px-3 py-1.5 text-sm rounded-lg',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'sm',
    },
  }
);

export interface BadgeProps
  extends HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof badgeVariants> {}

export function Badge({
  className,
  variant,
  size,
  ...props
}: BadgeProps): JSX.Element {
  return (
    <span
      className={cn(badgeVariants({ variant, size, className }))}
      {...props}
    />
  );
}

// Convenience component for quality badges
export interface QualityBadgeProps {
  format: 'flac' | 'hires' | 'lossless' | 'mp3' | 'aac';
  className?: string;
}

const QUALITY_BADGE_LABELS: Record<QualityBadgeProps['format'], string> = {
  flac: 'FLAC',
  hires: 'HI-RES',
  lossless: 'LOSSLESS',
  mp3: 'MP3',
  aac: 'AAC',
};

export function QualityBadge({
  format,
  className,
}: QualityBadgeProps): JSX.Element {
  const isHires = format === 'hires';

  return (
    <Badge
      variant={isHires ? 'quality-hires' : 'quality'}
      size="sm"
      className={className}
    >
      {QUALITY_BADGE_LABELS[format]}
    </Badge>
  );
}
