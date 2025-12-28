import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Shuffle,
  Repeat,
  Repeat1,
} from 'lucide-react';
import { usePlayerStore } from '../../stores/playerStore';
import { Button } from '../ui/Button';
import { ProgressBar } from './ProgressBar';
import { cn } from '../../lib/utils';

export function PlaybackControls(): JSX.Element {
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const shuffle = usePlayerStore((s) => s.shuffle);
  const repeat = usePlayerStore((s) => s.repeat);
  const queue = usePlayerStore((s) => s.queue);

  const togglePlay = usePlayerStore((s) => s.togglePlay);
  const nextTrack = usePlayerStore((s) => s.nextTrack);
  const previousTrack = usePlayerStore((s) => s.previousTrack);
  const toggleShuffle = usePlayerStore((s) => s.toggleShuffle);
  const cycleRepeat = usePlayerStore((s) => s.cycleRepeat);

  const hasQueue = queue.length > 0;

  return (
    <div className="flex flex-col items-center gap-1 w-full max-w-xl">
      {/* Control Buttons Row */}
      <div className="flex items-center gap-2">
        {/* Shuffle Toggle */}
        <Button
          variant="icon"
          size="icon"
          onClick={toggleShuffle}
          aria-label={shuffle ? 'Disable shuffle' : 'Enable shuffle'}
          aria-pressed={shuffle}
          className={cn(
            'transition-colors duration-150',
            shuffle && 'text-navy hover:text-navy-hover'
          )}
        >
          <Shuffle size={18} strokeWidth={2} />
        </Button>

        {/* Previous Track */}
        <Button
          variant="icon"
          size="icon"
          onClick={previousTrack}
          disabled={!hasQueue}
          aria-label="Previous track"
        >
          <SkipBack size={20} strokeWidth={2} />
        </Button>

        {/* Play/Pause - Main Action Button */}
        <Button
          variant="accent"
          size="icon"
          onClick={togglePlay}
          aria-label={isPlaying ? 'Pause' : 'Play'}
          className="w-10 h-10 rounded-full shadow-[0_0_20px_rgba(37,99,235,0.3)] hover:shadow-[0_0_25px_rgba(37,99,235,0.4)] transition-all duration-150"
        >
          {isPlaying ? (
            <Pause size={20} strokeWidth={2} fill="currentColor" />
          ) : (
            <Play size={20} strokeWidth={2} fill="currentColor" className="ml-0.5" />
          )}
        </Button>

        {/* Next Track */}
        <Button
          variant="icon"
          size="icon"
          onClick={nextTrack}
          disabled={!hasQueue}
          aria-label="Next track"
        >
          <SkipForward size={20} strokeWidth={2} />
        </Button>

        {/* Repeat Cycle */}
        <Button
          variant="icon"
          size="icon"
          onClick={cycleRepeat}
          aria-label={
            repeat === 'off'
              ? 'Enable repeat all'
              : repeat === 'queue'
                ? 'Enable repeat one'
                : 'Disable repeat'
          }
          aria-pressed={repeat !== 'off'}
          className={cn(
            'transition-colors duration-150',
            repeat !== 'off' && 'text-navy hover:text-navy-hover'
          )}
        >
          {repeat === 'track' ? (
            <Repeat1 size={18} strokeWidth={2} />
          ) : (
            <Repeat size={18} strokeWidth={2} />
          )}
        </Button>
      </div>

      {/* Progress Bar Row */}
      <ProgressBar />
    </div>
  );
}
