/**
 * LibraryStep component for Resonance Setup Wizard
 *
 * Step to configure the music library path.
 */

import { useState, type FormEvent } from 'react'
import { Loader2 } from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'
import { Input } from '../ui/Input'
import { useUpdateSystemSetting } from '../../hooks/useSetup'

interface LibraryStepProps {
  /** Callback when step is complete */
  onNext: () => void
  /** Callback to go back to previous step */
  onBack: () => void
}

/**
 * Music library configuration step of the setup wizard.
 */
export function LibraryStep({ onNext, onBack }: LibraryStepProps): JSX.Element {
  const [libraryPath, setLibraryPath] = useState('/music')
  const [validationError, setValidationError] = useState<string | null>(null)

  const updateSetting = useUpdateSystemSetting()

  async function handleSubmit(e: FormEvent<HTMLFormElement>): Promise<void> {
    e.preventDefault()
    setValidationError(null)

    if (!libraryPath.trim()) {
      setValidationError('Please enter a library path')
      return
    }

    // Basic path validation
    if (!libraryPath.startsWith('/')) {
      setValidationError('Path must be an absolute path (starting with /)')
      return
    }

    try {
      await updateSetting.mutateAsync({
        service: 'MUSIC_LIBRARY',
        enabled: true,
        config: JSON.stringify({ path: libraryPath }),
      })
      onNext()
    } catch {
      // Error handled by mutation
    }
  }

  const isLoading = updateSetting.isPending

  return (
    <Card variant="glass" padding="lg">
      <div className="space-y-6">
        {/* Header */}
        <div className="text-center">
          <h1 className="font-display text-2xl text-text-primary">
            Music Library
          </h1>
          <p className="mt-2 text-text-secondary">
            Set the path to your music collection.
          </p>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Error message */}
          {(validationError || updateSetting.error) && (
            <div className="rounded-lg bg-error/20 border border-error/30 p-4 text-error-text">
              <p className="text-sm">
                {validationError || (updateSetting.error as Error)?.message || 'An error occurred'}
              </p>
            </div>
          )}

          {/* Library Path */}
          <div>
            <label
              htmlFor="libraryPath"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Library Path
            </label>
            <Input
              id="libraryPath"
              type="text"
              value={libraryPath}
              onChange={(e) => setLibraryPath(e.target.value)}
              placeholder="/music"
              required
              disabled={isLoading}
            />
            <p className="mt-1 text-xs text-text-muted">
              The path where your music files are stored (inside the container)
            </p>
          </div>

          {/* Docker hint */}
          <div className="rounded-lg bg-background-tertiary/50 border border-white/5 p-4">
            <p className="text-sm text-text-muted">
              <strong className="text-text-secondary">Docker users:</strong> This should match the
              container path from your volume mount. For example, if your docker-compose has:
            </p>
            <pre className="mt-2 text-xs bg-background-secondary/50 p-2 rounded overflow-x-auto">
              volumes:{'\n'}  - /path/on/host:/music
            </pre>
            <p className="mt-2 text-sm text-text-muted">
              Then enter <code className="text-accent-light">/music</code> here.
            </p>
          </div>

          {/* Actions */}
          <div className="flex gap-3 pt-2">
            <Button
              type="button"
              variant="secondary"
              onClick={onBack}
              disabled={isLoading}
              className="flex-1"
            >
              Back
            </Button>
            <Button
              type="submit"
              disabled={isLoading}
              className="flex-1"
            >
              {isLoading ? (
                <span className="flex items-center justify-center gap-2">
                  <Loader2 className="h-5 w-5 animate-spin" />
                  Saving...
                </span>
              ) : (
                'Continue'
              )}
            </Button>
          </div>
        </form>

        {/* Info box */}
        <div className="rounded-lg bg-primary/10 border border-primary/20 p-4">
          <p className="text-sm text-text-secondary">
            <strong className="text-text-primary">Required:</strong> The music library path is
            needed to scan and serve your music files. Make sure the path is accessible from
            the Resonance container.
          </p>
        </div>
      </div>
    </Card>
  )
}

export default LibraryStep
