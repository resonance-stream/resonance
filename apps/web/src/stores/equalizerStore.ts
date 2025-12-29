import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { EqBandFrequency, EqSettings } from '../audio/types';
import { DEFAULT_EQ_SETTINGS, EQ_PRESETS } from '../audio/constants';

export interface EqualizerPreset {
  id: string;
  name: string;
  isBuiltIn: boolean;
  preamp: number;
  bands: Record<EqBandFrequency, number>;
}

interface EqualizerState {
  // Current EQ settings
  settings: EqSettings;

  // Currently selected preset (null if custom)
  activePreset: string | null;

  // User-created presets (built-in presets are in constants.ts)
  customPresets: EqualizerPreset[];

  // Actions
  setEnabled: (enabled: boolean) => void;
  setPreamp: (preamp: number) => void;
  setBand: (frequency: EqBandFrequency, gain: number) => void;
  setAllBands: (bands: Record<EqBandFrequency, number>) => void;
  applyPreset: (presetId: string) => void;
  resetToFlat: () => void;

  // Custom preset management
  saveAsPreset: (name: string) => EqualizerPreset;
  deletePreset: (presetId: string) => void;
  updatePreset: (presetId: string, name: string) => void;

  // Get all presets (built-in + custom)
  getAllPresets: () => EqualizerPreset[];

  // Get settings for syncing to engine
  getSettings: () => EqSettings;
}

// Convert built-in presets to EqualizerPreset format
const builtInPresets: EqualizerPreset[] = Object.entries(EQ_PRESETS).map(
  ([id, preset]) => ({
    id,
    name: preset.name,
    isBuiltIn: true,
    preamp: preset.preamp,
    bands: { ...preset.bands },
  })
);

const clampGain = (gain: number): number => Math.max(-12, Math.min(12, gain));

export const useEqualizerStore = create<EqualizerState>()(
  persist(
    (set, get) => ({
      settings: { ...DEFAULT_EQ_SETTINGS },
      activePreset: 'flat',
      customPresets: [],

      setEnabled: (enabled) =>
        set((state) => ({
          settings: { ...state.settings, enabled },
        })),

      setPreamp: (preamp) =>
        set((state) => ({
          settings: { ...state.settings, preamp: clampGain(preamp) },
          activePreset: null, // Custom settings
        })),

      setBand: (frequency, gain) =>
        set((state) => ({
          settings: {
            ...state.settings,
            bands: {
              ...state.settings.bands,
              [frequency]: clampGain(gain),
            },
          },
          activePreset: null, // Custom settings
        })),

      setAllBands: (bands) =>
        set((state) => ({
          settings: {
            ...state.settings,
            bands: Object.fromEntries(
              Object.entries(bands).map(([freq, gain]) => [freq, clampGain(gain)])
            ) as Record<EqBandFrequency, number>,
          },
          activePreset: null,
        })),

      applyPreset: (presetId) => {
        const allPresets = get().getAllPresets();
        const preset = allPresets.find((p) => p.id === presetId);

        if (!preset) return;

        set((state) => ({
          settings: {
            ...state.settings,
            preamp: preset.preamp,
            bands: { ...preset.bands },
          },
          activePreset: presetId,
        }));
      },

      resetToFlat: () => {
        set((state) => ({
          settings: {
            ...state.settings,
            preamp: 0,
            bands: { ...DEFAULT_EQ_SETTINGS.bands },
          },
          activePreset: 'flat',
        }));
      },

      saveAsPreset: (name) => {
        const { settings, customPresets } = get();
        const newPreset: EqualizerPreset = {
          id: `custom-${Date.now()}`,
          name,
          isBuiltIn: false,
          preamp: settings.preamp,
          bands: { ...settings.bands },
        };

        set({
          customPresets: [...customPresets, newPreset],
          activePreset: newPreset.id,
        });

        return newPreset;
      },

      deletePreset: (presetId) => {
        set((state) => ({
          customPresets: state.customPresets.filter((p) => p.id !== presetId),
          activePreset: state.activePreset === presetId ? null : state.activePreset,
        }));
      },

      updatePreset: (presetId, name) => {
        set((state) => ({
          customPresets: state.customPresets.map((p) =>
            p.id === presetId ? { ...p, name } : p
          ),
        }));
      },

      getAllPresets: () => {
        const { customPresets } = get();
        return [...builtInPresets, ...customPresets];
      },

      getSettings: () => get().settings,
    }),
    {
      name: 'resonance-equalizer',
      version: 1,
      partialize: (state) => ({
        settings: state.settings,
        activePreset: state.activePreset,
        customPresets: state.customPresets,
      }),
    }
  )
);
