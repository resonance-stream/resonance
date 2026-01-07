/**
 * Library path settings component
 *
 * Provides UI for managing user-specific music library paths:
 * - View configured library paths
 * - Add new library paths with optional labels
 * - Remove library paths
 * - Set primary library path
 * - Edit path labels
 */

import { useState, useCallback, type FormEvent } from 'react'
import { FolderOpen, Star, Trash2, Pencil, Plus, Check, X } from 'lucide-react'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../ui/Card'
import { Button } from '../ui/Button'
import { Input } from '../ui/Input'
import { Badge } from '../ui/Badge'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/Dialog'
import {
  useUserLibraryPaths,
  useAddLibraryPath,
  useRemoveLibraryPath,
  useSetPrimaryLibraryPath,
  useUpdateLibraryPath,
} from '../../hooks/useLibraryPaths'
import type { UserLibraryPath } from '@resonance/shared-types'

interface EditingState {
  id: string
  label: string
}

interface RemoveConfirmState {
  id: string
  path: string
  label: string | null
}

export function LibraryPathSettings(): JSX.Element {
  // Local form state
  const [newPath, setNewPath] = useState('')
  const [newLabel, setNewLabel] = useState('')
  const [showAddForm, setShowAddForm] = useState(false)
  const [editing, setEditing] = useState<EditingState | null>(null)
  const [removeConfirm, setRemoveConfirm] = useState<RemoveConfirmState | null>(null)
  const [mutationError, setMutationError] = useState<string | null>(null)

  // Query and mutations
  const { data: libraryPaths, isLoading } = useUserLibraryPaths()

  const addLibraryPath = useAddLibraryPath({
    onSuccess: () => {
      setNewPath('')
      setNewLabel('')
      setShowAddForm(false)
      setMutationError(null)
    },
    onError: (error) => {
      setMutationError(error.message || 'Failed to add library path')
    },
  })

  const removeLibraryPath = useRemoveLibraryPath({
    onSuccess: () => {
      setRemoveConfirm(null)
      setMutationError(null)
    },
    onError: (error) => {
      setMutationError(error.message || 'Failed to remove library path')
    },
  })

  const setPrimaryLibraryPath = useSetPrimaryLibraryPath({
    onError: (error) => {
      setMutationError(error.message || 'Failed to set primary library path')
    },
  })

  const updateLibraryPath = useUpdateLibraryPath({
    onSuccess: () => {
      setEditing(null)
      setMutationError(null)
    },
    onError: (error) => {
      setMutationError(error.message || 'Failed to update library path')
    },
  })

  // Handlers
  const handleAddPath = useCallback(
    (e: FormEvent) => {
      e.preventDefault()
      const trimmedPath = newPath.trim()
      if (!trimmedPath) return

      setMutationError(null)
      addLibraryPath.mutate({
        path: trimmedPath,
        label: newLabel.trim() || undefined,
      })
    },
    [newPath, newLabel, addLibraryPath]
  )

  const handleRemovePath = useCallback(
    (path: UserLibraryPath) => {
      setRemoveConfirm({
        id: path.id,
        path: path.path,
        label: path.label,
      })
    },
    []
  )

  const confirmRemove = useCallback(() => {
    if (!removeConfirm) return
    setMutationError(null)
    removeLibraryPath.mutate(removeConfirm.id)
  }, [removeConfirm, removeLibraryPath])

  const handleSetPrimary = useCallback(
    (id: string) => {
      setMutationError(null)
      setPrimaryLibraryPath.mutate(id)
    },
    [setPrimaryLibraryPath]
  )

  const handleStartEdit = useCallback((path: UserLibraryPath) => {
    setEditing({
      id: path.id,
      label: path.label || '',
    })
  }, [])

  const handleCancelEdit = useCallback(() => {
    setEditing(null)
  }, [])

  const handleSaveEdit = useCallback(() => {
    if (!editing) return
    setMutationError(null)
    updateLibraryPath.mutate({
      id: editing.id,
      label: editing.label.trim(),
    })
  }, [editing, updateLibraryPath])

  const handleCancelAdd = useCallback(() => {
    setShowAddForm(false)
    setNewPath('')
    setNewLabel('')
    setMutationError(null)
  }, [])

  const isPending =
    addLibraryPath.isPending ||
    removeLibraryPath.isPending ||
    setPrimaryLibraryPath.isPending ||
    updateLibraryPath.isPending

  return (
    <>
      <Card padding="lg">
        <CardHeader>
          <CardTitle>Library Paths</CardTitle>
          <CardDescription>
            Configure paths to your personal music library
          </CardDescription>
        </CardHeader>
        <CardContent className="mt-4 space-y-4">
          {/* Loading state */}
          {isLoading && (
            <div className="flex items-center gap-2 text-sm text-text-muted" role="status">
              <span className="animate-pulse">Loading library paths...</span>
            </div>
          )}

          {/* Mutation error */}
          {mutationError && (
            <div className="text-sm text-error-text bg-error/10 rounded-lg px-3 py-2" role="alert">
              {mutationError}
            </div>
          )}

          {/* Library paths list */}
          {libraryPaths && libraryPaths.length > 0 && (
            <div className="space-y-2">
              {libraryPaths.map((path) => (
                <div
                  key={path.id}
                  className="flex items-center gap-3 p-3 rounded-lg bg-background-tertiary/50 border border-white/5"
                >
                  <FolderOpen className="h-5 w-5 text-text-muted flex-shrink-0" aria-hidden="true" />

                  <div className="flex-1 min-w-0">
                    {editing?.id === path.id ? (
                      <div className="flex items-center gap-2">
                        <Input
                          value={editing.label}
                          onChange={(e) => setEditing({ ...editing, label: e.target.value })}
                          placeholder="Enter label"
                          className="text-sm h-8"
                          autoFocus
                          disabled={updateLibraryPath.isPending}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                              e.preventDefault()
                              handleSaveEdit()
                            } else if (e.key === 'Escape') {
                              handleCancelEdit()
                            }
                          }}
                        />
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={handleSaveEdit}
                          disabled={updateLibraryPath.isPending}
                          aria-label="Save label"
                        >
                          <Check className="h-4 w-4 text-success" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={handleCancelEdit}
                          disabled={updateLibraryPath.isPending}
                          aria-label="Cancel editing"
                        >
                          <X className="h-4 w-4" />
                        </Button>
                      </div>
                    ) : (
                      <>
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
                      </>
                    )}
                  </div>

                  {!editing && (
                    <div className="flex items-center gap-1 flex-shrink-0">
                      {!path.isPrimary && (
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => handleSetPrimary(path.id)}
                          disabled={isPending}
                          aria-label="Set as primary"
                          title="Set as primary"
                        >
                          <Star className="h-4 w-4" />
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleStartEdit(path)}
                        disabled={isPending}
                        aria-label="Edit label"
                        title="Edit label"
                      >
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleRemovePath(path)}
                        disabled={isPending || path.isPrimary}
                        aria-label="Remove path"
                        title={path.isPrimary ? 'Cannot remove primary path' : 'Remove path'}
                        className="text-error-text hover:bg-error/20"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          {/* Empty state */}
          {!isLoading && (!libraryPaths || libraryPaths.length === 0) && !showAddForm && (
            <div className="text-center py-8 text-text-muted">
              <FolderOpen className="h-12 w-12 mx-auto mb-3 opacity-50" aria-hidden="true" />
              <p className="text-sm">No library paths configured</p>
              <p className="text-xs mt-1">Add a path to your music library to get started</p>
            </div>
          )}

          {/* Add path form */}
          {showAddForm ? (
            <form onSubmit={handleAddPath} className="space-y-3 p-3 rounded-lg bg-background-tertiary/30 border border-white/5">
              <div className="space-y-2">
                <label htmlFor="new-path" className="text-sm font-medium text-text-primary">
                  Path
                </label>
                <Input
                  id="new-path"
                  type="text"
                  value={newPath}
                  onChange={(e) => setNewPath(e.target.value)}
                  placeholder="/path/to/music/library"
                  autoFocus
                  disabled={addLibraryPath.isPending}
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="new-label" className="text-sm font-medium text-text-primary">
                  Label <span className="text-text-muted font-normal">(optional)</span>
                </label>
                <Input
                  id="new-label"
                  type="text"
                  value={newLabel}
                  onChange={(e) => setNewLabel(e.target.value)}
                  placeholder="e.g., NAS Music, Local Collection"
                  disabled={addLibraryPath.isPending}
                />
              </div>
              <div className="flex gap-2 pt-2">
                <Button
                  type="submit"
                  variant="primary"
                  size="sm"
                  disabled={!newPath.trim() || addLibraryPath.isPending}
                >
                  {addLibraryPath.isPending ? 'Adding...' : 'Add Path'}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={handleCancelAdd}
                  disabled={addLibraryPath.isPending}
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
              disabled={isPending}
              className="w-full justify-center"
            >
              <Plus className="h-4 w-4 mr-2" />
              Add Library Path
            </Button>
          )}
        </CardContent>
      </Card>

      {/* Remove confirmation dialog */}
      <Dialog
        open={removeConfirm !== null}
        onOpenChange={(open) => !open && setRemoveConfirm(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Remove Library Path</DialogTitle>
            <DialogDescription>
              Are you sure you want to remove this library path?
            </DialogDescription>
          </DialogHeader>

          <div className="py-4">
            <div className="rounded-lg bg-background-tertiary/50 p-3 space-y-1">
              {removeConfirm?.label && (
                <p className="font-medium text-text-primary">{removeConfirm.label}</p>
              )}
              <p className="text-sm text-text-muted break-all">{removeConfirm?.path}</p>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="ghost"
              onClick={() => setRemoveConfirm(null)}
              disabled={removeLibraryPath.isPending}
            >
              Cancel
            </Button>
            <Button
              variant="ghost"
              className="text-error-text hover:bg-error/20"
              onClick={confirmRemove}
              disabled={removeLibraryPath.isPending}
            >
              {removeLibraryPath.isPending ? 'Removing...' : 'Remove'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
