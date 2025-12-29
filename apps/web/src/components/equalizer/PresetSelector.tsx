import { useState } from 'react';
import { Button } from '../ui/Button';
import type { EqualizerPreset } from '../../stores/equalizerStore';
import { cn } from '../../lib/utils';

interface PresetSelectorProps {
  presets: EqualizerPreset[];
  activePreset: string | null;
  onSelect: (presetId: string) => void;
  onSave: (name: string) => void;
  onDelete?: (presetId: string) => void;
  className?: string;
}

/**
 * Dropdown selector for EQ presets with save/delete functionality
 */
export function PresetSelector({
  presets,
  activePreset,
  onSelect,
  onSave,
  onDelete,
  className,
}: PresetSelectorProps): JSX.Element {
  const [isOpen, setIsOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [newPresetName, setNewPresetName] = useState('');

  const currentPreset = presets.find((p) => p.id === activePreset);
  const displayName = currentPreset?.name ?? 'Custom';

  const handleSelect = (presetId: string) => {
    onSelect(presetId);
    setIsOpen(false);
  };

  const handleSave = () => {
    if (newPresetName.trim()) {
      onSave(newPresetName.trim());
      setNewPresetName('');
      setIsSaving(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSave();
    } else if (e.key === 'Escape') {
      setIsSaving(false);
      setNewPresetName('');
    }
  };

  return (
    <div className={cn('relative', className)}>
      {/* Dropdown trigger */}
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setIsOpen(!isOpen)}
        className="w-full justify-between"
        aria-haspopup="listbox"
        aria-expanded={isOpen}
      >
        <span className="truncate">{displayName}</span>
        <svg
          className={cn('w-4 h-4 transition-transform', isOpen && 'rotate-180')}
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </Button>

      {/* Dropdown menu */}
      {isOpen && (
        <div
          className="absolute top-full left-0 right-0 mt-1 z-50 bg-background-secondary border border-border rounded-lg shadow-lg overflow-hidden"
          role="listbox"
          aria-label="Equalizer presets"
        >
          <div className="max-h-64 overflow-y-auto">
            {/* Built-in presets */}
            <div className="px-2 py-1.5 text-xs font-medium text-text-muted uppercase tracking-wide">
              Presets
            </div>
            {presets
              .filter((p) => p.isBuiltIn)
              .map((preset) => (
                <button
                  key={preset.id}
                  onClick={() => handleSelect(preset.id)}
                  className={cn(
                    'w-full px-3 py-2 text-left text-sm transition-colors',
                    'hover:bg-background-tertiary',
                    activePreset === preset.id && 'bg-accent/10 text-accent'
                  )}
                  role="option"
                  aria-selected={activePreset === preset.id}
                >
                  {preset.name}
                </button>
              ))}

            {/* Custom presets */}
            {presets.some((p) => !p.isBuiltIn) && (
              <>
                <div className="px-2 py-1.5 text-xs font-medium text-text-muted uppercase tracking-wide border-t border-border mt-1">
                  Custom Presets
                </div>
                {presets
                  .filter((p) => !p.isBuiltIn)
                  .map((preset) => (
                    <div
                      key={preset.id}
                      className={cn(
                        'flex items-center justify-between px-3 py-2 transition-colors',
                        'hover:bg-background-tertiary',
                        activePreset === preset.id && 'bg-accent/10'
                      )}
                    >
                      <button
                        onClick={() => handleSelect(preset.id)}
                        className={cn(
                          'flex-1 text-left text-sm',
                          activePreset === preset.id && 'text-accent'
                        )}
                        role="option"
                        aria-selected={activePreset === preset.id}
                      >
                        {preset.name}
                      </button>
                      {onDelete && (
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            onDelete(preset.id);
                          }}
                          className="p-1 text-text-muted hover:text-error-text transition-colors"
                          aria-label={`Delete ${preset.name}`}
                        >
                          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path
                              strokeLinecap="round"
                              strokeLinejoin="round"
                              strokeWidth={2}
                              d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                            />
                          </svg>
                        </button>
                      )}
                    </div>
                  ))}
              </>
            )}
          </div>

          {/* Save as new preset */}
          <div className="border-t border-border p-2">
            {isSaving ? (
              <div className="flex gap-2">
                <input
                  type="text"
                  value={newPresetName}
                  onChange={(e) => setNewPresetName(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="Preset name..."
                  className="flex-1 px-2 py-1 text-sm bg-background-tertiary rounded border border-border focus:outline-none focus:ring-1 focus:ring-accent"
                  autoFocus
                />
                <Button size="sm" variant="ghost" onClick={handleSave}>
                  Save
                </Button>
              </div>
            ) : (
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setIsSaving(true)}
                className="w-full justify-center"
              >
                Save as Preset
              </Button>
            )}
          </div>
        </div>
      )}

      {/* Click outside to close */}
      {isOpen && (
        <div
          className="fixed inset-0 z-40"
          onClick={() => setIsOpen(false)}
          aria-hidden="true"
        />
      )}
    </div>
  );
}
