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

interface ChangeEmailModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentEmail: string
}

export function ChangeEmailModal({ open, onOpenChange, currentEmail }: ChangeEmailModalProps): JSX.Element {
  const updateEmail = useAuthStore((s) => s.updateEmail)

  const [newEmail, setNewEmail] = useState('')
  const [password, setPassword] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<AuthError | null>(null)
  const [success, setSuccess] = useState(false)

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
    setNewEmail('')
    setPassword('')
    setError(null)
    setSuccess(false)
  }

  const handleOpenChange = (newOpen: boolean): void => {
    if (!newOpen) {
      resetForm()
    }
    onOpenChange(newOpen)
  }

  const isValidEmail = (email: string): boolean => {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
    return emailRegex.test(email)
  }

  const validateForm = (): string | null => {
    if (!newEmail) {
      return 'New email is required'
    }
    if (!isValidEmail(newEmail)) {
      return 'Please enter a valid email address'
    }
    if (newEmail.toLowerCase() === currentEmail.toLowerCase()) {
      return 'New email must be different from current email'
    }
    if (!password) {
      return 'Password is required to change email'
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

    setIsLoading(true)
    try {
      await updateEmail(newEmail, password)
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
          <DialogTitle>Change Email</DialogTitle>
          <DialogDescription>
            Enter your new email address. You'll need to verify your password.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 mt-4">
          {/* Read-only display field with muted label styling */}
          <div className="space-y-2">
            <label className="text-sm font-medium text-text-muted">
              Current Email
            </label>
            <p className="text-text-primary">{currentEmail}</p>
          </div>

          <div className="space-y-2">
            <label htmlFor="new-email" className="text-sm font-medium text-text-primary">
              New Email
            </label>
            <Input
              id="new-email"
              type="email"
              value={newEmail}
              onChange={(e) => setNewEmail(e.target.value)}
              placeholder="Enter new email address"
              autoComplete="email"
              disabled={isLoading || success}
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="email-password" className="text-sm font-medium text-text-primary">
              Password
            </label>
            <Input
              id="email-password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Enter your password to confirm"
              autoComplete="current-password"
              disabled={isLoading || success}
            />
          </div>

          {error && (
            <div role="alert" className="text-sm text-error-text bg-error/10 rounded-lg px-3 py-2">
              {error.message}
            </div>
          )}

          {success && (
            <div role="status" aria-live="polite" className="text-sm text-success bg-success/10 rounded-lg px-3 py-2">
              Email updated successfully!
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
              {isLoading ? 'Updating...' : 'Update Email'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
