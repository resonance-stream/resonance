import { useState, useRef, useEffect } from 'react';
import { MoreHorizontal, Sparkles, ListPlus, Share2, Radio } from 'lucide-react';
import { useVisualizerStore } from '../../stores/visualizerStore';
import { Button } from '../ui/Button';
import { cn } from '../../lib/utils';

/**
 * Three-dot menu for fullscreen player
 * Contains visualizer toggle and other track options
 */
export function FullscreenMenu(): JSX.Element {
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const showVisualizer = useVisualizerStore((s) => s.showVisualizer);
  const toggleVisualizer = useVisualizerStore((s) => s.toggleVisualizer);

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      // Safely check if target is a Node before using contains()
      if (!(e.target instanceof Node)) return;

      if (
        menuRef.current &&
        buttonRef.current &&
        !menuRef.current.contains(e.target) &&
        !buttonRef.current.contains(e.target)
      ) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen]);

  // Close menu on escape
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        e.stopPropagation();
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('keydown', handleKeyDown);
    }

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  return (
    <div className="relative">
      <Button
        ref={buttonRef}
        variant="icon"
        size="icon"
        onClick={() => setIsOpen(!isOpen)}
        aria-label="More options"
        aria-expanded={isOpen}
        aria-haspopup="menu"
        aria-controls={isOpen ? 'fullscreen-menu' : undefined}
        className="p-2"
      >
        <MoreHorizontal size={24} />
      </Button>

      {/* Dropdown Menu */}
      {isOpen && (
        <div
          id="fullscreen-menu"
          ref={menuRef}
          className={cn(
            'absolute right-0 top-full mt-2 z-50',
            'min-w-[200px] py-2',
            'bg-background-secondary rounded-lg',
            'border border-white/10',
            'shadow-xl',
            'animate-fade-in'
          )}
          role="menu"
          aria-orientation="vertical"
        >
          {/* Visualizer Toggle */}
          <button
            onClick={() => {
              toggleVisualizer();
              setIsOpen(false);
            }}
            className={cn(
              'w-full flex items-center gap-3 px-4 py-2.5',
              'text-left text-sm',
              'hover:bg-white/5 transition-colors',
              showVisualizer && 'text-accent'
            )}
            role="menuitem"
          >
            <Sparkles size={18} />
            <span>Visualizer</span>
            <span
              className={cn(
                'ml-auto text-xs px-2 py-0.5 rounded-full',
                showVisualizer
                  ? 'bg-accent/20 text-accent'
                  : 'bg-white/10 text-text-muted'
              )}
            >
              {showVisualizer ? 'On' : 'Off'}
            </span>
          </button>

          <div className="h-px bg-white/10 my-1" />

          {/* Add to Playlist */}
          <button
            onClick={() => setIsOpen(false)}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-left text-sm hover:bg-white/5 transition-colors"
            role="menuitem"
          >
            <ListPlus size={18} />
            <span>Add to Playlist</span>
          </button>

          {/* Go to Radio */}
          <button
            onClick={() => setIsOpen(false)}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-left text-sm hover:bg-white/5 transition-colors"
            role="menuitem"
          >
            <Radio size={18} />
            <span>Go to Radio</span>
          </button>

          {/* Share */}
          <button
            onClick={() => setIsOpen(false)}
            className="w-full flex items-center gap-3 px-4 py-2.5 text-left text-sm hover:bg-white/5 transition-colors"
            role="menuitem"
          >
            <Share2 size={18} />
            <span>Share</span>
          </button>
        </div>
      )}
    </div>
  );
}
