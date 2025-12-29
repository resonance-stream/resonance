import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export interface PlaybackSettings {
  // Crossfade
  crossfadeEnabled: boolean
  crossfadeDuration: number // 0-12 seconds

  // Gapless
  gaplessEnabled: boolean

  // Volume normalization
  normalizeVolume: boolean
}

export interface AudioQualitySettings {
  quality: 'auto' | 'low' | 'normal' | 'high' | 'lossless'
  downloadQuality: 'low' | 'normal' | 'high' | 'lossless'
}

interface SettingsState {
  playback: PlaybackSettings
  audioQuality: AudioQualitySettings

  // Playback actions
  setCrossfadeEnabled: (enabled: boolean) => void
  setCrossfadeDuration: (duration: number) => void
  setGaplessEnabled: (enabled: boolean) => void
  setNormalizeVolume: (enabled: boolean) => void

  // Audio quality actions
  setAudioQuality: (quality: AudioQualitySettings['quality']) => void
  setDownloadQuality: (quality: AudioQualitySettings['downloadQuality']) => void

  // Reset
  resetToDefaults: () => void
}

const DEFAULT_PLAYBACK: PlaybackSettings = {
  crossfadeEnabled: false,
  crossfadeDuration: 3,
  gaplessEnabled: true,
  normalizeVolume: false,
}

const DEFAULT_AUDIO_QUALITY: AudioQualitySettings = {
  quality: 'high',
  downloadQuality: 'high',
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      playback: { ...DEFAULT_PLAYBACK },
      audioQuality: { ...DEFAULT_AUDIO_QUALITY },

      // Playback actions
      setCrossfadeEnabled: (enabled) =>
        set((state) => ({
          playback: { ...state.playback, crossfadeEnabled: enabled },
        })),

      setCrossfadeDuration: (duration) =>
        set((state) => ({
          playback: {
            ...state.playback,
            crossfadeDuration: Math.max(0, Math.min(12, duration)),
          },
        })),

      setGaplessEnabled: (enabled) =>
        set((state) => ({
          playback: { ...state.playback, gaplessEnabled: enabled },
        })),

      setNormalizeVolume: (enabled) =>
        set((state) => ({
          playback: { ...state.playback, normalizeVolume: enabled },
        })),

      // Audio quality actions
      setAudioQuality: (quality) =>
        set((state) => ({
          audioQuality: { ...state.audioQuality, quality },
        })),

      setDownloadQuality: (quality) =>
        set((state) => ({
          audioQuality: { ...state.audioQuality, downloadQuality: quality },
        })),

      // Reset
      resetToDefaults: () =>
        set({
          playback: { ...DEFAULT_PLAYBACK },
          audioQuality: { ...DEFAULT_AUDIO_QUALITY },
        }),
    }),
    {
      name: 'resonance-settings',
    }
  )
)
