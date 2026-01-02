import { useRef, useEffect, useState } from 'react';
import { usePlayerStore } from '../../stores/playerStore';
import { useVisualizerStore } from '../../stores/visualizerStore';
import { AlbumArt } from '../media/AlbumArt';
import { cn } from '../../lib/utils';

export function NowPlaying(): JSX.Element | null {
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const openFullscreen = useVisualizerStore((s) => s.openFullscreen);

  // Track previous track ID to only announce on actual track changes
  const previousTrackIdRef = useRef<string | null>(null);
  const [announcement, setAnnouncement] = useState<string | null>(null);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    if (currentTrack && currentTrack.id !== previousTrackIdRef.current) {
      previousTrackIdRef.current = currentTrack.id;
      setAnnouncement(`Now playing: ${currentTrack.title} by ${currentTrack.artist}`);
      // Clear announcement after screen reader has time to read it
      timer = setTimeout(() => setAnnouncement(null), 3000);
    }
    return () => {
      if (timer) clearTimeout(timer);
    };
  }, [currentTrack]);

  if (!currentTrack) {
    return null;
  }

  return (
    <button
      onClick={openFullscreen}
      className={cn(
        'flex items-center gap-3 min-w-0',
        'rounded-lg p-1 -m-1',
        'hover:bg-white/5 transition-colors',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/50'
      )}
      aria-label={`Now playing: ${currentTrack.title} by ${currentTrack.artist}. Click to open fullscreen view.`}
    >
      {/* Album Art with subtle animation when playing */}
      <div
        className={cn(
          'relative flex-shrink-0 transition-all duration-300',
          isPlaying && 'shadow-[0_0_20px_rgba(90,106,125,0.3)]'
        )}
      >
        <AlbumArt
          src={currentTrack.coverUrl}
          alt={`${currentTrack.albumTitle} album art`}
          size="sm"
          showPlayButton={false}
        />
      </div>

      {/* Track Info */}
      <div className="flex flex-col min-w-0 text-left">
        {/* Track Title - uses display font for premium feel */}
        <span
          className="font-display text-sm text-text-primary truncate"
          title={currentTrack.title}
        >
          {currentTrack.title}
        </span>

        {/* Artist Name */}
        <span
          className="text-xs text-text-secondary truncate"
          title={currentTrack.artist}
        >
          {currentTrack.artist}
        </span>
      </div>

      {/* Screen reader announcement only on track changes */}
      {announcement && (
        <div className="sr-only" role="status" aria-live="polite">
          {announcement}
        </div>
      )}
    </button>
  );
}
