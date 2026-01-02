/**
 * AudioEngine - Web Audio API-based audio player
 *
 * Features:
 * - Gapless playback via prebuffering and precise scheduling
 * - Crossfade between tracks with configurable duration
 * - 10-band graphic equalizer
 * - Volume control with smooth transitions
 * - Event-based state management
 */

import {
  DEFAULT_CONFIG,
  DEFAULT_EQ_BANDS,
  DEFAULT_EQ_SETTINGS,
  EQ_FREQUENCIES,
} from './constants';
import type {
  AudioEngineConfig,
  AudioEngineEventListener,
  AudioEngineEvents,
  AudioTrack,
  CrossfadeSettings,
  EqBandFrequency,
  EqSettings,
  PlaybackState,
  SourceNodeState,
} from './types';

type EventMap = {
  [K in keyof AudioEngineEvents]: Set<AudioEngineEventListener<K>>;
};

/**
 * Web Audio API-based audio engine with support for gapless playback,
 * crossfade, and equalization.
 */
export class AudioEngine {
  // Audio context and nodes
  private audioContext: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private preampGain: GainNode | null = null;
  private eqFilters: BiquadFilterNode[] = [];
  private analyser: AnalyserNode | null = null;

  // Current source management
  private currentSource: SourceNodeState | null = null;
  private nextSource: SourceNodeState | null = null;

  // State
  private _state: PlaybackState = 'stopped';
  private _volume: number;
  private _isMuted: boolean = false;
  private _duration: number = 0;
  private _currentTrack: AudioTrack | null = null;
  private _prefetchedTrack: AudioTrack | null = null;

  // Configuration
  private config: Required<AudioEngineConfig>;
  private eqSettings: EqSettings;
  private crossfadeSettings: CrossfadeSettings;

  // Timing
  private pauseTime: number = 0;
  private timeUpdateTimer: number | null = null;

  // Event emitter
  private events: EventMap = {
    stateChange: new Set(),
    timeUpdate: new Set(),
    durationChange: new Set(),
    ended: new Set(),
    error: new Set(),
    buffering: new Set(),
    loaded: new Set(),
    prefetched: new Set(),
    prefetchNeeded: new Set(),
  };

  // Track loading state
  private loadingAbortController: AbortController | null = null;
  private prefetchAbortController: AbortController | null = null;

  // Prefetch event throttling
  private prefetchNeededEmitted: boolean = false;

  // Crossfade timeout tracking for cleanup
  private crossfadeTimeoutId: number | null = null;

  // Gapless transition scheduling
  private gaplessScheduled: boolean = false;

  constructor(config: AudioEngineConfig = {}) {
    this._volume = config.initialVolume ?? DEFAULT_CONFIG.initialVolume;
    this.config = {
      bufferSize: config.bufferSize ?? DEFAULT_CONFIG.bufferSize,
      prefetchThreshold: config.prefetchThreshold ?? DEFAULT_CONFIG.prefetchThreshold,
      initialVolume: this._volume,
      crossfade: config.crossfade ?? { enabled: false, duration: 0 },
      eq: config.eq ?? { ...DEFAULT_EQ_SETTINGS },
    };
    this.eqSettings = { ...this.config.eq };
    this.crossfadeSettings = { ...this.config.crossfade };
  }

  // ============ Initialization ============

  /**
   * Initialize the audio context and audio graph.
   * Must be called after a user gesture due to browser autoplay policies.
   */
  async initialize(): Promise<void> {
    if (this.audioContext) {
      // Already initialized, just ensure it's running
      if (this.audioContext.state === 'suspended') {
        await this.audioContext.resume();
      }
      return;
    }

    // Create audio context with fallback for older browsers
    const AudioContextClass = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    if (!AudioContextClass) {
      throw new Error('Web Audio API is not supported in this browser');
    }

    this.audioContext = new AudioContextClass();

    // Create master gain node for volume control
    this.masterGain = this.audioContext.createGain();
    this.masterGain.gain.value = this._isMuted ? 0 : this._volume;

    // Create preamp gain node for EQ
    this.preampGain = this.audioContext.createGain();
    this.preampGain.gain.value = this.dbToLinear(this.eqSettings.preamp);

    // Create analyser for future visualizations
    this.analyser = this.audioContext.createAnalyser();
    this.analyser.fftSize = 2048;

    // Create EQ filter chain
    this.createEqFilters();

    // Connect the graph: preamp -> EQ filters -> analyser -> master gain -> destination
    this.preampGain.connect(this.eqFilters[0]!);
    for (let i = 0; i < this.eqFilters.length - 1; i++) {
      this.eqFilters[i]!.connect(this.eqFilters[i + 1]!);
    }
    this.eqFilters[this.eqFilters.length - 1]!.connect(this.analyser);
    this.analyser.connect(this.masterGain);
    this.masterGain.connect(this.audioContext.destination);

    // Resume context if suspended
    if (this.audioContext.state === 'suspended') {
      await this.audioContext.resume();
    }
  }

  /**
   * Create the 10-band EQ filter chain
   */
  private createEqFilters(): void {
    if (!this.audioContext) return;

    this.eqFilters = DEFAULT_EQ_BANDS.map((band, index) => {
      const filter = this.audioContext!.createBiquadFilter();
      filter.type = band.type;
      filter.frequency.value = band.frequency;
      if (band.Q !== undefined) {
        filter.Q.value = band.Q;
      }
      // Apply saved gain if EQ is enabled
      const freqKey = EQ_FREQUENCIES[index]!.toString() as EqBandFrequency;
      const gain = this.eqSettings.enabled ? this.eqSettings.bands[freqKey] : 0;
      filter.gain.value = gain;
      return filter;
    });
  }

  // ============ Track Loading ============

  /**
   * Load a track for playback
   */
  async loadTrack(track: AudioTrack): Promise<void> {
    await this.initialize();

    // Cancel any ongoing load
    this.loadingAbortController?.abort();
    this.loadingAbortController = new AbortController();

    this.setState('loading');
    this.emit('buffering', true);

    try {
      const buffer = await this.fetchAndDecode(track.url, this.loadingAbortController.signal);

      // Clean up previous source
      this.cleanupSource(this.currentSource);

      // Create new source
      this.currentSource = this.createSourceNode(buffer, track.id);
      this._currentTrack = track;
      this._duration = buffer.duration;
      this.pauseTime = 0;
      this.prefetchNeededEmitted = false;
      this.gaplessScheduled = false;

      this.emit('durationChange', this._duration);
      this.emit('loaded');
      this.emit('buffering', false);
      this.setState('paused');
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        return; // Loading was cancelled, not an error
      }
      this.emit('buffering', false);
      this.emit('error', error instanceof Error ? error : new Error(String(error)));
      this.setState('stopped');
      throw error;
    }
  }

  /**
   * Prefetch the next track for gapless playback
   */
  async prefetchTrack(track: AudioTrack): Promise<void> {
    if (!this.audioContext) return;

    // Skip if this track is already prefetched
    if (this._prefetchedTrack?.id === track.id && this.nextSource) {
      return;
    }

    // Cancel any ongoing prefetch
    this.prefetchAbortController?.abort();
    this.prefetchAbortController = new AbortController();

    try {
      const buffer = await this.fetchAndDecode(track.url, this.prefetchAbortController.signal);

      // Clean up previous prefetched source
      this.cleanupSource(this.nextSource);

      // Create prefetched source
      this.nextSource = this.createSourceNode(buffer, track.id);
      this._prefetchedTrack = track;

      this.emit('prefetched', track.id);
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        return; // Prefetch was cancelled
      }
      // Prefetch errors are non-fatal, just log them
      console.warn('Prefetch failed:', error);
    }
  }

  /**
   * Fetch audio data and decode it
   */
  private async fetchAndDecode(url: string, signal: AbortSignal): Promise<AudioBuffer> {
    if (!this.audioContext) {
      throw new Error('AudioContext not initialized');
    }

    const response = await fetch(url, { signal });
    if (!response.ok) {
      throw new Error(`Failed to fetch audio: ${response.status} ${response.statusText}`);
    }

    const arrayBuffer = await response.arrayBuffer();
    return await this.audioContext.decodeAudioData(arrayBuffer);
  }

  /**
   * Create a source node from an audio buffer
   */
  private createSourceNode(buffer: AudioBuffer, trackId: string): SourceNodeState {
    if (!this.audioContext || !this.preampGain) {
      throw new Error('AudioContext not initialized');
    }

    const source = this.audioContext.createBufferSource();
    source.buffer = buffer;

    // Create individual gain node for crossfade control
    const gainNode = this.audioContext.createGain();
    gainNode.gain.value = 1;

    // Connect source -> gain -> preamp (which leads to EQ chain)
    source.connect(gainNode);
    gainNode.connect(this.preampGain);

    return {
      source,
      gainNode,
      buffer,
      startTime: 0,
      offsetTime: 0,
      trackId,
    };
  }

  /**
   * Clean up a source node
   */
  private cleanupSource(state: SourceNodeState | null): void {
    if (!state) return;

    try {
      state.source.stop();
    } catch {
      // Source may not have been started
    }
    state.source.disconnect();
    state.gainNode.disconnect();
  }

  // ============ Playback Control ============

  /**
   * Start or resume playback
   */
  play(): void {
    if (!this.audioContext || !this.currentSource) return;

    if (this._state === 'playing') return;

    // Resume audio context if suspended
    if (this.audioContext.state === 'suspended') {
      this.audioContext.resume().catch((error) => {
        console.error('Failed to resume AudioContext:', error);
        this.emit('error', error instanceof Error ? error : new Error(String(error)));
      });
    }

    const now = this.audioContext.currentTime;

    if (this._state === 'paused' && this.currentSource.startTime > 0) {
      // Resuming from pause - need to create new source at the paused position
      const buffer = this.currentSource.buffer;
      const trackId = this.currentSource.trackId;

      this.cleanupSource(this.currentSource);
      this.currentSource = this.createSourceNode(buffer, trackId);
      this.currentSource.offsetTime = this.pauseTime;
      this.currentSource.startTime = now;
      this.currentSource.source.start(0, this.pauseTime);

      // Set up ended handler
      this.currentSource.source.onended = () => this.handleTrackEnded();
    } else {
      // Starting fresh
      this.currentSource.startTime = now;
      this.currentSource.offsetTime = 0;
      this.currentSource.source.start(0, 0);

      // Set up ended handler
      this.currentSource.source.onended = () => this.handleTrackEnded();
    }

    this.setState('playing');
    this.startTimeUpdate();
  }

  /**
   * Pause playback
   */
  pause(): void {
    if (!this.audioContext || !this.currentSource || this._state !== 'playing') return;

    // Save current position
    this.pauseTime = this.getCurrentTime();

    // Stop the source (can't pause AudioBufferSourceNode)
    try {
      this.currentSource.source.stop();
    } catch {
      // May already be stopped
    }

    this.stopTimeUpdate();
    this.setState('paused');
  }

  /**
   * Stop playback and reset position
   */
  stop(): void {
    this.cleanupSource(this.currentSource);
    this.currentSource = null;
    this._currentTrack = null;
    this.pauseTime = 0;
    this._duration = 0;
    this.stopTimeUpdate();
    this.setState('stopped');
  }

  /**
   * Seek to a specific time in seconds
   */
  seek(time: number): void {
    if (!this.currentSource || !this.audioContext) return;

    const clampedTime = Math.max(0, Math.min(time, this._duration));
    const wasPlaying = this._state === 'playing';

    if (wasPlaying) {
      // Prevent onended from firing during seek
      this.currentSource.source.onended = null;
      // Stop current playback
      try {
        this.currentSource.source.stop();
      } catch {
        // May already be stopped
      }
    }

    // Create new source at the seek position
    const buffer = this.currentSource.buffer;
    const trackId = this.currentSource.trackId;

    this.cleanupSource(this.currentSource);
    this.currentSource = this.createSourceNode(buffer, trackId);
    this.currentSource.offsetTime = clampedTime;
    this.pauseTime = clampedTime;

    if (wasPlaying) {
      this.currentSource.startTime = this.audioContext.currentTime;
      this.currentSource.source.start(0, clampedTime);
      this.currentSource.source.onended = () => this.handleTrackEnded();
    }

    this.emit('timeUpdate', clampedTime);
  }

  /**
   * Handle track ended event
   */
  private handleTrackEnded(): void {
    // Check if this was a natural end (not a seek/stop)
    if (this._state !== 'playing') return;

    // Check if we should crossfade to next track
    if (this.nextSource && this.crossfadeSettings.enabled && this.crossfadeSettings.duration > 0) {
      this.performCrossfade();
    } else if (this.nextSource) {
      // Gapless transition without crossfade
      this.transitionToNextSource();
    } else {
      this.stopTimeUpdate();
      this.setState('stopped');
      this.emit('ended');
    }
  }

  // ============ Gapless & Crossfade ============

  /**
   * Transition to the next source for gapless playback
   */
  private transitionToNextSource(): void {
    if (!this.nextSource || !this.audioContext) return;

    this.cleanupSource(this.currentSource);
    this.currentSource = this.nextSource;
    this.nextSource = null;
    this._currentTrack = this._prefetchedTrack;
    this._prefetchedTrack = null;
    this._duration = this.currentSource.buffer.duration;
    this.pauseTime = 0;
    this.prefetchNeededEmitted = false;
    this.gaplessScheduled = false;

    // Start the new source
    this.currentSource.startTime = this.audioContext.currentTime;
    this.currentSource.offsetTime = 0;
    this.currentSource.source.start(0);
    this.currentSource.source.onended = () => this.handleTrackEnded();

    this.emit('durationChange', this._duration);
    this.emit('ended'); // Signal track change
  }

  /**
   * Perform crossfade to the next track
   */
  private performCrossfade(): void {
    if (!this.nextSource || !this.audioContext || !this.currentSource) return;

    const now = this.audioContext.currentTime;
    const fadeDuration = this.crossfadeSettings.duration;

    // Fade out current track
    this.currentSource.gainNode.gain.setValueAtTime(1, now);
    this.currentSource.gainNode.gain.linearRampToValueAtTime(0, now + fadeDuration);

    // Start and fade in next track
    this.nextSource.startTime = now;
    this.nextSource.offsetTime = 0;
    this.nextSource.gainNode.gain.setValueAtTime(0, now);
    this.nextSource.gainNode.gain.linearRampToValueAtTime(1, now + fadeDuration);
    this.nextSource.source.start(0);
    this.nextSource.source.onended = () => this.handleTrackEnded();

    // Capture references before setTimeout to avoid stale closures
    const outgoingSource = this.currentSource;
    const incomingSource = this.nextSource;
    const incomingTrack = this._prefetchedTrack;

    // Schedule cleanup of old source and state transition
    this.crossfadeTimeoutId = window.setTimeout(() => {
      this.crossfadeTimeoutId = null;

      // Guard: verify we haven't been destroyed or track changed
      if (!this.audioContext || this.currentSource !== outgoingSource) {
        // Engine destroyed or track changed - just cleanup outgoing source
        this.cleanupSource(outgoingSource);
        return;
      }

      this.cleanupSource(outgoingSource);
      this.currentSource = incomingSource;
      this.nextSource = null;
      this._currentTrack = incomingTrack;
      this._prefetchedTrack = null;
      this._duration = incomingSource?.buffer.duration ?? 0;
      this.pauseTime = 0;
      this.prefetchNeededEmitted = false;
      this.emit('durationChange', this._duration);
    }, fadeDuration * 1000);

    this.emit('ended'); // Signal track change
  }

  /**
   * Schedule gapless transition at a specific time
   */
  scheduleGaplessTransition(endTime: number): void {
    if (!this.nextSource || !this.audioContext) return;

    // Schedule the next source to start at the exact end time
    this.nextSource.startTime = endTime;
    this.nextSource.offsetTime = 0;
    this.nextSource.source.start(endTime);
    this.nextSource.source.onended = () => this.handleTrackEnded();
  }

  /**
   * Schedule gapless transition based on current track's end time
   * This provides sample-accurate timing for truly gapless playback
   */
  private scheduleGaplessTransitionFromCurrent(): void {
    if (!this.nextSource || !this.audioContext || !this.currentSource) return;

    // Calculate the precise end time of the current track
    const currentEndTime =
      this.currentSource.startTime +
      this.currentSource.buffer.duration -
      this.currentSource.offsetTime;

    // Prevent current source's onended from triggering transition
    this.currentSource.source.onended = null;

    // Capture references for the scheduled transition
    const incomingSource = this.nextSource;
    const incomingTrack = this._prefetchedTrack;

    // Schedule the next source to start at the exact end time
    incomingSource.startTime = currentEndTime;
    incomingSource.offsetTime = 0;
    incomingSource.source.start(currentEndTime);
    incomingSource.source.onended = () => this.handleTrackEnded();

    // Schedule state transition to occur when the new track starts
    const delayMs = Math.max(0, (currentEndTime - this.audioContext.currentTime) * 1000);
    setTimeout(() => {
      // Guard against engine being destroyed
      if (!this.audioContext) return;

      this.cleanupSource(this.currentSource);
      this.currentSource = incomingSource;
      this.nextSource = null;
      this._currentTrack = incomingTrack;
      this._prefetchedTrack = null;
      this._duration = incomingSource.buffer.duration;
      this.pauseTime = 0;
      this.prefetchNeededEmitted = false;
      this.gaplessScheduled = false;
      this.emit('durationChange', this._duration);
      this.emit('ended'); // Signal track change for UI update
    }, delayMs);
  }

  // ============ Volume & EQ ============

  /**
   * Set the volume (0-1)
   */
  setVolume(value: number): void {
    this._volume = Math.max(0, Math.min(1, value));
    if (this.masterGain && !this._isMuted) {
      this.masterGain.gain.value = this._volume;
    }
  }

  /**
   * Get current volume
   */
  get volume(): number {
    return this._volume;
  }

  /**
   * Set muted state
   */
  setMuted(muted: boolean): void {
    this._isMuted = muted;
    if (this.masterGain) {
      this.masterGain.gain.value = muted ? 0 : this._volume;
    }
  }

  /**
   * Get muted state
   */
  get isMuted(): boolean {
    return this._isMuted;
  }

  /**
   * Set EQ band gain
   */
  setEqBand(frequency: EqBandFrequency, gain: number): void {
    const index = EQ_FREQUENCIES.findIndex(f => f.toString() === frequency);
    if (index === -1 || !this.eqFilters[index]) return;

    const clampedGain = Math.max(-12, Math.min(12, gain));
    this.eqSettings.bands[frequency] = clampedGain;

    if (this.eqSettings.enabled) {
      this.eqFilters[index].gain.value = clampedGain;
    }
  }

  /**
   * Set EQ preamp
   */
  setPreamp(db: number): void {
    const clampedDb = Math.max(-12, Math.min(12, db));
    this.eqSettings.preamp = clampedDb;

    if (this.preampGain) {
      this.preampGain.gain.value = this.dbToLinear(clampedDb);
    }
  }

  /**
   * Enable or disable EQ
   */
  setEqEnabled(enabled: boolean): void {
    this.eqSettings.enabled = enabled;

    this.eqFilters.forEach((filter, index) => {
      const freqKey = EQ_FREQUENCIES[index]!.toString() as EqBandFrequency;
      filter.gain.value = enabled ? this.eqSettings.bands[freqKey] : 0;
    });
  }

  /**
   * Get current EQ settings
   */
  getEqSettings(): EqSettings {
    return { ...this.eqSettings };
  }

  /**
   * Apply full EQ settings
   */
  applyEqSettings(settings: EqSettings): void {
    this.eqSettings = { ...settings };
    this.setPreamp(settings.preamp);
    this.setEqEnabled(settings.enabled);

    Object.entries(settings.bands).forEach(([freq, gain]) => {
      this.setEqBand(freq as EqBandFrequency, gain);
    });
  }

  // ============ Crossfade Settings ============

  /**
   * Set crossfade settings
   */
  setCrossfade(settings: CrossfadeSettings): void {
    this.crossfadeSettings = { ...settings };
  }

  /**
   * Get crossfade settings
   */
  getCrossfadeSettings(): CrossfadeSettings {
    return { ...this.crossfadeSettings };
  }

  // ============ State & Time ============

  /**
   * Get current playback time in seconds
   */
  getCurrentTime(): number {
    if (!this.audioContext || !this.currentSource) return 0;

    if (this._state === 'playing') {
      const elapsed = this.audioContext.currentTime - this.currentSource.startTime;
      return this.currentSource.offsetTime + elapsed;
    }

    return this.pauseTime;
  }

  /**
   * Get track duration in seconds
   */
  get duration(): number {
    return this._duration;
  }

  /**
   * Get current playback state
   */
  get state(): PlaybackState {
    return this._state;
  }

  /**
   * Get current track
   */
  get currentTrack(): AudioTrack | null {
    return this._currentTrack;
  }

  /**
   * Check if a track is prefetched
   */
  get hasPrefetchedTrack(): boolean {
    return this.nextSource !== null;
  }

  /**
   * Get prefetched track info
   */
  get prefetchedTrack(): AudioTrack | null {
    return this._prefetchedTrack;
  }

  // ============ Visualizer Access ============

  /**
   * Get the AnalyserNode for audio visualization
   * Returns null if audio engine is not initialized
   */
  getAnalyser(): AnalyserNode | null {
    return this.analyser;
  }

  /**
   * Get the AudioContext for audio visualization
   * Returns null if audio engine is not initialized
   */
  getAudioContext(): AudioContext | null {
    return this.audioContext;
  }

  // ============ Events ============

  /**
   * Add an event listener
   */
  on<K extends keyof AudioEngineEvents>(
    event: K,
    listener: AudioEngineEventListener<K>
  ): void {
    this.events[event].add(listener as never);
  }

  /**
   * Remove an event listener
   */
  off<K extends keyof AudioEngineEvents>(
    event: K,
    listener: AudioEngineEventListener<K>
  ): void {
    this.events[event].delete(listener as never);
  }

  /**
   * Emit an event
   */
  private emit<K extends keyof AudioEngineEvents>(
    event: K,
    ...args: Parameters<AudioEngineEvents[K]>
  ): void {
    this.events[event].forEach((listener) => {
      try {
        (listener as (...args: unknown[]) => void)(...args);
      } catch (error) {
        console.error(`Error in ${event} listener:`, error);
      }
    });
  }

  /**
   * Set playback state and emit event
   */
  private setState(state: PlaybackState): void {
    if (this._state === state) return;
    this._state = state;
    this.emit('stateChange', state);
  }

  // ============ Time Update ============

  /**
   * Start the time update interval
   */
  private startTimeUpdate(): void {
    this.stopTimeUpdate();
    this.timeUpdateTimer = window.setInterval(() => {
      const currentTime = this.getCurrentTime();
      this.emit('timeUpdate', currentTime);

      // Check for prefetch trigger (emit only once per track)
      const remaining = this._duration - currentTime;
      if (remaining <= this.config.prefetchThreshold && remaining > 0 && !this.prefetchNeededEmitted) {
        this.prefetchNeededEmitted = true;
        this.emit('prefetchNeeded', remaining);
      }

      // Schedule gapless transition when we have a prefetched track and are close to the end
      // Schedule 2 seconds before end to allow for precise timing
      if (
        remaining <= 2 &&
        remaining > 0 &&
        this.nextSource &&
        !this.gaplessScheduled &&
        !this.crossfadeSettings.enabled
      ) {
        this.gaplessScheduled = true;
        this.scheduleGaplessTransitionFromCurrent();
      }
    }, DEFAULT_CONFIG.timeUpdateInterval);
  }

  /**
   * Stop the time update interval
   */
  private stopTimeUpdate(): void {
    if (this.timeUpdateTimer !== null) {
      clearInterval(this.timeUpdateTimer);
      this.timeUpdateTimer = null;
    }
  }

  // ============ Utilities ============

  /**
   * Convert decibels to linear gain
   */
  private dbToLinear(db: number): number {
    return Math.pow(10, db / 20);
  }

  // ============ Cleanup ============

  /**
   * Destroy the audio engine and release resources
   */
  destroy(): void {
    this.stopTimeUpdate();
    this.loadingAbortController?.abort();
    this.prefetchAbortController?.abort();

    // Cancel any pending crossfade timeout
    if (this.crossfadeTimeoutId !== null) {
      clearTimeout(this.crossfadeTimeoutId);
      this.crossfadeTimeoutId = null;
    }

    this.cleanupSource(this.currentSource);
    this.cleanupSource(this.nextSource);

    if (this.audioContext) {
      this.audioContext.close();
      this.audioContext = null;
    }

    this.masterGain = null;
    this.preampGain = null;
    this.eqFilters = [];
    this.analyser = null;
    this._state = 'stopped';
  }
}

// Export singleton instance for app-wide use
let engineInstance: AudioEngine | null = null;

export function getAudioEngine(config?: AudioEngineConfig): AudioEngine {
  if (!engineInstance) {
    engineInstance = new AudioEngine(config);
  }
  return engineInstance;
}

export function destroyAudioEngine(): void {
  if (engineInstance) {
    engineInstance.destroy();
    engineInstance = null;
  }
}
