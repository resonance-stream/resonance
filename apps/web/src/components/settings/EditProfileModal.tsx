import { useState, useEffect, useRef, useMemo, type FormEvent } from 'react'
import { Button } from '../ui/Button'
import { Input } from '../ui/Input'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/Dialog'
import { useAuthStore } from '../../stores/authStore'
import { isAuthError, type AuthError } from '../../types/auth'

interface EditProfileModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function EditProfileModal({ open, onOpenChange }: EditProfileModalProps): JSX.Element {
  const user = useAuthStore((s) => s.user)
  const updateProfile = useAuthStore((s) => s.updateProfile)

  const [displayName, setDisplayName] = useState('')
  const [avatarUrl, setAvatarUrl] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<AuthError | null>(null)
  const [success, setSuccess] = useState(false)
  const [avatarLoadError, setAvatarLoadError] = useState(false)

  // Compute sanitized URL for preview - only http/https URLs pass through
  // This is separate from isValidUrl to make security validation explicit for static analyzers
  const sanitizedPreviewUrl = useMemo(() => {
    if (!avatarUrl) return null
    const trimmed = avatarUrl.trim()
    if (!trimmed) return null
    try {
      const parsed = new URL(trimmed)
      // Only allow safe protocols to prevent XSS
      if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
        return trimmed
      }
    } catch {
      // Invalid URL - don't render preview
    }
    return null
  }, [avatarUrl])

  // Ref for auto-close timeout cleanup
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [])

  // Initialize form with current user values when modal opens
  useEffect(() => {
    if (open && user) {
      setDisplayName(user.displayName || '')
      setAvatarUrl(user.avatarUrl || '')
      setAvatarLoadError(false)
    }
  }, [open, user])

  // Reset avatar error when URL changes
  useEffect(() => {
    setAvatarLoadError(false)
  }, [avatarUrl])

  const resetForm = (): void => {
    setError(null)
    setSuccess(false)
    setAvatarLoadError(false)
  }

  const handleOpenChange = (newOpen: boolean): void => {
    if (!newOpen) {
      // Clear any pending auto-close timer to prevent race condition
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
        timeoutRef.current = null
      }
      resetForm()
    }
    onOpenChange(newOpen)
  }

  const isValidUrl = (url: string): boolean => {
    if (!url) return true // Empty is valid (optional field)
    const trimmed = url.trim()
    if (!trimmed) return true
    try {
      const parsed = new URL(trimmed)
      // Only allow http/https protocols to prevent XSS via javascript: URLs
      return parsed.protocol === 'http:' || parsed.protocol === 'https:'
    } catch {
      return false
    }
  }

  const validateForm = (): string | null => {
    const trimmedName = displayName.trim()
    const trimmedUrl = avatarUrl.trim()

    // Check if at least one field has changed
    const nameChanged = trimmedName !== (user?.displayName || '')
    const avatarChanged = trimmedUrl !== (user?.avatarUrl || '')

    if (!nameChanged && !avatarChanged) {
      return 'No changes to save'
    }

    if (trimmedName && (trimmedName.length < 1 || trimmedName.length > 100)) {
      return 'Display name must be between 1 and 100 characters'
    }

    if (trimmedUrl && !isValidUrl(trimmedUrl)) {
      return 'Please enter a valid URL for the avatar'
    }

    return null
  }

  const handleSubmit = async (e: FormEvent): Promise<void> => {
    e.preventDefault()
    setError(null)

    const validationError = validateForm()
    if (validationError) {
      setError({ code: 'VALIDATION_ERROR', message: validationError })
      return
    }

    const trimmedName = displayName.trim()
    const trimmedUrl = avatarUrl.trim()

    // Determine what changed
    const nameChanged = trimmedName !== (user?.displayName || '')
    const avatarChanged = trimmedUrl !== (user?.avatarUrl || '')

    setIsLoading(true)
    try {
      await updateProfile(
        nameChanged ? trimmedName : undefined,
        avatarChanged ? (trimmedUrl || null) : undefined
      )
      setSuccess(true)
      // Auto-close after success
      timeoutRef.current = setTimeout(() => {
        handleOpenChange(false)
      }, 2000)
    } catch (err) {
      if (isAuthError(err)) {
        setError(err)
      } else {
        setError({ code: 'UNKNOWN_ERROR', message: 'An unexpected error occurred' })
      }
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Edit Profile</DialogTitle>
          <DialogDescription>
            Update your display name and avatar.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 mt-4">
          <div className="space-y-2">
            <label htmlFor="display-name" className="text-sm font-medium text-text-primary">
              Display Name
            </label>
            <Input
              id="display-name"
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="Enter your display name"
              maxLength={100}
              disabled={isLoading || success}
            />
            <p className="text-xs text-text-muted">
              This is how you'll appear to others.
            </p>
          </div>

          <div className="space-y-2">
            <label htmlFor="avatar-url" className="text-sm font-medium text-text-primary">
              Avatar URL
            </label>
            <Input
              id="avatar-url"
              type="url"
              value={avatarUrl}
              onChange={(e) => setAvatarUrl(e.target.value)}
              placeholder="https://example.com/avatar.jpg"
              disabled={isLoading || success}
            />
            <p className="text-xs text-text-muted">
              Optional. Enter a URL to an image for your avatar.
            </p>
          </div>

          {/* Avatar preview - uses sanitizedPreviewUrl which is explicitly validated */}
          {sanitizedPreviewUrl && (
            <div className="flex items-center gap-3">
              <span className="text-sm text-text-muted">Preview:</span>
              {!avatarLoadError ? (
                <img
                  src={sanitizedPreviewUrl}
                  alt="Avatar preview"
                  className="w-10 h-10 rounded-full object-cover bg-background-tertiary"
                  onError={() => setAvatarLoadError(true)}
                />
              ) : (
                <span className="text-xs text-text-muted">Failed to load image</span>
              )}
            </div>
          )}

          {error && (
            <div role="alert" className="text-sm text-error-text bg-error/10 rounded-lg px-3 py-2">
              {error.message}
            </div>
          )}

          {success && (
            <div role="status" aria-live="polite" className="text-sm text-success bg-success/10 rounded-lg px-3 py-2">
              Profile updated successfully!
            </div>
          )}

          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={() => handleOpenChange(false)}
              disabled={isLoading}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              variant="accent"
              disabled={isLoading || success}
            >
              {isLoading ? 'Saving...' : 'Save Changes'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
