import { forwardRef, type InputHTMLAttributes } from 'react';
import { cn } from '../../lib/utils';

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  icon?: React.ReactNode;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, icon, ...props }, ref) => {
    if (icon) {
      return (
        <div className="relative">
          <div className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted">
            {icon}
          </div>
          <input
            className={cn(
              'w-full px-3 py-2 pl-10 rounded-lg',
              'bg-background-secondary',
              'border border-background-tertiary',
              'text-text-primary placeholder:text-text-muted',
              'focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow',
              'focus:border-accent-light',
              'transition-all duration-150',
              className
            )}
            ref={ref}
            {...props}
          />
        </div>
      );
    }

    return (
      <input
        className={cn(
          'w-full px-3 py-2 rounded-lg',
          'bg-background-secondary',
          'border border-background-tertiary',
          'text-text-primary placeholder:text-text-muted',
          'focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow',
          'focus:border-accent-light',
          'transition-all duration-150',
          className
        )}
        ref={ref}
        {...props}
      />
    );
  }
);

Input.displayName = 'Input';
