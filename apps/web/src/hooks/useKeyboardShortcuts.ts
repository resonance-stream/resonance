import { useEffect } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useAudio } from './useAudio';

/**
 * Global keyboard shortcuts for the audio player
 * - Space: Play/Pause
 * - ArrowLeft: Seek backward 5 seconds
 * - ArrowRight: Seek forward 5 seconds
 * - ArrowUp: Increase volume
 * - ArrowDown: Decrease volume
 * - M: Toggle mute
 */
export function useKeyboardShortcuts(): void {
  const { seek } = useAudio();

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent): void => {
      // Don't trigger shortcuts when typing in inputs
      const target = event.target as HTMLElement;
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        return;
      }

      // Read current state at event time to avoid stale closures
      // and prevent unnecessary effect re-runs
      const state = usePlayerStore.getState();
      const { currentTrack, currentTime, volume } = state;
      const { togglePlay, toggleMute, setVolume } = state;

      switch (event.code) {
        case 'Space':
          if (currentTrack) {
            event.preventDefault();
            togglePlay();
          }
          break;

        case 'ArrowLeft':
          if (currentTrack) {
            event.preventDefault();
            const newTime = Math.max(0, currentTime - 5);
            seek(newTime);
          }
          break;

        case 'ArrowRight':
          if (currentTrack) {
            event.preventDefault();
            const maxTime = currentTrack.duration;
            const newTime = Math.min(maxTime, currentTime + 5);
            seek(newTime);
          }
          break;

        case 'ArrowUp':
          event.preventDefault();
          setVolume(Math.min(1, Math.round((volume + 0.1) * 100) / 100));
          break;

        case 'ArrowDown':
          event.preventDefault();
          setVolume(Math.max(0, Math.round((volume - 0.1) * 100) / 100));
          break;

        case 'KeyM':
          event.preventDefault();
          toggleMute();
          break;

        default:
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [seek]); // Only depends on seek which is stable
}
