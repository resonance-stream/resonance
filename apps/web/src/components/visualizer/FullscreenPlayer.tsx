import { useEffect, useCallback } from 'react';
import {
  ChevronDown,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Heart,
  Share2,
  Volume2,
} from 'lucide-react';
import { usePlayerStore } from '../../stores/playerStore';
import { useVisualizerStore } from '../../stores/visualizerStore';
import { AlbumArt } from '../media/AlbumArt';
import { ProgressBar } from '../player/ProgressBar';
import { Button } from '../ui/Button';
import { MusicolorsVisualizer } from './MusicolorsVisualizer';
import { FullscreenMenu } from './FullscreenMenu';
import { VisualizerErrorBoundary } from './VisualizerErrorBoundary';
import { cn } from '../../lib/utils';

/**
 * Fullscreen player view (Spotify-style Now Playing)
 * Shows album art or visualizer, track info, and playback controls
 */
export function FullscreenPlayer(): JSX.Element | null {
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const togglePlay = usePlayerStore((s) => s.togglePlay);
  const nextTrack = usePlayerStore((s) => s.nextTrack);
  const previousTrack = usePlayerStore((s) => s.previousTrack);

  const isFullscreen = useVisualizerStore((s) => s.isFullscreen);
  const showVisualizer = useVisualizerStore((s) => s.showVisualizer);
  const closeFullscreen = useVisualizerStore((s) => s.closeFullscreen);

  // Handle escape key to close fullscreen
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isFullscreen) {
        closeFullscreen();
      }
    },
    [isFullscreen, closeFullscreen]
  );

  useEffect(() => {
    if (!isFullscreen) return;
    document.addEventListener('keydown', handleKeyDown, true);
    return () => document.removeEventListener('keydown', handleKeyDown, true);
  }, [handleKeyDown, isFullscreen]);

  // Prevent body scroll when fullscreen is open
  useEffect(() => {
    if (!isFullscreen) return;

    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';

    return () => {
      document.body.style.overflow = prevOverflow;
    };
  }, [isFullscreen]);

  // Auto-close if track becomes null while fullscreen is open
  useEffect(() => {
    if (isFullscreen && !currentTrack) {
      closeFullscreen();
    }
  }, [isFullscreen, currentTrack, closeFullscreen]);

  if (!isFullscreen || !currentTrack) {
    return null;
  }

  return (
    <div
      className={cn(
        'fixed inset-0 z-[100] bg-background',
        'flex flex-col items-center justify-between',
        'px-6 py-8 md:px-12 md:py-12',
        'animate-fade-in'
      )}
      role="dialog"
      aria-modal="true"
      aria-label="Now playing fullscreen view"
    >
      {/* Header */}
      <div className="w-full max-w-lg flex items-center justify-between">
        {/* Close button */}
        <Button
          variant="icon"
          size="icon"
          onClick={closeFullscreen}
          aria-label="Close fullscreen view"
          className="p-2"
        >
          <ChevronDown size={24} />
        </Button>

        {/* Context (e.g., playlist name) */}
        <span className="text-sm text-text-secondary font-medium uppercase tracking-wide">
          Now Playing
        </span>

        {/* Menu button */}
        <FullscreenMenu />
      </div>

      {/* Album Art / Visualizer Area */}
      <div className="flex-1 flex items-center justify-center w-full max-w-lg py-8">
        <div className="relative w-80 h-80 rounded-lg overflow-hidden shadow-2xl">
          {showVisualizer ? (
            <VisualizerErrorBoundary
              fallback={
                <AlbumArt
                  src={currentTrack.coverUrl}
                  alt={`${currentTrack.albumTitle} album art`}
                  size="fullscreen"
                  showPlayButton={false}
                  className="w-full h-full"
                />
              }
            >
              <MusicolorsVisualizer className="absolute inset-0" />
            </VisualizerErrorBoundary>
          ) : (
            <AlbumArt
              src={currentTrack.coverUrl}
              alt={`${currentTrack.albumTitle} album art`}
              size="fullscreen"
              showPlayButton={false}
              className="w-full h-full"
            />
          )}
        </div>
      </div>

      {/* Track Info + Controls */}
      <div className="w-full max-w-lg space-y-6">
        {/* Track Title and Artist */}
        <div className="flex items-start justify-between">
          <div className="min-w-0 flex-1 pr-4">
            <h2 className="font-display text-xl md:text-2xl text-text-primary truncate">
              {currentTrack.title}
            </h2>
            <p className="text-text-secondary truncate">
              {currentTrack.artist}
            </p>
          </div>
          {/* Like button */}
          <Button
            variant="icon"
            size="icon"
            aria-label="Add to favorites"
            className="flex-shrink-0"
          >
            <Heart size={24} />
          </Button>
        </div>

        {/* Progress Bar */}
        <ProgressBar />

        {/* Playback Controls */}
        <div className="flex items-center justify-center gap-6">
          <Button
            variant="icon"
            size="icon"
            onClick={previousTrack}
            aria-label="Previous track"
            className="p-3"
          >
            <SkipBack size={28} strokeWidth={2} />
          </Button>

          <Button
            variant="accent"
            size="icon"
            onClick={togglePlay}
            aria-label={isPlaying ? 'Pause' : 'Play'}
            className="w-16 h-16 rounded-full shadow-[0_0_30px_rgba(37,99,235,0.4)]"
          >
            {isPlaying ? (
              <Pause size={28} strokeWidth={2} fill="currentColor" />
            ) : (
              <Play size={28} strokeWidth={2} fill="currentColor" className="ml-1" />
            )}
          </Button>

          <Button
            variant="icon"
            size="icon"
            onClick={nextTrack}
            aria-label="Next track"
            className="p-3"
          >
            <SkipForward size={28} strokeWidth={2} />
          </Button>
        </div>

        {/* Bottom Row: Quality, Share, Device */}
        <div className="flex items-center justify-between text-text-muted">
          <span className="text-xs uppercase tracking-wide">
            Audio
          </span>
          <div className="flex items-center gap-4">
            <Button
              variant="icon"
              size="icon"
              aria-label="Share"
              className="p-2"
            >
              <Share2 size={20} />
            </Button>
            <Button
              variant="icon"
              size="icon"
              aria-label="Devices"
              className="p-2"
            >
              <Volume2 size={20} />
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
