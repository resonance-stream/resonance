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
  const play = usePlayerStore((s) => s.play);
  const setLoading = usePlayerStore((s) => s.setLoading);
  const setBuffering = usePlayerStore((s) => s.setBuffering);

  // Track whether audio is ready to play (canplay received)
  const isReadyRef = useRef(false);
  // Track whether we initiated the play/pause to avoid sync loops
  const actionSourceRef = useRef<'store' | 'audio' | null>(null);
  // Track pending play intent - canplay handler will trigger play when ready
  const playPendingRef = useRef(false);

  // Seek function exposed via context
  const seek = useCallback((time: number) => {
    const audio = audioRef.current;
    if (!audio) return;

    // Validate input
    if (!Number.isFinite(time)) return;

    // Clamp to valid range
    const duration = Number.isFinite(audio.duration) ? audio.duration : undefined;
    const clamped = duration !== undefined
      ? Math.min(Math.max(0, time), duration)
      : Math.max(0, time);

    try {
      audio.currentTime = clamped;
      setCurrentTime(clamped);
    } catch {
      // Ignore failed seeks (e.g., metadata not loaded yet)
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
      isReadyRef.current = false; // Reset ready state for new track

      if (currentTrack) {
        const streamUrl = `/api/stream/${encodeURIComponent(currentTrack.id)}`;
        audio.src = streamUrl;
        audio.load();
      } else {
        audio.src = '';
        audio.load();
        setLoading(false); // No track, not loading
      }
    }
  }, [currentTrack, setLoading]);

  // Handle play/pause state changes from store
  // Only attempt to play if audio is ready (canplay received)
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio || !audio.src) return;

    // Skip if this change was triggered by audio element events (avoid loops)
    if (actionSourceRef.current === 'audio') {
      actionSourceRef.current = null;
      return;
    }

    actionSourceRef.current = 'store';

    if (isPlaying) {
      if (isReadyRef.current) {
        // Audio is ready, play immediately
        playPendingRef.current = false;
        audio.play().catch((error) => {
          // AbortError is expected during rapid track changes - don't pause
          if (error instanceof Error && error.name === 'AbortError') {
            return;
          }
          console.warn('Play prevented:', error);
          actionSourceRef.current = 'audio';
          pause();
        });
      } else {
        // Audio not ready, signal intent - canplay handler will trigger play
        playPendingRef.current = true;
      }
    } else {
      playPendingRef.current = false;
      audio.pause();
    }
  }, [isPlaying, pause]);

  // Handle volume and mute changes
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;

    // Clamp volume to valid range [0, 1] with fallback
    const safeVolume = Number.isFinite(volume) ? Math.min(1, Math.max(0, volume)) : 1;
    audio.volume = safeVolume;
    audio.muted = isMuted;
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
      setLoading(false);
      setBuffering(false);
      actionSourceRef.current = 'audio';
      pause();
    };

    // Loading state events
    const handleLoadStart = (): void => {
      isReadyRef.current = false;
      setLoading(true);
      setBuffering(false);
    };

    const handleCanPlay = (): void => {
      isReadyRef.current = true;
      setLoading(false);

      // If play was requested before audio was ready, trigger it now
      if (playPendingRef.current && audio.paused) {
        playPendingRef.current = false;
        audio.play().catch((error) => {
          if (error instanceof Error && error.name === 'AbortError') {
            return;
          }
          console.warn('Play prevented after canplay:', error);
          actionSourceRef.current = 'audio';
          pause();
        });
      }
    };

    // Buffering state events
    const handleWaiting = (): void => {
      setBuffering(true);
    };

    const handlePlaying = (): void => {
      setBuffering(false);
      // Sync store if audio started playing (e.g., from autoplay or external control)
      const { isPlaying: storeIsPlaying } = usePlayerStore.getState();
      if (!storeIsPlaying && actionSourceRef.current !== 'store') {
        // Audio started playing externally - sync store
        actionSourceRef.current = 'audio';
        play();
        // Don't clear actionSourceRef here - let the effect clear it
      } else if (actionSourceRef.current === 'store') {
        // Store-initiated play completed - clear the guard
        actionSourceRef.current = null;
      }
    };

    const handlePause = (): void => {
      // Sync store if audio was paused externally (not from store action)
      const { isPlaying: storeIsPlaying } = usePlayerStore.getState();
      if (storeIsPlaying && actionSourceRef.current !== 'store') {
        // Audio paused externally - sync store
        actionSourceRef.current = 'audio';
        pause();
        // Don't clear actionSourceRef here - let the effect clear it
      } else if (actionSourceRef.current === 'store') {
        // Store-initiated pause completed - clear the guard
        actionSourceRef.current = null;
      }
    };

    audio.addEventListener('timeupdate', handleTimeUpdate);
    audio.addEventListener('ended', handleEnded);
    audio.addEventListener('error', handleError);
    audio.addEventListener('loadstart', handleLoadStart);
    audio.addEventListener('canplay', handleCanPlay);
    audio.addEventListener('waiting', handleWaiting);
    audio.addEventListener('playing', handlePlaying);
    audio.addEventListener('pause', handlePause);

    return () => {
      audio.removeEventListener('timeupdate', handleTimeUpdate);
      audio.removeEventListener('ended', handleEnded);
      audio.removeEventListener('error', handleError);
      audio.removeEventListener('loadstart', handleLoadStart);
      audio.removeEventListener('canplay', handleCanPlay);
      audio.removeEventListener('waiting', handleWaiting);
      audio.removeEventListener('playing', handlePlaying);
      audio.removeEventListener('pause', handlePause);
    };
  }, [setCurrentTime, nextTrack, pause, play, setLoading, setBuffering]);

  // Cleanup audio element on unmount to prevent orphaned playback
  useEffect(() => {
    const audio = audioRef.current;
    return () => {
      // Reset sync guards
      actionSourceRef.current = null;
      playPendingRef.current = false;
      isReadyRef.current = false;
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
