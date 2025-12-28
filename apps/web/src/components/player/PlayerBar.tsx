import { usePlayerStore } from '../../stores/playerStore';
import { NowPlaying } from './NowPlaying';
import { PlaybackControls } from './PlaybackControls';
import { VolumeControl } from './VolumeControl';

export function PlayerBar(): JSX.Element | null {
  const currentTrack = usePlayerStore((s) => s.currentTrack);

  // Don't render if no track is loaded
  if (!currentTrack) {
    return null;
  }

  return (
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

        {/* Right: Volume Control */}
        <div className="flex justify-end">
          <VolumeControl />
        </div>
      </div>
    </div>
  );
}
