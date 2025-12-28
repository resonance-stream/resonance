import { useRef, useEffect, useState } from 'react';
import { usePlayerStore } from '../../stores/playerStore';
import { AlbumArt } from '../media/AlbumArt';
import { cn } from '../../lib/utils';

export function NowPlaying(): JSX.Element | null {
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const isPlaying = usePlayerStore((s) => s.isPlaying);

  // Track previous track ID to only announce on actual track changes
  const previousTrackIdRef = useRef<string | null>(null);
  const [announcement, setAnnouncement] = useState<string | null>(null);

  useEffect(() => {
    if (currentTrack && currentTrack.id !== previousTrackIdRef.current) {
      previousTrackIdRef.current = currentTrack.id;
      setAnnouncement(`Now playing: ${currentTrack.title} by ${currentTrack.artist}`);
      // Clear announcement after screen reader has time to read it
      const timer = setTimeout(() => setAnnouncement(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [currentTrack]);

  if (!currentTrack) {
    return null;
  }

  return (
    <div className="flex items-center gap-3 min-w-0">
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
      <div className="flex flex-col min-w-0">
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
    </div>
  );
}
