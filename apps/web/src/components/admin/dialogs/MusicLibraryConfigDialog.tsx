/**
 * MusicLibraryConfigDialog - Dialog for configuring the system music library path
 *
 * This is the system-level default path, different from user-specific library paths.
 */

import { useState, useEffect } from 'react'
import { Folder, AlertTriangle } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../../ui/Dialog'
import { Button } from '../../ui/Button'
import { Input } from '../../ui/Input'
import { Switch } from '../../ui/Switch'
import type { SystemSettingInfo } from '../../../types/systemSettings'

interface MusicLibraryConfig {
  path: string
}

interface MusicLibraryConfigDialogProps {
  setting: SystemSettingInfo | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onSave: (data: {
    service: 'MUSIC_LIBRARY'
    enabled: boolean
    config: string
  }) => void
  isSaving: boolean
}

export function MusicLibraryConfigDialog({
  setting,
  open,
  onOpenChange,
  onSave,
  isSaving,
}: MusicLibraryConfigDialogProps): JSX.Element | null {
  const [enabled, setEnabled] = useState(false)
  const [path, setPath] = useState('')

  // Reset form when dialog opens with new setting
  useEffect(() => {
    if (setting && open) {
      setEnabled(setting.enabled)
      const config = setting.config as Partial<MusicLibraryConfig>
      setPath(config.path ?? '/music')
    }
  }, [setting, open])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    const config: MusicLibraryConfig = {
      path: path.trim(),
    }

    onSave({
      service: 'MUSIC_LIBRARY',
      enabled,
      config: JSON.stringify(config),
    })
  }

  if (!setting) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="rounded-lg bg-accent-dark/20 p-2">
              <Folder className="h-5 w-5 text-accent" />
            </div>
            <div>
              <DialogTitle>Configure Music Library</DialogTitle>
              <DialogDescription>
                System default path to your music files
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-6 mt-4">
          {/* Enabled Toggle */}
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-text-primary">Enabled</p>
              <p className="text-xs text-text-secondary">
                Enable the music library
              </p>
            </div>
            <Switch checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {/* System Path Info */}
          <div className="rounded-lg bg-background-tertiary/50 p-4">
            <div className="flex items-start gap-2">
              <AlertTriangle className="h-4 w-4 text-yellow-400 mt-0.5 shrink-0" />
              <div>
                <p className="text-sm font-medium text-text-primary">System Default Path</p>
                <p className="text-xs text-text-tertiary mt-1">
                  This is the system-level library path used as the default for all users.
                  Individual users can configure additional library paths in their settings.
                </p>
              </div>
            </div>
          </div>

          {/* Path Field */}
          <div>
            <label
              htmlFor="library-path"
              className="block text-sm font-medium text-text-primary mb-1"
            >
              Library Path <span className="text-error">*</span>
            </label>
            <Input
              id="library-path"
              type="text"
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder="/path/to/music"
              required
            />
            <p className="text-xs text-text-tertiary mt-1">
              The absolute path where your music files are stored
            </p>
          </div>

          {/* Path Examples */}
          <div className="space-y-2">
            <p className="text-xs font-medium text-text-secondary">Common Paths</p>
            <div className="grid grid-cols-1 gap-1">
              {[
                { path: '/music', desc: 'Docker default' },
                { path: '/mnt/media/music', desc: 'Mounted volume' },
                { path: '/data/music', desc: 'Data directory' },
              ].map((example) => (
                <button
                  key={example.path}
                  type="button"
                  onClick={() => setPath(example.path)}
                  className="text-left text-xs px-2 py-1.5 rounded bg-background-secondary hover:bg-background-tertiary transition-colors"
                >
                  <span className="font-mono text-accent">{example.path}</span>
                  <span className="text-text-tertiary ml-2">- {example.desc}</span>
                </button>
              ))}
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={() => onOpenChange(false)}
              disabled={isSaving}
            >
              Cancel
            </Button>
            <Button type="submit" variant="accent" disabled={isSaving}>
              {isSaving ? 'Saving...' : 'Save Changes'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
