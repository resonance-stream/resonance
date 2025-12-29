/**
 * Audio Engine Types
 *
 * TypeScript interfaces for the Web Audio API-based audio engine.
 */

/**
 * Playback state of the audio engine
 */
export type PlaybackState = 'stopped' | 'playing' | 'paused' | 'loading';

/**
 * EQ band frequency identifier
 */
export type EqBandFrequency =
  | '32'
  | '64'
  | '125'
  | '250'
  | '500'
  | '1000'
  | '2000'
  | '4000'
  | '8000'
  | '16000';

/**
 * EQ band configuration
 */
export interface EqBand {
  frequency: number;
  gain: number; // -12 to +12 dB
  type: BiquadFilterType;
  Q?: number; // Quality factor for peaking filters
}

/**
 * Complete EQ settings
 */
export interface EqSettings {
  enabled: boolean;
  preamp: number; // -12 to +12 dB
  bands: Record<EqBandFrequency, number>;
}

/**
 * Crossfade configuration
 */
export interface CrossfadeSettings {
  enabled: boolean;
  duration: number; // 0-12 seconds
}

/**
 * Audio engine configuration
 */
export interface AudioEngineConfig {
  /** Buffer size for Web Audio API (default: 4096) */
  bufferSize?: number;
  /** Prefetch threshold in seconds (default: 30) */
  prefetchThreshold?: number;
  /** Initial volume (0-1, default: 0.75) */
  initialVolume?: number;
  /** Crossfade settings */
  crossfade?: CrossfadeSettings;
  /** EQ settings */
  eq?: EqSettings;
}

/**
 * Track information for the audio engine
 */
export interface AudioTrack {
  id: string;
  url: string;
  duration?: number;
}

/**
 * Audio engine event types
 */
export interface AudioEngineEvents {
  /** Fired when playback state changes */
  stateChange: (state: PlaybackState) => void;
  /** Fired when current time updates */
  timeUpdate: (currentTime: number) => void;
  /** Fired when duration is known */
  durationChange: (duration: number) => void;
  /** Fired when track ends */
  ended: () => void;
  /** Fired when an error occurs */
  error: (error: Error) => void;
  /** Fired when buffering state changes */
  buffering: (isBuffering: boolean) => void;
  /** Fired when a track is loaded and ready */
  loaded: () => void;
  /** Fired when next track is prefetched */
  prefetched: (trackId: string) => void;
  /** Fired when prefetch should be triggered (seconds remaining) */
  prefetchNeeded: (secondsRemaining: number) => void;
}

/**
 * Audio engine event listener
 */
export type AudioEngineEventListener<K extends keyof AudioEngineEvents> =
  AudioEngineEvents[K];

/**
 * Internal source node state for managing gapless playback
 */
export interface SourceNodeState {
  source: AudioBufferSourceNode;
  gainNode: GainNode;
  buffer: AudioBuffer;
  startTime: number; // AudioContext time when playback started
  offsetTime: number; // Offset into the buffer when started
  trackId: string;
}

/**
 * Audio analysis data for visualizations (future use)
 */
export interface AudioAnalysisData {
  frequencyData: Uint8Array;
  timeDomainData: Uint8Array;
  averageFrequency: number;
  peakFrequency: number;
}
