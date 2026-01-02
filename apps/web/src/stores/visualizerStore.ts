import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface VisualizerState {
  /** Whether the visualizer is enabled */
  showVisualizer: boolean;

  /** Whether the fullscreen player is open */
  isFullscreen: boolean;

  /** Actions */
  setShowVisualizer: (show: boolean) => void;
  toggleVisualizer: () => void;
  setFullscreen: (fullscreen: boolean) => void;
  openFullscreen: () => void;
  closeFullscreen: () => void;
}

export const useVisualizerStore = create<VisualizerState>()(
  persist(
    (set) => ({
      showVisualizer: false,
      isFullscreen: false,

      setShowVisualizer: (show) => set({ showVisualizer: show }),

      toggleVisualizer: () =>
        set((state) => ({ showVisualizer: !state.showVisualizer })),

      setFullscreen: (fullscreen) => set({ isFullscreen: fullscreen }),

      openFullscreen: () => set({ isFullscreen: true }),

      closeFullscreen: () => set({ isFullscreen: false }),
    }),
    {
      name: 'resonance-visualizer',
      version: 1,
      partialize: (state) => ({
        showVisualizer: state.showVisualizer,
        // Don't persist isFullscreen - always start closed
      }),
    }
  )
);
