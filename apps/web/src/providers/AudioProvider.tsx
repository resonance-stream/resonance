import { useRef, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { KeyboardShortcuts } from '../components/player/KeyboardShortcuts';
import { AudioContext, type AudioContextValue } from '../contexts/AudioContext';

interface AudioProviderProps {
  children: ReactNode;
}

export function AudioProvider({ children }: AudioProviderProps): JSX.Element {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const lastTrackIdRef = useRef<string | null>(null);
  const lastTimeUpdateRef = useRef<number>(0);

  // Subscribe to playerStore state
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const volume = usePlayerStore((s) => s.volume);
  const isMuted = usePlayerStore((s) => s.isMuted);

  // Use refs for values accessed in event handlers to avoid recreating listeners
  const repeatRef = useRef(usePlayerStore.getState().repeat);
  useEffect(() => {
    return usePlayerStore.subscribe((state) => {
      repeatRef.current = state.repeat;
    });
  }, []);

  // Get actions from store
  const setCurrentTime = usePlayerStore((s) => s.setCurrentTime);
  const nextTrack = usePlayerStore((s) => s.nextTrack);
  const pause = usePlayerStore((s) => s.pause);

  // Seek function exposed via context
  const seek = useCallback((time: number) => {
    if (audioRef.current) {
      audioRef.current.currentTime = time;
      setCurrentTime(time);
    }
  }, [setCurrentTime]);

  // Handle track changes - only update source when track actually changes
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;

    const trackId = currentTrack?.id ?? null;

    // Only update source if track actually changed
    if (trackId !== lastTrackIdRef.current) {
      lastTrackIdRef.current = trackId;

      if (currentTrack) {
        const streamUrl = `/api/stream/${currentTrack.id}`;
        audio.src = streamUrl;
        audio.load();
      } else {
        audio.src = '';
        audio.load();
      }
    }
  }, [currentTrack]);

  // Handle play/pause state changes (separate from track loading)
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio || !audio.src) return;

    if (isPlaying) {
      audio.play().catch((error) => {
        console.warn('Play prevented:', error);
        pause();
      });
    } else {
      audio.pause();
    }
  }, [isPlaying, pause]);

  // Handle volume changes
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;

    audio.volume = isMuted ? 0 : volume;
  }, [volume, isMuted]);

  // Set up audio event listeners (stable - no recreation on state changes)
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;

    // Throttle timeupdate to ~4 updates per second for performance
    const handleTimeUpdate = (): void => {
      const now = Date.now();
      if (now - lastTimeUpdateRef.current >= 250) {
        lastTimeUpdateRef.current = now;
        setCurrentTime(audio.currentTime);
      }
    };

    const handleEnded = (): void => {
      if (repeatRef.current === 'track') {
        // Repeat the current track
        audio.currentTime = 0;
        audio.play().catch(console.warn);
      } else {
        // Move to next track (or stop if at end)
        nextTrack();
      }
    };

    const handleError = (e: Event): void => {
      console.error('Audio error:', e);
      pause();
    };

    audio.addEventListener('timeupdate', handleTimeUpdate);
    audio.addEventListener('ended', handleEnded);
    audio.addEventListener('error', handleError);

    return () => {
      audio.removeEventListener('timeupdate', handleTimeUpdate);
      audio.removeEventListener('ended', handleEnded);
      audio.removeEventListener('error', handleError);
    };
  }, [setCurrentTime, nextTrack, pause]);

  // Cleanup audio element on unmount to prevent orphaned playback
  useEffect(() => {
    const audio = audioRef.current;
    return () => {
      if (audio) {
        audio.pause();
        audio.src = '';
        audio.load();
      }
    };
  }, []);

  // Memoize context value to prevent unnecessary re-renders in consumers
  const contextValue = useMemo<AudioContextValue>(
    () => ({ seek, audioRef }),
    [seek]
  );

  return (
    <AudioContext.Provider value={contextValue}>
      <audio ref={audioRef} preload="auto" />
      <KeyboardShortcuts />
      {children}
    </AudioContext.Provider>
  );
}
