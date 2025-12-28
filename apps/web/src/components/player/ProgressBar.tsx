import * as Slider from '@radix-ui/react-slider';
import { usePlayerStore } from '../../stores/playerStore';
import { useAudio } from '../../hooks/useAudio';

/**
 * Format seconds to mm:ss display
 */
function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) {
    return '0:00';
  }

  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

export function ProgressBar(): JSX.Element {
  const currentTime = usePlayerStore((s) => s.currentTime);
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const { seek } = useAudio();

  const duration = currentTrack?.duration ?? 0;
  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;

  const handleSeek = (value: number[]): void => {
    const percent = value[0] ?? 0;
    const newTime = (percent / 100) * duration;
    seek(newTime);
  };

  return (
    <div className="flex items-center gap-2 w-full">
      {/* Current Time */}
      <span className="text-xs text-text-muted tabular-nums min-w-[40px] text-right">
        {formatTime(currentTime)}
      </span>

      {/* Seekable Slider */}
      <Slider.Root
        className="relative flex items-center select-none touch-none w-full h-5 group"
        value={[progress]}
        onValueChange={handleSeek}
        max={100}
        step={0.1}
        aria-label="Seek"
      >
        <Slider.Track className="relative h-1 grow rounded-full bg-background-tertiary">
          <Slider.Range className="absolute h-full rounded-full bg-accent-light transition-all duration-100" />
        </Slider.Track>
        <Slider.Thumb
          className="block w-3 h-3 rounded-full bg-text-primary
                     opacity-0 group-hover:opacity-100 focus:opacity-100
                     focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow
                     transition-opacity duration-150"
          aria-label="Seek position"
        />
      </Slider.Root>

      {/* Duration */}
      <span className="text-xs text-text-muted tabular-nums min-w-[40px]">
        {formatTime(duration)}
      </span>
    </div>
  );
}
