import { forwardRef, useId, type InputHTMLAttributes } from 'react';
import { cn } from '../../lib/utils';

export interface SliderProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: string;
  showValue?: boolean;
  valueFormatter?: (value: number) => string;
}

export const Slider = forwardRef<HTMLInputElement, SliderProps>(
  ({ className, label, showValue = true, valueFormatter, value, id, ...props }, ref) => {
    const generatedId = useId();
    const inputId = id || generatedId;
    const numValue = typeof value === 'number' ? value : Number(value) || 0;
    const displayValue = valueFormatter ? valueFormatter(numValue) : String(numValue);

    return (
      <div className="flex items-center gap-3 w-full">
        {label && (
          <label htmlFor={inputId} className="text-sm text-text-secondary whitespace-nowrap">
            {label}
          </label>
        )}
        <input
          id={inputId}
          type="range"
          className={cn(
            'w-full h-2 rounded-full appearance-none cursor-pointer',
            'bg-background-tertiary',
            // Thumb styling
            '[&::-webkit-slider-thumb]:appearance-none',
            '[&::-webkit-slider-thumb]:w-4',
            '[&::-webkit-slider-thumb]:h-4',
            '[&::-webkit-slider-thumb]:rounded-full',
            '[&::-webkit-slider-thumb]:bg-accent',
            '[&::-webkit-slider-thumb]:shadow-md',
            '[&::-webkit-slider-thumb]:transition-transform',
            '[&::-webkit-slider-thumb]:hover:scale-110',
            '[&::-moz-range-thumb]:w-4',
            '[&::-moz-range-thumb]:h-4',
            '[&::-moz-range-thumb]:rounded-full',
            '[&::-moz-range-thumb]:bg-accent',
            '[&::-moz-range-thumb]:border-0',
            // Focus styling
            'focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow',
            className
          )}
          ref={ref}
          value={value}
          {...props}
        />
        {showValue && (
          <span className="text-sm text-text-primary font-medium min-w-[3rem] text-right">
            {displayValue}
          </span>
        )}
      </div>
    );
  }
);

Slider.displayName = 'Slider';
