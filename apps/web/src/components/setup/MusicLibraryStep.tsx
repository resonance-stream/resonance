/**
 * MusicLibraryStep component for Resonance Setup Wizard
 *
 * Step to configure user music library paths.
 * This step is required and cannot be skipped.
 */

import { useState, useCallback, type FormEvent } from 'react'
import { Loader2, FolderOpen, Plus, Trash2, Star, AlertCircle } from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'
import { Input } from '../ui/Input'
import { Badge } from '../ui/Badge'
import {
  useUserLibraryPaths,
  useAddLibraryPath,
  useRemoveLibraryPath,
  useSetPrimaryLibraryPath,
} from '../../hooks/useLibraryPaths'

interface MusicLibraryStepProps {
  /** Callback when step is complete */
  onNext: () => void
  /** Callback to go back to previous step */
  onBack: () => void
}

interface PathInput {
  path: string
  label: string
}

/**
 * Validates a music library path
 */
function validatePath(path: string): string | null {
  const trimmed = path.trim()

  if (!trimmed) {
    return 'Path is required'
  }

  // Must be an absolute path
  if (!trimmed.startsWith('/')) {
    return 'Path must be absolute (starting with /)'
  }

  // Basic security checks
  if (trimmed.includes('..')) {
    return 'Path cannot contain ".."'
  }

  // Check for reasonable path characters
  if (!/^[\w\s/\-_.]+$/.test(trimmed)) {
    return 'Path contains invalid characters'
  }

  return null
}

/**
 * Music library configuration step of the setup wizard.
 * Allows users to configure one or more library paths.
 * This step is required and cannot be skipped.
 */
export function MusicLibraryStep({ onNext, onBack }: MusicLibraryStepProps): JSX.Element {
  // Form state for adding new paths
  const [newPath, setNewPath] = useState<PathInput>({ path: '/music', label: '' })
  const [validationError, setValidationError] = useState<string | null>(null)
  const [showAddForm, setShowAddForm] = useState(false)

  // Query and mutations
  const { data: libraryPaths = [], isLoading: isLoadingPaths } = useUserLibraryPaths()

  const addLibraryPath = useAddLibraryPath({
    onSuccess: () => {
      setNewPath({ path: '', label: '' })
      setShowAddForm(false)
      setValidationError(null)
    },
    onError: (error) => {
      setValidationError(error.message || 'Failed to add library path')
    },
  })

  const removeLibraryPath = useRemoveLibraryPath({
    onError: (error) => {
      setValidationError(error.message || 'Failed to remove library path')
    },
  })

  const setPrimaryLibraryPath = useSetPrimaryLibraryPath({
    onError: (error) => {
      setValidationError(error.message || 'Failed to set primary library path')
    },
  })

  // Check if there are any paths configured
  const hasPaths = libraryPaths.length > 0

  /**
   * Handle adding the first path (inline form)
   */
  const handleAddFirstPath = useCallback(
    async (e: FormEvent<HTMLFormElement>) => {
      e.preventDefault()
      setValidationError(null)

      const pathError = validatePath(newPath.path)
      if (pathError) {
        setValidationError(pathError)
        return
      }

      try {
        await addLibraryPath.mutateAsync({
          path: newPath.path.trim(),
          label: newPath.label.trim() || undefined,
        })
      } catch {
        // Error handled by mutation
      }
    },
    [newPath, addLibraryPath]
  )

  /**
   * Handle adding additional paths
   */
  const handleAddAdditionalPath = useCallback(
    async (e: FormEvent<HTMLFormElement>) => {
      e.preventDefault()
      setValidationError(null)

      const pathError = validatePath(newPath.path)
      if (pathError) {
        setValidationError(pathError)
        return
      }

      // Check for duplicate paths
      const isDuplicate = libraryPaths.some(
        (p) => p.path.toLowerCase() === newPath.path.trim().toLowerCase()
      )
      if (isDuplicate) {
        setValidationError('This path has already been added')
        return
      }

      try {
        await addLibraryPath.mutateAsync({
          path: newPath.path.trim(),
          label: newPath.label.trim() || undefined,
        })
      } catch {
        // Error handled by mutation
      }
    },
    [newPath, addLibraryPath, libraryPaths]
  )

  /**
   * Handle removing a path
   */
  const handleRemovePath = useCallback(
    (id: string) => {
      setValidationError(null)
      removeLibraryPath.mutate(id)
    },
    [removeLibraryPath]
  )

  /**
   * Handle setting a path as primary
   */
  const handleSetPrimary = useCallback(
    (id: string) => {
      setValidationError(null)
      setPrimaryLibraryPath.mutate(id)
    },
    [setPrimaryLibraryPath]
  )

  /**
   * Handle continuing to next step
   */
  const handleContinue = useCallback(() => {
    if (!hasPaths) {
      setValidationError('Please add at least one library path to continue')
      return
    }
    onNext()
  }, [hasPaths, onNext])

  /**
   * Cancel adding additional path
   */
  const handleCancelAdd = useCallback(() => {
    setShowAddForm(false)
    setNewPath({ path: '', label: '' })
    setValidationError(null)
  }, [])

  const isLoading =
    addLibraryPath.isPending ||
    removeLibraryPath.isPending ||
    setPrimaryLibraryPath.isPending

  return (
    <Card variant="glass" padding="lg">
      <div className="space-y-6">
        {/* Header */}
        <div className="text-center">
          <h1 className="font-display text-2xl text-text-primary">
            Music Library
          </h1>
          <p className="mt-2 text-text-secondary">
            Configure the paths to your music collection.
          </p>
        </div>

        {/* Error message */}
        {validationError && (
          <div className="rounded-lg bg-error/20 border border-error/30 p-4 text-error-text flex items-start gap-3">
            <AlertCircle className="h-5 w-5 flex-shrink-0 mt-0.5" />
            <p className="text-sm">{validationError}</p>
          </div>
        )}

        {/* Loading paths */}
        {isLoadingPaths && (
          <div className="flex items-center justify-center gap-2 py-8 text-text-muted">
            <Loader2 className="h-5 w-5 animate-spin" />
            <span>Loading library paths...</span>
          </div>
        )}

        {/* No paths yet - show initial form */}
        {!isLoadingPaths && !hasPaths && (
          <form onSubmit={handleAddFirstPath} className="space-y-4">
            {/* Path input */}
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
                value={newPath.path}
                onChange={(e) => setNewPath({ ...newPath, path: e.target.value })}
                placeholder="/music"
                required
                disabled={isLoading}
              />
              <p className="mt-1 text-xs text-text-muted">
                The absolute path where your music files are stored
              </p>
            </div>

            {/* Label input */}
            <div>
              <label
                htmlFor="libraryLabel"
                className="mb-2 block text-sm font-medium text-text-secondary"
              >
                Label <span className="text-text-muted font-normal">(optional)</span>
              </label>
              <Input
                id="libraryLabel"
                type="text"
                value={newPath.label}
                onChange={(e) => setNewPath({ ...newPath, label: e.target.value })}
                placeholder="e.g., Main Collection, NAS Music"
                disabled={isLoading}
              />
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

            {/* Submit button */}
            <Button
              type="submit"
              disabled={isLoading || !newPath.path.trim()}
              className="w-full"
            >
              {addLibraryPath.isPending ? (
                <span className="flex items-center justify-center gap-2">
                  <Loader2 className="h-5 w-5 animate-spin" />
                  Adding...
                </span>
              ) : (
                <span className="flex items-center justify-center gap-2">
                  <Plus className="h-5 w-5" />
                  Add Library Path
                </span>
              )}
            </Button>
          </form>
        )}

        {/* Paths list */}
        {!isLoadingPaths && hasPaths && (
          <div className="space-y-4">
            {/* Existing paths */}
            <div className="space-y-2">
              {libraryPaths.map((path) => (
                <div
                  key={path.id}
                  className="flex items-center gap-3 p-3 rounded-lg bg-background-tertiary/50 border border-white/5"
                >
                  <FolderOpen className="h-5 w-5 text-text-muted flex-shrink-0" aria-hidden="true" />

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      {path.label && (
                        <span className="font-medium text-text-primary">
                          {path.label}
                        </span>
                      )}
                      {path.isPrimary && (
                        <Badge variant="success" size="sm">
                          Primary
                        </Badge>
                      )}
                    </div>
                    <p className="text-sm text-text-muted truncate" title={path.path}>
                      {path.path}
                    </p>
                  </div>

                  <div className="flex items-center gap-1 flex-shrink-0">
                    {!path.isPrimary && (
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleSetPrimary(path.id)}
                        disabled={isLoading}
                        aria-label="Set as primary"
                        title="Set as primary"
                      >
                        <Star className="h-4 w-4" />
                      </Button>
                    )}
                    {!path.isPrimary && libraryPaths.length > 1 && (
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleRemovePath(path.id)}
                        disabled={isLoading}
                        aria-label="Remove path"
                        title="Remove path"
                        className="text-error-text hover:bg-error/20"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {/* Add additional path form */}
            {showAddForm ? (
              <form
                onSubmit={handleAddAdditionalPath}
                className="space-y-3 p-3 rounded-lg bg-background-tertiary/30 border border-white/5"
              >
                <div className="space-y-2">
                  <label htmlFor="additionalPath" className="text-sm font-medium text-text-primary">
                    Path
                  </label>
                  <Input
                    id="additionalPath"
                    type="text"
                    value={newPath.path}
                    onChange={(e) => setNewPath({ ...newPath, path: e.target.value })}
                    placeholder="/path/to/music/library"
                    autoFocus
                    disabled={isLoading}
                  />
                </div>
                <div className="space-y-2">
                  <label htmlFor="additionalLabel" className="text-sm font-medium text-text-primary">
                    Label <span className="text-text-muted font-normal">(optional)</span>
                  </label>
                  <Input
                    id="additionalLabel"
                    type="text"
                    value={newPath.label}
                    onChange={(e) => setNewPath({ ...newPath, label: e.target.value })}
                    placeholder="e.g., NAS Music, External Drive"
                    disabled={isLoading}
                  />
                </div>
                <div className="flex gap-2 pt-2">
                  <Button
                    type="submit"
                    variant="primary"
                    size="sm"
                    disabled={!newPath.path.trim() || isLoading}
                  >
                    {addLibraryPath.isPending ? 'Adding...' : 'Add Path'}
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={handleCancelAdd}
                    disabled={isLoading}
                  >
                    Cancel
                  </Button>
                </div>
              </form>
            ) : (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowAddForm(true)}
                disabled={isLoading}
                className="w-full justify-center"
              >
                <Plus className="h-4 w-4 mr-2" />
                Add Another Path
              </Button>
            )}
          </div>
        )}

        {/* Navigation buttons */}
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
            type="button"
            onClick={handleContinue}
            disabled={isLoading || !hasPaths}
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

        {/* Info box - required notice */}
        <div className="rounded-lg bg-primary/10 border border-primary/20 p-4">
          <p className="text-sm text-text-secondary">
            <strong className="text-text-primary">Required:</strong> At least one library path
            is needed to start using Resonance. You can add more paths later in the settings.
          </p>
        </div>
      </div>
    </Card>
  )
}

export default MusicLibraryStep
