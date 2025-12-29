import { forwardRef, type InputHTMLAttributes } from 'react';
import { cn } from '../../lib/utils';

export interface SwitchProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  checked?: boolean;
  onCheckedChange?: (checked: boolean) => void;
}

export const Switch = forwardRef<HTMLInputElement, SwitchProps>(
  ({ className, checked, onCheckedChange, onChange, ...props }, ref) => {
    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      onCheckedChange?.(e.target.checked);
      onChange?.(e);
    };

    return (
      <label className="relative inline-flex items-center cursor-pointer">
        <input
          type="checkbox"
          role="switch"
          aria-checked={!!checked}
          className="sr-only peer"
          checked={!!checked}
          onChange={handleChange}
          ref={ref}
          {...props}
        />
        <div
          className={cn(
            // Base styling
            'w-11 h-6 rounded-full transition-colors duration-200',
            // Unchecked state
            'bg-background-tertiary',
            // Checked state
            'peer-checked:bg-accent',
            // Focus ring
            'peer-focus-visible:ring-2 peer-focus-visible:ring-accent-glow',
            // Thumb
            'after:content-[""]',
            'after:absolute after:top-[2px] after:left-[2px]',
            'after:bg-text-primary',
            'after:rounded-full',
            'after:h-5 after:w-5',
            'after:transition-transform after:duration-200',
            'peer-checked:after:translate-x-5',
            'peer-checked:after:bg-background-primary',
            className
          )}
        />
      </label>
    );
  }
);

Switch.displayName = 'Switch';
