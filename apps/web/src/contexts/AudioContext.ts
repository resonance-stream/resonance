import { createContext, type RefObject } from 'react';
import type { AudioEngine } from '../audio';
import type { EqSettings, CrossfadeSettings, PlaybackState } from '../audio/types';

export interface AudioContextValue {
  /** Seek to a specific time in seconds */
  seek: (time: number) => void;

  /** Legacy audio element ref (deprecated, use engine methods) */
  audioRef: RefObject<HTMLAudioElement | null>;

  /** The audio engine instance */
  engine: AudioEngine | null;

  /** Current playback state from the engine */
  engineState: PlaybackState;

  /** Whether the audio engine is initialized */
  isInitialized: boolean;

  /** Initialize the audio engine (must be called after user gesture) */
  initializeEngine: () => Promise<void>;

  /** Prefetch the next track for gapless playback */
  prefetchNextTrack: (trackId: string, url: string) => Promise<void>;

  /** Get current EQ settings */
  getEqSettings: () => EqSettings;

  /** Apply EQ settings */
  applyEqSettings: (settings: EqSettings) => void;

  /** Get crossfade settings */
  getCrossfadeSettings: () => CrossfadeSettings;

  /** Set crossfade settings */
  setCrossfadeSettings: (settings: CrossfadeSettings) => void;
}

export const AudioContext = createContext<AudioContextValue | null>(null);
