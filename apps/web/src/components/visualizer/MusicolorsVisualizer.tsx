import { useEffect, useRef, useCallback, useState, useMemo } from 'react';
import { Visualizer } from 'resonance-visualizer';
import { useAudio } from '../../hooks/useAudio';

interface MusicolorsVisualizerProps {
  className?: string;
}

/**
 * Debounce utility with cancel function to prevent memory leaks
 */
interface DebouncedFn<T extends (...args: unknown[]) => void> {
  (...args: Parameters<T>): void;
  cancel: () => void;
}

function debounce<T extends (...args: unknown[]) => void>(
  fn: T,
  delay: number
): DebouncedFn<T> {
  let timeoutId: ReturnType<typeof setTimeout> | undefined;

  const debouncedFn = (...args: Parameters<T>): void => {
    if (timeoutId) clearTimeout(timeoutId);
    timeoutId = setTimeout(() => fn(...args), delay);
  };

  debouncedFn.cancel = (): void => {
    if (timeoutId) {
      clearTimeout(timeoutId);
      timeoutId = undefined;
    }
  };

  return debouncedFn;
}

/**
 * Wrapper component for the resonance-visualizer (musicolors) library.
 * Renders a 3D sphere visualization that reacts to audio in real-time.
 */
export function MusicolorsVisualizer({ className }: MusicolorsVisualizerProps): JSX.Element {
  const containerRef = useRef<HTMLDivElement>(null);
  const visualizerRef = useRef<Visualizer | null>(null);
  const { getAnalyser, getAudioContext, isInitialized } = useAudio();

  // Reactive reduced motion preference
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(() =>
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );

  // Listen for reduced motion preference changes
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    const handleChange = (e: MediaQueryListEvent) => {
      setPrefersReducedMotion(e.matches);
      // If reduced motion is enabled, destroy the visualizer
      if (e.matches && visualizerRef.current) {
        visualizerRef.current.destroy();
        visualizerRef.current = null;
      }
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  // Handle resize with debouncing
  const handleResize = useCallback(() => {
    if (visualizerRef.current && containerRef.current) {
      visualizerRef.current.resize(
        containerRef.current.clientWidth,
        containerRef.current.clientHeight
      );
    }
  }, []);

  const debouncedResize = useMemo(
    () => debounce(handleResize, 100),
    [handleResize]
  );

  useEffect(() => {
    if (!containerRef.current || !isInitialized) return;

    // Don't start visualizer if user prefers reduced motion
    if (prefersReducedMotion) {
      return;
    }

    const analyser = getAnalyser();
    const audioContext = getAudioContext();

    if (!analyser || !audioContext) {
      return;
    }

    // Create and initialize the visualizer
    const viz = new Visualizer(containerRef.current);
    viz.initWithAnalyser(analyser, audioContext);
    viz.start();
    visualizerRef.current = viz;

    // Handle resize with debouncing
    window.addEventListener('resize', debouncedResize);

    // Initial resize to fit container
    handleResize();

    // Cleanup
    return () => {
      window.removeEventListener('resize', debouncedResize);
      debouncedResize.cancel(); // Cancel any pending debounced calls
      if (visualizerRef.current) {
        visualizerRef.current.destroy();
        visualizerRef.current = null;
      }
    };
  }, [isInitialized, prefersReducedMotion, getAnalyser, getAudioContext, handleResize, debouncedResize]);

  return (
    <div
      ref={containerRef}
      className={className}
      style={{
        width: '100%',
        height: '100%',
        background: 'transparent',
      }}
      role="img"
      aria-label="Audio visualization"
    />
  );
}
