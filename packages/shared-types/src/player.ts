/**
 * Player and playback-related types for Resonance
 */

import type { Track } from './library.js';

// ============================================================================
// Playback State Types
// ============================================================================

/**
 * Current playback state
 */
export interface PlaybackState {
  /** Currently playing track ID, null if nothing playing */
  trackId: string | null;
  /** Whether audio is currently playing */
  isPlaying: boolean;
  /** Current position in seconds */
  position: number;
  /** Volume level (0-1) */
  volume: number;
  /** Whether audio is muted */
  isMuted: boolean;
  /** Shuffle mode enabled */
  shuffle: boolean;
  /** Repeat mode */
  repeat: RepeatMode;
  /** Ordered queue of track IDs */
  queueIds: string[];
  /** Current index in the queue */
  queueIndex: number;
  /** Playback quality setting */
  quality: PlaybackQuality;
}

/**
 * Repeat mode options
 */
export type RepeatMode = 'off' | 'track' | 'queue';

/**
 * Audio quality levels
 */
export type PlaybackQuality = 'low' | 'medium' | 'high' | 'lossless';

// ============================================================================
// Queue Types
// ============================================================================

/**
 * Queue item with track data and queue-specific metadata
 */
export interface QueueItem {
  /** Unique queue item ID (different from track ID) */
  id: string;
  /** Track ID reference */
  trackId: string;
  /** Full track data (optional, for client caching) */
  track?: Track;
  /** Source context for this queue item */
  context: QueueContext;
  /** When this item was added to the queue */
  addedAt: string;
}

/**
 * Context describing where a queue item originated from
 */
export interface QueueContext {
  /** Type of source */
  type: 'album' | 'artist' | 'playlist' | 'search' | 'recommendation' | 'radio' | 'manual';
  /** ID of the source (album ID, playlist ID, etc.) */
  id?: string;
  /** Display name of the source */
  name?: string;
}

/**
 * Queue manipulation actions
 */
export type QueueAction =
  | { type: 'add'; trackIds: string[]; position?: 'next' | 'last' }
  | { type: 'remove'; queueItemIds: string[] }
  | { type: 'move'; queueItemId: string; toIndex: number }
  | { type: 'clear' }
  | { type: 'shuffle' }
  | { type: 'replace'; trackIds: string[]; startIndex?: number };

// ============================================================================
// Device Types
// ============================================================================

/**
 * Connected device information
 */
export interface Device {
  /** Unique device ID */
  id: string;
  /** User-friendly device name */
  name: string;
  /** Device type/platform */
  type: DeviceType;
  /** Whether this device is currently controlling playback */
  isActive: boolean;
  /** Whether this device is currently available */
  isAvailable: boolean;
  /** Current volume on this device (0-100) */
  volume: number;
  /** Last time this device was seen online */
  lastSeen: string;
  /** Device capabilities */
  capabilities: DeviceCapabilities;
}

/**
 * Device type/platform
 */
export type DeviceType = 'web' | 'mobile' | 'desktop' | 'speaker' | 'tv' | 'unknown';

/**
 * Device capabilities
 */
export interface DeviceCapabilities {
  /** Whether device supports volume control */
  volumeControl: boolean;
  /** Whether device supports seeking */
  seeking: boolean;
  /** Whether device supports gapless playback */
  gapless: boolean;
  /** Whether device supports crossfade */
  crossfade: boolean;
  /** Maximum supported quality */
  maxQuality: PlaybackQuality;
}

// ============================================================================
// Audio Processing Types
// ============================================================================

/**
 * 10-band equalizer settings
 */
export interface EqualizerSettings {
  /** Whether equalizer is enabled */
  enabled: boolean;
  /** Preset name if using a preset, null for custom */
  preset: EqualizerPreset | null;
  /** Band gains in dB (-12 to +12) */
  bands: EqualizerBands;
}

/**
 * Equalizer band frequencies and their gains
 */
export interface EqualizerBands {
  /** 32 Hz band */
  hz32: number;
  /** 64 Hz band */
  hz64: number;
  /** 125 Hz band */
  hz125: number;
  /** 250 Hz band */
  hz250: number;
  /** 500 Hz band */
  hz500: number;
  /** 1 kHz band */
  hz1k: number;
  /** 2 kHz band */
  hz2k: number;
  /** 4 kHz band */
  hz4k: number;
  /** 8 kHz band */
  hz8k: number;
  /** 16 kHz band */
  hz16k: number;
}

/**
 * Built-in equalizer presets
 */
export type EqualizerPreset =
  | 'flat'
  | 'bass_boost'
  | 'treble_boost'
  | 'rock'
  | 'pop'
  | 'jazz'
  | 'classical'
  | 'electronic'
  | 'vocal'
  | 'loudness';

/**
 * Crossfade settings
 */
export interface CrossfadeSettings {
  /** Whether crossfade is enabled */
  enabled: boolean;
  /** Crossfade duration in seconds (1-12) */
  duration: number;
  /** Curve type for crossfade */
  curve: 'linear' | 'equal_power' | 'logarithmic';
}

/**
 * Normalization (ReplayGain) settings
 */
export interface NormalizationSettings {
  /** Whether normalization is enabled */
  enabled: boolean;
  /** Normalization mode */
  mode: 'track' | 'album';
  /** Target loudness in LUFS */
  targetLoudness: number;
  /** Whether to prevent clipping */
  preventClipping: boolean;
}

// ============================================================================
// Playback History Types
// ============================================================================

/**
 * Single playback history entry
 */
export interface PlaybackHistoryEntry {
  /** History entry ID */
  id: string;
  /** Track that was played */
  trackId: string;
  /** When playback started */
  playedAt: string;
  /** How long the track was played (seconds) */
  playedDuration: number;
  /** Total track duration (seconds) */
  totalDuration: number;
  /** Context in which track was played */
  context: QueueContext;
  /** Whether this play was scrobbled */
  scrobbled: boolean;
}

/**
 * Listening statistics
 */
export interface ListeningStats {
  /** Total tracks played */
  totalPlays: number;
  /** Total listening time in seconds */
  totalListeningTime: number;
  /** Most played tracks */
  topTracks: Array<{ trackId: string; playCount: number }>;
  /** Most played artists */
  topArtists: Array<{ artistId: string; playCount: number }>;
  /** Most played albums */
  topAlbums: Array<{ albumId: string; playCount: number }>;
  /** Listening time by hour of day */
  byHour: Record<number, number>;
  /** Listening time by day of week */
  byDayOfWeek: Record<number, number>;
}
