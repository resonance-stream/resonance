import { memo, useId, type ChangeEvent } from 'react';
import { cn } from '../../lib/utils';

interface EqualizerSliderProps {
  /** Frequency identifier for debugging/testing */
  frequency: string;
  label: string;
  gain: number;
  min?: number;
  max?: number;
  step?: number;
  onChange: (gain: number) => void;
  className?: string;
}

/**
 * Vertical slider for individual EQ band control
 * Memoized to prevent unnecessary re-renders when sibling bands change
 */
export const EqualizerSlider = memo(function EqualizerSlider({
  frequency,
  label,
  gain,
  min = -12,
  max = 12,
  step = 1,
  onChange,
  className,
}: EqualizerSliderProps): JSX.Element {
  const id = useId();

  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    onChange(Number(e.target.value));
  };

  // Format gain for display
  const displayGain = gain > 0 ? `+${gain}` : String(gain);

  return (
    <div className={cn('flex flex-col items-center gap-2', className)}>
      {/* Gain value */}
      <span className="text-xs font-medium text-text-primary min-w-[2.5rem] text-center">
        {displayGain} dB
      </span>

      {/* Vertical slider container */}
      <div className="relative h-32 flex items-center justify-center">
        {/* Zero line indicator */}
        <div className="absolute w-2 h-px bg-text-muted/50 left-1/2 -translate-x-1/2" />

        <input
          id={id}
          type="range"
          min={min}
          max={max}
          step={step}
          value={gain}
          onChange={handleChange}
          data-testid={`eq-band-${frequency}`}
          aria-label={`${label} equalizer band`}
          aria-valuenow={gain}
          aria-valuemin={min}
          aria-valuemax={max}
          className={cn(
            'appearance-none cursor-pointer',
            // Rotate to make vertical
            '-rotate-90',
            'w-32 h-2',
            // Track styling
            'bg-background-tertiary rounded-full',
            // Thumb styling
            '[&::-webkit-slider-thumb]:appearance-none',
            '[&::-webkit-slider-thumb]:w-3',
            '[&::-webkit-slider-thumb]:h-3',
            '[&::-webkit-slider-thumb]:rounded-full',
            '[&::-webkit-slider-thumb]:bg-accent',
            '[&::-webkit-slider-thumb]:shadow-sm',
            '[&::-webkit-slider-thumb]:transition-transform',
            '[&::-webkit-slider-thumb]:hover:scale-125',
            '[&::-moz-range-thumb]:w-3',
            '[&::-moz-range-thumb]:h-3',
            '[&::-moz-range-thumb]:rounded-full',
            '[&::-moz-range-thumb]:bg-accent',
            '[&::-moz-range-thumb]:border-0',
            // Focus styling
            'focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow'
          )}
        />
      </div>

      {/* Frequency label */}
      <label htmlFor={id} className="text-xs text-text-muted whitespace-nowrap">
        {label}
      </label>
    </div>
  );
});
