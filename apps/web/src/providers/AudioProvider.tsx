import { useRef, useEffect, useCallback, useMemo, useState, type ReactNode } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useEqualizerStore } from '../stores/equalizerStore';
import { KeyboardShortcuts } from '../components/player/KeyboardShortcuts';
import { AudioContext, type AudioContextValue } from '../contexts/AudioContext';
import { AudioEngine } from '../audio';
import type { EqSettings, CrossfadeSettings, PlaybackState } from '../audio/types';

interface AudioProviderProps {
  children: ReactNode;
}

export function AudioProvider({ children }: AudioProviderProps): JSX.Element {
  // Keep legacy audio ref for backward compatibility
  const audioRef = useRef<HTMLAudioElement | null>(null);

  // Audio engine state
  const [engine] = useState<AudioEngine>(() => new AudioEngine());
  const [isInitialized, setIsInitialized] = useState(false);
  const [engineState, setEngineState] = useState<PlaybackState>('stopped');

  // Track refs to prevent duplicate loads
  const lastTrackIdRef = useRef<string | null>(null);
  const lastTimeUpdateRef = useRef<number>(0);

  // Subscribe to playerStore state
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const volume = usePlayerStore((s) => s.volume);
  const isMuted = usePlayerStore((s) => s.isMuted);
  const queue = usePlayerStore((s) => s.queue);
  const queueIndex = usePlayerStore((s) => s.queueIndex);

  // Use refs for values accessed in event handlers to avoid recreating listeners
  const repeatRef = useRef(usePlayerStore.getState().repeat);
  const queueRef = useRef(queue);
  const queueIndexRef = useRef(queueIndex);

  useEffect(() => {
    queueRef.current = queue;
    queueIndexRef.current = queueIndex;
  }, [queue, queueIndex]);

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

  // Track whether we initiated the play/pause to avoid sync loops
  const actionSourceRef = useRef<'store' | 'engine' | null>(null);

  // Initialize audio engine
  const initializeEngine = useCallback(async () => {
    if (isInitialized) return;

    try {
      await engine.initialize();
      setIsInitialized(true);
    } catch (error) {
      console.error('Failed to initialize audio engine:', error);
    }
  }, [engine, isInitialized]);

  // Set up engine event listeners
  useEffect(() => {
    const handleStateChange = (state: PlaybackState) => {
      setEngineState(state);

      if (state === 'playing' && actionSourceRef.current !== 'store') {
        actionSourceRef.current = 'engine';
        play();
      } else if (state === 'paused' && actionSourceRef.current !== 'store') {
        actionSourceRef.current = 'engine';
        pause();
      }
    };

    const handleTimeUpdate = (time: number) => {
      const now = Date.now();
      if (now - lastTimeUpdateRef.current >= 250) {
        lastTimeUpdateRef.current = now;
        setCurrentTime(time);
      }
    };

    const handleEnded = () => {
      if (repeatRef.current === 'track') {
        engine.seek(0);
        engine.play();
      } else {
        nextTrack();
      }
    };

    const handleError = (error: Error) => {
      console.error('Audio engine error:', error);
      setLoading(false);
      setBuffering(false);
      actionSourceRef.current = 'engine';
      pause();
    };

    const handleBuffering = (isBuffering: boolean) => {
      setBuffering(isBuffering);
    };

    const handleLoaded = () => {
      setLoading(false);
    };

    const handlePrefetchNeeded = () => {
      // Use refs to get current queue state without recreating listener
      const currentQueue = queueRef.current;
      const currentQueueIndex = queueIndexRef.current;

      // Check if there's a next track and we haven't already prefetched it
      if (currentQueueIndex < currentQueue.length - 1 && !engine.hasPrefetchedTrack) {
        const nextTrackData = currentQueue[currentQueueIndex + 1];
        if (nextTrackData) {
          const streamUrl = `/api/stream/${encodeURIComponent(nextTrackData.id)}`;
          engine.prefetchTrack({
            id: nextTrackData.id,
            url: streamUrl,
            duration: nextTrackData.duration,
          });
        }
      }
    };

    engine.on('stateChange', handleStateChange);
    engine.on('timeUpdate', handleTimeUpdate);
    engine.on('ended', handleEnded);
    engine.on('error', handleError);
    engine.on('buffering', handleBuffering);
    engine.on('loaded', handleLoaded);
    engine.on('prefetchNeeded', handlePrefetchNeeded);

    return () => {
      engine.off('stateChange', handleStateChange);
      engine.off('timeUpdate', handleTimeUpdate);
      engine.off('ended', handleEnded);
      engine.off('error', handleError);
      engine.off('buffering', handleBuffering);
      engine.off('loaded', handleLoaded);
      engine.off('prefetchNeeded', handlePrefetchNeeded);
    };
  }, [engine, setCurrentTime, nextTrack, pause, play, setLoading, setBuffering]);

  // Handle track changes
  useEffect(() => {
    const trackId = currentTrack?.id ?? null;

    if (trackId !== lastTrackIdRef.current) {
      lastTrackIdRef.current = trackId;

      if (currentTrack) {
        const streamUrl = `/api/stream/${encodeURIComponent(currentTrack.id)}`;
        setLoading(true);

        // Initialize engine on first track load (user gesture)
        const loadTrack = async () => {
          try {
            await initializeEngine();
            await engine.loadTrack({
              id: currentTrack.id,
              url: streamUrl,
              duration: currentTrack.duration,
            });

            // Auto-play if isPlaying is true
            if (usePlayerStore.getState().isPlaying) {
              engine.play();
            }
          } catch (error) {
            console.error('Failed to load track:', error);
            setLoading(false);
          }
        };

        loadTrack();
      } else {
        engine.stop();
        setLoading(false);
      }
    }
  }, [currentTrack, engine, initializeEngine, setLoading]);

  // Handle play/pause state changes from store
  useEffect(() => {
    if (!isInitialized) return;

    // Skip if this change was triggered by engine events
    if (actionSourceRef.current === 'engine') {
      actionSourceRef.current = null;
      return;
    }

    actionSourceRef.current = 'store';

    if (isPlaying && engineState !== 'playing') {
      engine.play();
    } else if (!isPlaying && engineState === 'playing') {
      engine.pause();
    }

    // Clear action source using microtask for more predictable timing
    queueMicrotask(() => {
      if (actionSourceRef.current === 'store') {
        actionSourceRef.current = null;
      }
    });
  }, [isPlaying, engineState, engine, isInitialized]);

  // Handle volume changes
  useEffect(() => {
    engine.setVolume(volume);
    engine.setMuted(isMuted);
  }, [engine, volume, isMuted]);

  // Sync crossfade settings from settings store
  const crossfadeEnabled = useSettingsStore((s) => s.playback.crossfadeEnabled);
  const crossfadeDuration = useSettingsStore((s) => s.playback.crossfadeDuration);

  useEffect(() => {
    engine.setCrossfade({
      enabled: crossfadeEnabled,
      duration: crossfadeDuration,
    });
  }, [engine, crossfadeEnabled, crossfadeDuration]);

  // Sync EQ settings from equalizer store
  // This ensures EQ is applied on app start, not just when EqualizerPanel is mounted
  const eqSettings = useEqualizerStore((s) => s.settings);

  useEffect(() => {
    engine.applyEqSettings(eqSettings);
  }, [engine, eqSettings]);

  // Seek function exposed via context
  const seek = useCallback((time: number) => {
    if (!isInitialized) return;
    engine.seek(time);
    setCurrentTime(time);
  }, [engine, isInitialized, setCurrentTime]);

  // Prefetch next track
  const prefetchNextTrack = useCallback(async (trackId: string, url: string) => {
    if (!isInitialized) return;
    await engine.prefetchTrack({ id: trackId, url });
  }, [engine, isInitialized]);

  // EQ settings
  const getEqSettings = useCallback((): EqSettings => {
    return engine.getEqSettings();
  }, [engine]);

  const applyEqSettings = useCallback((settings: EqSettings) => {
    engine.applyEqSettings(settings);
  }, [engine]);

  // Crossfade settings
  const getCrossfadeSettings = useCallback((): CrossfadeSettings => {
    return engine.getCrossfadeSettings();
  }, [engine]);

  const setCrossfadeSettings = useCallback((settings: CrossfadeSettings) => {
    engine.setCrossfade(settings);
  }, [engine]);

  // Visualizer access methods
  const getAnalyser = useCallback((): AnalyserNode | null => {
    return engine.getAnalyser();
  }, [engine]);

  const getAudioContext = useCallback((): globalThis.AudioContext | null => {
    const ctx = engine.getAudioContext();
    if (!ctx || ctx.state === 'closed') return null;
    return ctx;
  }, [engine]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      engine.destroy();
    };
  }, [engine]);

  // Memoize context value
  const contextValue = useMemo<AudioContextValue>(
    () => ({
      seek,
      audioRef,
      engine,
      engineState,
      isInitialized,
      initializeEngine,
      prefetchNextTrack,
      getEqSettings,
      applyEqSettings,
      getCrossfadeSettings,
      setCrossfadeSettings,
      getAnalyser,
      getAudioContext,
    }),
    [
      seek,
      engine,
      engineState,
      isInitialized,
      initializeEngine,
      prefetchNextTrack,
      getEqSettings,
      applyEqSettings,
      getCrossfadeSettings,
      setCrossfadeSettings,
      getAnalyser,
      getAudioContext,
    ]
  );

  return (
    <AudioContext.Provider value={contextValue}>
      {/* Keep hidden audio element for legacy compatibility */}
      <audio ref={audioRef} style={{ display: 'none' }} />
      <KeyboardShortcuts />
      {children}
    </AudioContext.Provider>
  );
}
