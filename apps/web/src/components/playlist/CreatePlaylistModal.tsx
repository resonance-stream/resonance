/**
 * Create Playlist Modal Component
 *
 * Modal dialog for creating smart playlists with rule-based filtering.
 * Features:
 * - Name, description, and public toggle
 * - Embedded SmartPlaylistRuleBuilder for rules
 * - Form validation with error display
 * - Loading, error, and success states
 * - Auto-navigate to new playlist on success
 */

import { useState, useRef, useEffect, useCallback, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { Button } from '../ui/Button'
import { Input } from '../ui/Input'
import { Switch } from '../ui/Switch'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/Dialog'
import { SmartPlaylistRuleBuilder } from './SmartPlaylistRuleBuilder'
import { useCreatePlaylist } from '../../hooks/usePlaylistMutations'
import {
  type SmartRule,
  type SmartMatchMode,
  type SmartPlaylistSort,
  type SmartRuleField,
  type ValidationError,
  VALIDATION_LIMITS,
  validateSmartPlaylistForm,
  formStateToInput,
  createDefaultRule,
} from '../../types/playlist'

// ============================================================================
// Types
// ============================================================================

interface CreatePlaylistModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

// ============================================================================
// Main Component
// ============================================================================

export function CreatePlaylistModal({
  open,
  onOpenChange,
}: CreatePlaylistModalProps): JSX.Element {
  const navigate = useNavigate()

  // Form state
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [isPublic, setIsPublic] = useState(false)
  const [matchMode, setMatchMode] = useState<SmartMatchMode>('all')
  const [rules, setRules] = useState<SmartRule[]>([createDefaultRule()])
  const [limit, setLimit] = useState<number>(VALIDATION_LIMITS.DEFAULT_PLAYLIST_LIMIT)
  const [sortField, setSortField] = useState<SmartPlaylistSort>('random')
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('desc')

  // UI state
  const [validationError, setValidationError] = useState<ValidationError | null>(null)
  const [success, setSuccess] = useState(false)

  // Ref for auto-close timeout cleanup
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Create playlist mutation
  const createPlaylist = useCreatePlaylist({
    onSuccess: (data) => {
      setSuccess(true)
      // Auto-navigate after brief success display
      timeoutRef.current = setTimeout(() => {
        onOpenChange(false)
        navigate(`/playlist/${data.id}`)
      }, 1500)
    },
  })

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [])

  // Reset form to initial state (stable callback without mutation dependency)
  const resetForm = useCallback(() => {
    setName('')
    setDescription('')
    setIsPublic(false)
    setMatchMode('all')
    setRules([createDefaultRule()])
    setLimit(VALIDATION_LIMITS.DEFAULT_PLAYLIST_LIMIT)
    setSortField('random')
    setSortDirection('desc')
    setValidationError(null)
    setSuccess(false)
  }, [])

  // Handle modal open/close
  const handleOpenChange = useCallback(
    (newOpen: boolean) => {
      if (!newOpen) {
        // Clear any pending auto-close timer
        if (timeoutRef.current) {
          clearTimeout(timeoutRef.current)
          timeoutRef.current = null
        }
        resetForm()
        createPlaylist.reset()
      }
      onOpenChange(newOpen)
    },
    [onOpenChange, resetForm, createPlaylist]
  )

  // Handle form submission
  const handleSubmit = useCallback(
    async (e: FormEvent) => {
      e.preventDefault()
      setValidationError(null)

      // Build form state for validation
      // Note: 'random' is a valid sort option but not a SmartRuleField
      const formState = {
        name,
        description,
        isPublic,
        matchMode,
        rules,
        limit,
        sortBy: sortField === 'random' ? null : (sortField as SmartRuleField),
        sortOrder: sortDirection,
      }

      // Validate form
      const error = validateSmartPlaylistForm(formState)
      if (error) {
        setValidationError(error)
        return
      }

      // Submit
      const input = formStateToInput(formState)
      createPlaylist.mutate(input)
    },
    [name, description, isPublic, matchMode, rules, limit, sortField, sortDirection, createPlaylist]
  )

  // Determine if form is submitting or succeeded
  const isSubmitting = createPlaylist.isPending
  const isDisabled = isSubmitting || success

  // Get error message (validation or mutation error)
  const errorMessage = validationError?.message ?? (createPlaylist.isError ? 'Failed to create playlist. Please try again.' : null)

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Create Smart Playlist</DialogTitle>
          <DialogDescription>
            Define rules to automatically populate your playlist based on track metadata,
            audio features, and more.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-6 mt-4">
          {/* Basic Info Section */}
          <div className="space-y-4">
            {/* Playlist Name */}
            <div className="space-y-2">
              <label htmlFor="playlist-name" className="text-sm font-medium text-text-primary">
                Playlist Name <span className="text-error-text">*</span>
              </label>
              <Input
                id="playlist-name"
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Enter playlist name"
                maxLength={VALIDATION_LIMITS.MAX_NAME_LENGTH}
                disabled={isDisabled}
                aria-required="true"
                aria-invalid={validationError?.field === 'name' || undefined}
                aria-describedby={errorMessage ? 'form-error' : undefined}
              />
              <p className="text-xs text-text-muted">
                {name.length}/{VALIDATION_LIMITS.MAX_NAME_LENGTH}
              </p>
            </div>

            {/* Description */}
            <div className="space-y-2">
              <label htmlFor="playlist-description" className="text-sm font-medium text-text-primary">
                Description
              </label>
              <textarea
                id="playlist-description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Add an optional description"
                maxLength={VALIDATION_LIMITS.MAX_DESCRIPTION_LENGTH}
                disabled={isDisabled}
                rows={2}
                className="w-full px-3 py-2 text-sm rounded-lg border border-white/10 bg-background-secondary text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-accent resize-none disabled:opacity-50"
                aria-invalid={validationError?.field === 'description' || undefined}
              />
              <p className="text-xs text-text-muted">
                {description.length}/{VALIDATION_LIMITS.MAX_DESCRIPTION_LENGTH}
              </p>
            </div>

            {/* Public Toggle */}
            <div className="flex items-center gap-3">
              <Switch
                id="playlist-public"
                checked={isPublic}
                onCheckedChange={setIsPublic}
                disabled={isDisabled}
              />
              <label htmlFor="playlist-public" className="text-sm text-text-primary cursor-pointer">
                Make playlist public
              </label>
            </div>
          </div>

          {/* Divider */}
          <div className="border-t border-white/10" />

          {/* Rules Section */}
          <div className="space-y-2">
            <h3 className="text-sm font-medium text-text-primary">Rules</h3>
            <SmartPlaylistRuleBuilder
              rules={rules}
              onRulesChange={setRules}
              matchMode={matchMode}
              onMatchModeChange={setMatchMode}
              limit={limit}
              onLimitChange={setLimit}
              sortField={sortField}
              onSortFieldChange={setSortField}
              sortDirection={sortDirection}
              onSortDirectionChange={setSortDirection}
              disabled={isDisabled}
            />
          </div>

          {/* Error Display */}
          {errorMessage && (
            <div
              id="form-error"
              role="alert"
              className="text-sm text-error-text bg-error/10 rounded-lg px-3 py-2"
            >
              {errorMessage}
            </div>
          )}

          {/* Success Display */}
          {success && (
            <div
              role="status"
              aria-live="polite"
              className="text-sm text-success bg-success/10 rounded-lg px-3 py-2"
            >
              Playlist created successfully! Redirecting...
            </div>
          )}

          {/* Footer */}
          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={() => handleOpenChange(false)}
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              variant="accent"
              disabled={isDisabled}
            >
              {isSubmitting ? 'Creating...' : 'Create Playlist'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

// Export component types for testing
export type { CreatePlaylistModalProps }
