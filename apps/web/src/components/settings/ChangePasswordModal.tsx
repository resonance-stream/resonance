import { useState, useRef, useEffect, type FormEvent } from 'react'
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

interface ChangePasswordModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function ChangePasswordModal({ open, onOpenChange }: ChangePasswordModalProps): JSX.Element {
  const changePassword = useAuthStore((s) => s.changePassword)

  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<AuthError | null>(null)
  const [success, setSuccess] = useState<{ sessionsInvalidated: number } | null>(null)

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

  const resetForm = (): void => {
    setCurrentPassword('')
    setNewPassword('')
    setConfirmPassword('')
    setError(null)
    setSuccess(null)
  }

  const handleOpenChange = (newOpen: boolean): void => {
    if (!newOpen) {
      resetForm()
    }
    onOpenChange(newOpen)
  }

  const validateForm = (): string | null => {
    if (!currentPassword) {
      return 'Current password is required'
    }
    if (!newPassword) {
      return 'New password is required'
    }
    if (newPassword.length < 8) {
      return 'New password must be at least 8 characters'
    }
    if (newPassword !== confirmPassword) {
      return 'Passwords do not match'
    }
    if (currentPassword === newPassword) {
      return 'New password must be different from current password'
    }
    return null
  }

  const handleSubmit = async (e: FormEvent): Promise<void> => {
    e.preventDefault()
    setError(null)
    setSuccess(null)

    const validationError = validateForm()
    if (validationError) {
      setError({ code: 'VALIDATION_ERROR', message: validationError })
      return
    }

    setIsLoading(true)
    try {
      const result = await changePassword(currentPassword, newPassword)
      setSuccess(result)
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
          <DialogTitle>Change Password</DialogTitle>
          <DialogDescription>
            Enter your current password and choose a new one.
            This will log you out of all other devices.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 mt-4">
          <div className="space-y-2">
            <label htmlFor="current-password" className="text-sm font-medium text-text-primary">
              Current Password
            </label>
            <Input
              id="current-password"
              type="password"
              value={currentPassword}
              onChange={(e) => setCurrentPassword(e.target.value)}
              placeholder="Enter current password"
              autoComplete="current-password"
              disabled={isLoading || !!success}
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="new-password" className="text-sm font-medium text-text-primary">
              New Password
            </label>
            <Input
              id="new-password"
              type="password"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              placeholder="Enter new password (min. 8 characters)"
              autoComplete="new-password"
              disabled={isLoading || !!success}
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="confirm-password" className="text-sm font-medium text-text-primary">
              Confirm New Password
            </label>
            <Input
              id="confirm-password"
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              placeholder="Confirm new password"
              autoComplete="new-password"
              disabled={isLoading || !!success}
            />
          </div>

          {error && (
            <div role="alert" className="text-sm text-error-text bg-error/10 rounded-lg px-3 py-2">
              {error.message}
            </div>
          )}

          {success && (
            <div role="status" aria-live="polite" className="text-sm text-success bg-success/10 rounded-lg px-3 py-2">
              Password changed successfully!
              {success.sessionsInvalidated > 0 && (
                <span className="block text-text-muted mt-1">
                  {success.sessionsInvalidated} other session{success.sessionsInvalidated === 1 ? '' : 's'} logged out.
                </span>
              )}
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
              disabled={isLoading || !!success}
            >
              {isLoading ? 'Changing...' : 'Change Password'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
