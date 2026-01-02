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
    if (typeof window === 'undefined') return;

    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    const handleChange = (e: MediaQueryListEvent) => {
      setPrefersReducedMotion(e.matches);
      // Let the visualizer effect cleanup handle stopping/destroying to avoid double teardown
      if (e.matches) {
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
    const container = containerRef.current;
    if (!container || !isInitialized) return;

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
    const viz = new Visualizer(container);
    viz.initWithAnalyser(analyser, audioContext);
    viz.start();
    visualizerRef.current = viz;

    // Use ResizeObserver for more efficient resize handling
    // Note: ResizeObserver callback receives (entries, observer) but we only need to trigger resize
    const resizeObserver = new ResizeObserver(function onResize() {
      debouncedResize();
    });
    resizeObserver.observe(container);

    // Cleanup
    return () => {
      resizeObserver.disconnect();
      debouncedResize.cancel();
      if (visualizerRef.current) {
        visualizerRef.current.stop();
        visualizerRef.current.destroy();
        visualizerRef.current = null;
      }
    };
  }, [isInitialized, prefersReducedMotion, getAnalyser, getAudioContext, debouncedResize]);

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
