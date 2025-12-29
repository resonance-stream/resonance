import { useState } from 'react';
import { usePlayerStore } from '../../stores/playerStore';
import { NowPlaying } from './NowPlaying';
import { PlaybackControls } from './PlaybackControls';
import { VolumeControl } from './VolumeControl';
import { EqualizerPanel } from '../equalizer';
import { QueuePanel } from '../queue';
import { useEqualizerStore } from '../../stores/equalizerStore';
import { cn } from '../../lib/utils';

export function PlayerBar(): JSX.Element | null {
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const queueLength = usePlayerStore((s) => s.queue.length);
  const [showEqualizer, setShowEqualizer] = useState(false);
  const [showQueue, setShowQueue] = useState(false);
  const eqEnabled = useEqualizerStore((s) => s.settings.enabled);

  // Don't render if no track is loaded
  if (!currentTrack) {
    return null;
  }

  // Close other panel when opening one
  const handleShowEqualizer = (): void => {
    setShowQueue(false);
    setShowEqualizer(!showEqualizer);
  };

  const handleShowQueue = (): void => {
    setShowEqualizer(false);
    setShowQueue(!showQueue);
  };

  return (
    <>
      {/* EQ Panel Overlay */}
      {showEqualizer && (
        <div className="fixed bottom-24 right-4 z-50 animate-fade-in">
          <EqualizerPanel onClose={() => setShowEqualizer(false)} />
        </div>
      )}

      {/* Queue Panel Overlay */}
      {showQueue && (
        <div className="fixed bottom-24 right-4 z-50 animate-fade-in">
          <QueuePanel onClose={() => setShowQueue(false)} />
        </div>
      )}

      <div
        className="fixed bottom-0 left-0 right-0 h-20 z-50
                   bg-background/95 backdrop-blur-xl
                   border-t border-white/5
                   flex items-center px-4"
        role="region"
        aria-label="Audio player"
      >
        {/* Three-column grid layout */}
        <div className="w-full grid grid-cols-3 items-center gap-4">
          {/* Left: Now Playing */}
          <div className="flex justify-start">
            <NowPlaying />
          </div>

          {/* Center: Playback Controls */}
          <div className="flex justify-center">
            <PlaybackControls />
          </div>

          {/* Right: Volume Control + EQ Toggle + Queue Toggle */}
          <div className="flex justify-end items-center gap-2">
            {/* Queue Toggle Button */}
            <button
              onClick={handleShowQueue}
              className={cn(
                'p-2 rounded-full transition-colors',
                'hover:bg-white/10',
                showQueue && 'bg-white/10',
                queueLength > 0 && 'text-accent'
              )}
              aria-label={showQueue ? 'Hide queue' : 'Show queue'}
              aria-pressed={showQueue}
            >
              <svg
                className="w-5 h-5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M4 6h16M4 10h16M4 14h16M4 18h16"
                />
              </svg>
            </button>

            {/* EQ Toggle Button */}
            <button
              onClick={handleShowEqualizer}
              className={cn(
                'p-2 rounded-full transition-colors',
                'hover:bg-white/10',
                showEqualizer && 'bg-white/10',
                eqEnabled && 'text-accent'
              )}
              aria-label={showEqualizer ? 'Hide equalizer' : 'Show equalizer'}
              aria-pressed={showEqualizer}
            >
              <svg
                className="w-5 h-5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4"
                />
              </svg>
            </button>
            <VolumeControl />
          </div>
        </div>
      </div>
    </>
  );
}
