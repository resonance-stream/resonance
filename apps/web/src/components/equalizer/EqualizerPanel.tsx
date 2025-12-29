import { useCallback, useMemo } from 'react';
import { Switch } from '../ui/Switch';
import { Button } from '../ui/Button';
import { EqualizerSlider } from './EqualizerSlider';
import { PresetSelector } from './PresetSelector';
import { useEqualizerStore } from '../../stores/equalizerStore';
import { EQ_FREQUENCY_LABELS } from '../../audio/constants';
import type { EqBandFrequency } from '../../audio/types';
import { cn } from '../../lib/utils';

interface EqualizerPanelProps {
  className?: string;
  onClose?: () => void;
}

// EQ band frequencies in order
const FREQUENCIES: EqBandFrequency[] = [
  '32', '64', '125', '250', '500', '1000', '2000', '4000', '8000', '16000',
];

/**
 * Full EQ panel with preamp, 10-band sliders, and preset management
 * Note: EQ settings are synced to AudioEngine in AudioProvider.tsx
 */
export function EqualizerPanel({ className, onClose }: EqualizerPanelProps): JSX.Element {
  const {
    settings,
    activePreset,
    setEnabled,
    setPreamp,
    setBand,
    applyPreset,
    resetToFlat,
    saveAsPreset,
    deletePreset,
    customPresets,
  } = useEqualizerStore();

  // Get all presets (built-in + custom)
  // This is already memoized in the store's getAllPresets, we just need to call it when customPresets changes
  const presets = useMemo(() => {
    // Built-in presets are constant, only custom presets change
    return [...useEqualizerStore.getState().getAllPresets()];
    // eslint-disable-next-line react-hooks/exhaustive-deps -- customPresets triggers recalculation
  }, [customPresets]);

  // Memoize handlers to prevent unnecessary re-renders
  const handlePreampChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setPreamp(Number(e.target.value));
  }, [setPreamp]);

  // Create a stable callback for band changes
  const handleBandChange = useCallback(
    (frequency: EqBandFrequency) => (gain: number) => {
      setBand(frequency, gain);
    },
    [setBand]
  );

  return (
    <div
      className={cn(
        'bg-background-secondary rounded-xl border border-border p-4',
        className
      )}
      role="region"
      aria-label="Equalizer"
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <h3 className="font-medium text-text-primary">Equalizer</h3>
          <Switch
            id="eq-toggle"
            checked={settings.enabled}
            onCheckedChange={setEnabled}
            aria-label="Enable equalizer"
          />
        </div>
        <div className="flex items-center gap-2">
          <PresetSelector
            presets={presets}
            activePreset={activePreset}
            onSelect={applyPreset}
            onSave={saveAsPreset}
            onDelete={deletePreset}
            className="w-32"
          />
          <Button
            variant="ghost"
            size="sm"
            onClick={resetToFlat}
            aria-label="Reset to flat"
          >
            Reset
          </Button>
          {onClose && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onClose}
              aria-label="Close equalizer"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </Button>
          )}
        </div>
      </div>

      {/* EQ Controls */}
      <div
        className={cn(
          'transition-opacity',
          !settings.enabled && 'opacity-50 pointer-events-none'
        )}
      >
        {/* Preamp */}
        <div className="flex items-center gap-4 mb-4 pb-4 border-b border-border">
          <span className="text-sm text-text-secondary min-w-[4rem]">Preamp</span>
          <input
            type="range"
            min={-12}
            max={12}
            step={1}
            value={settings.preamp}
            onChange={handlePreampChange}
            disabled={!settings.enabled}
            aria-label="Preamp gain"
            className={cn(
              'flex-1 h-2 rounded-full appearance-none cursor-pointer',
              'bg-background-tertiary',
              '[&::-webkit-slider-thumb]:appearance-none',
              '[&::-webkit-slider-thumb]:w-4',
              '[&::-webkit-slider-thumb]:h-4',
              '[&::-webkit-slider-thumb]:rounded-full',
              '[&::-webkit-slider-thumb]:bg-accent',
              '[&::-webkit-slider-thumb]:shadow-md',
              '[&::-webkit-slider-thumb]:transition-transform',
              '[&::-webkit-slider-thumb]:hover:scale-110',
              '[&::-moz-range-thumb]:w-4',
              '[&::-moz-range-thumb]:h-4',
              '[&::-moz-range-thumb]:rounded-full',
              '[&::-moz-range-thumb]:bg-accent',
              '[&::-moz-range-thumb]:border-0',
              'focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow'
            )}
          />
          <span className="text-sm text-text-primary min-w-[3rem] text-right">
            {settings.preamp > 0 ? `+${settings.preamp}` : settings.preamp} dB
          </span>
        </div>

        {/* EQ Bands */}
        <div className="flex justify-between gap-1">
          {FREQUENCIES.map((freq) => (
            <EqualizerSlider
              key={freq}
              frequency={freq}
              label={EQ_FREQUENCY_LABELS[freq]}
              gain={settings.bands[freq]}
              onChange={handleBandChange(freq)}
            />
          ))}
        </div>

        {/* Scale labels */}
        <div className="flex justify-between mt-2 px-2 text-xs text-text-muted">
          <span>32Hz</span>
          <span>16kHz</span>
        </div>
      </div>
    </div>
  );
}
