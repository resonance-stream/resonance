/**
 * Audio Engine Constants
 *
 * Configuration constants for the Web Audio API-based audio engine.
 */

import type { EqBand, EqBandFrequency, EqSettings } from './types';

/**
 * Standard ISO EQ frequencies (Hz)
 * 10-band graphic equalizer based on ISO standard octave bands
 */
export const EQ_FREQUENCIES: readonly number[] = [
  32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000,
] as const;

/**
 * EQ frequency labels for display
 */
export const EQ_FREQUENCY_LABELS: Record<EqBandFrequency, string> = {
  '32': '32',
  '64': '64',
  '125': '125',
  '250': '250',
  '500': '500',
  '1000': '1K',
  '2000': '2K',
  '4000': '4K',
  '8000': '8K',
  '16000': '16K',
};

/**
 * Default EQ band configurations
 * First band uses lowshelf, last band uses highshelf, middle bands use peaking
 */
export const DEFAULT_EQ_BANDS: readonly EqBand[] = [
  { frequency: 32, gain: 0, type: 'lowshelf' },
  { frequency: 64, gain: 0, type: 'peaking', Q: 1.4 },
  { frequency: 125, gain: 0, type: 'peaking', Q: 1.4 },
  { frequency: 250, gain: 0, type: 'peaking', Q: 1.4 },
  { frequency: 500, gain: 0, type: 'peaking', Q: 1.4 },
  { frequency: 1000, gain: 0, type: 'peaking', Q: 1.4 },
  { frequency: 2000, gain: 0, type: 'peaking', Q: 1.4 },
  { frequency: 4000, gain: 0, type: 'peaking', Q: 1.4 },
  { frequency: 8000, gain: 0, type: 'peaking', Q: 1.4 },
  { frequency: 16000, gain: 0, type: 'highshelf' },
] as const;

/**
 * Default flat EQ settings
 */
export const DEFAULT_EQ_SETTINGS: EqSettings = {
  enabled: false,
  preamp: 0,
  bands: {
    '32': 0,
    '64': 0,
    '125': 0,
    '250': 0,
    '500': 0,
    '1000': 0,
    '2000': 0,
    '4000': 0,
    '8000': 0,
    '16000': 0,
  },
};

/**
 * EQ gain limits in dB
 */
export const EQ_GAIN_MIN = -12;
export const EQ_GAIN_MAX = 12;

/**
 * Preamp limits in dB
 */
export const PREAMP_MIN = -12;
export const PREAMP_MAX = 12;

/**
 * Crossfade duration limits in seconds
 */
export const CROSSFADE_MIN = 0;
export const CROSSFADE_MAX = 12;
export const CROSSFADE_DEFAULT = 0; // Disabled by default

/**
 * Default audio engine configuration
 */
export const DEFAULT_CONFIG = {
  /** Buffer size for decoding (not used directly in Web Audio API) */
  bufferSize: 4096,
  /** Start prefetching next track when this many seconds remain */
  prefetchThreshold: 30,
  /** Initial volume (0-1) */
  initialVolume: 0.75,
  /** Time update interval in milliseconds */
  timeUpdateInterval: 250,
  /** Fade duration for pause/resume in seconds */
  fadeDuration: 0.05,
  /** Minimum crossfade overlap in seconds */
  minCrossfadeOverlap: 0.5,
} as const;

/**
 * Audio context state values
 */
export const AUDIO_CONTEXT_STATES = {
  SUSPENDED: 'suspended',
  RUNNING: 'running',
  CLOSED: 'closed',
} as const;

/**
 * Supported audio MIME types for decoding
 */
export const SUPPORTED_AUDIO_TYPES = [
  'audio/mpeg', // MP3
  'audio/flac', // FLAC
  'audio/ogg', // OGG Vorbis
  'audio/opus', // Opus
  'audio/aac', // AAC
  'audio/wav', // WAV
  'audio/mp4', // ALAC in MP4 container
] as const;

/**
 * Prebuilt EQ presets
 */
export const EQ_PRESETS = {
  flat: {
    name: 'Flat',
    preamp: 0,
    bands: { '32': 0, '64': 0, '125': 0, '250': 0, '500': 0, '1000': 0, '2000': 0, '4000': 0, '8000': 0, '16000': 0 },
  },
  bassBoost: {
    name: 'Bass Boost',
    preamp: 0,
    bands: { '32': 6, '64': 5, '125': 4, '250': 2, '500': 0, '1000': 0, '2000': 0, '4000': 0, '8000': 0, '16000': 0 },
  },
  trebleBoost: {
    name: 'Treble Boost',
    preamp: 0,
    bands: { '32': 0, '64': 0, '125': 0, '250': 0, '500': 0, '1000': 2, '2000': 4, '4000': 5, '8000': 6, '16000': 6 },
  },
  vocal: {
    name: 'Vocal',
    preamp: 0,
    bands: { '32': -2, '64': -1, '125': 0, '250': 2, '500': 4, '1000': 4, '2000': 3, '4000': 1, '8000': -1, '16000': -2 },
  },
  rock: {
    name: 'Rock',
    preamp: 0,
    bands: { '32': 4, '64': 3, '125': 1, '250': 0, '500': -1, '1000': 1, '2000': 3, '4000': 4, '8000': 4, '16000': 3 },
  },
  electronic: {
    name: 'Electronic',
    preamp: 0,
    bands: { '32': 5, '64': 4, '125': 2, '250': 0, '500': 0, '1000': 0, '2000': 2, '4000': 4, '8000': 4, '16000': 5 },
  },
  acoustic: {
    name: 'Acoustic',
    preamp: 0,
    bands: { '32': 3, '64': 2, '125': 1, '250': 1, '500': 2, '1000': 2, '2000': 3, '4000': 3, '8000': 2, '16000': 1 },
  },
  classical: {
    name: 'Classical',
    preamp: 0,
    bands: { '32': 3, '64': 2, '125': 1, '250': 0, '500': 0, '1000': 0, '2000': 0, '4000': 1, '8000': 2, '16000': 3 },
  },
} as const;

export type EqPresetName = keyof typeof EQ_PRESETS;
