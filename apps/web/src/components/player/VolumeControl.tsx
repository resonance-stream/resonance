import * as Slider from '@radix-ui/react-slider';
import { Volume, Volume1, Volume2, VolumeX } from 'lucide-react';
import { usePlayerStore } from '../../stores/playerStore';
import { Button } from '../ui/Button';

function getVolumeIcon(volume: number, isMuted: boolean): JSX.Element {
  if (isMuted || volume === 0) {
    return <VolumeX size={20} strokeWidth={2} />;
  }
  if (volume < 0.33) {
    return <Volume size={20} strokeWidth={2} />;
  }
  if (volume < 0.66) {
    return <Volume1 size={20} strokeWidth={2} />;
  }
  return <Volume2 size={20} strokeWidth={2} />;
}

export function VolumeControl(): JSX.Element {
  const volume = usePlayerStore((s) => s.volume);
  const isMuted = usePlayerStore((s) => s.isMuted);
  const setVolume = usePlayerStore((s) => s.setVolume);
  const toggleMute = usePlayerStore((s) => s.toggleMute);

  const handleVolumeChange = (value: number[]): void => {
    const percent = value[0] ?? 0;
    setVolume(percent / 100);
  };

  const displayVolume = isMuted ? 0 : volume * 100;

  return (
    <div className="flex items-center gap-2">
      {/* Mute Toggle */}
      <Button
        variant="icon"
        size="icon"
        onClick={toggleMute}
        aria-label={isMuted ? 'Unmute' : 'Mute'}
        aria-pressed={isMuted}
      >
        {getVolumeIcon(volume, isMuted)}
      </Button>

      {/* Volume Slider */}
      <Slider.Root
        className="relative flex items-center select-none touch-none w-24 h-5 group"
        value={[displayVolume]}
        onValueChange={handleVolumeChange}
        max={100}
        step={1}
        aria-label="Volume"
      >
        <Slider.Track className="relative h-1 grow rounded-full bg-background-tertiary">
          <Slider.Range className="absolute h-full rounded-full bg-accent-light transition-all duration-100" />
        </Slider.Track>
        <Slider.Thumb
          className="block w-3 h-3 rounded-full bg-text-primary
                     opacity-0 group-hover:opacity-100 focus:opacity-100
                     focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow
                     transition-opacity duration-150"
          aria-label="Volume level"
        />
      </Slider.Root>
    </div>
  );
}
