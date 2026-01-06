import { useState, useRef, useEffect, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
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

interface DeleteAccountModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function DeleteAccountModal({ open, onOpenChange }: DeleteAccountModalProps): JSX.Element {
  const deleteAccount = useAuthStore((s) => s.deleteAccount)
  const navigate = useNavigate()

  const [password, setPassword] = useState('')
  const [confirmText, setConfirmText] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<AuthError | null>(null)

  // Ref for auto-redirect timeout cleanup
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
    setPassword('')
    setConfirmText('')
    setError(null)
  }

  const handleOpenChange = (newOpen: boolean): void => {
    if (!newOpen) {
      // Clear any pending auto-redirect timer
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
        timeoutRef.current = null
      }
      resetForm()
    }
    onOpenChange(newOpen)
  }

  const validateForm = (): string | null => {
    if (!password) {
      return 'Password is required'
    }
    if (confirmText !== 'DELETE') {
      return 'Please type DELETE to confirm'
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
      await deleteAccount(password)
      // Account deleted successfully - redirect to login
      navigate('/login', { replace: true })
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

  const isFormValid = password.length > 0 && confirmText === 'DELETE'

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="text-error-text">Delete Account</DialogTitle>
          <DialogDescription>
            This action is permanent and cannot be undone. All your data will be deleted.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 mt-4">
          {/* Warning Message */}
          <div className="rounded-lg bg-error/10 border border-error/30 p-4">
            <p className="text-sm text-error-text font-medium">Warning</p>
            <ul className="mt-2 text-sm text-text-secondary space-y-1">
              <li>Your account will be permanently deleted</li>
              <li>All your playlists, preferences, and data will be lost</li>
              <li>This action cannot be undone</li>
            </ul>
          </div>

          <div className="space-y-2">
            <label htmlFor="delete-password" className="text-sm font-medium text-text-primary">
              Enter your password to confirm
            </label>
            <Input
              id="delete-password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Enter your password"
              autoComplete="current-password"
              disabled={isLoading}
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="delete-confirm" className="text-sm font-medium text-text-primary">
              Type <span className="font-mono text-error-text">DELETE</span> to confirm
            </label>
            <Input
              id="delete-confirm"
              type="text"
              value={confirmText}
              onChange={(e) => setConfirmText(e.target.value)}
              placeholder="Type DELETE"
              autoComplete="off"
              disabled={isLoading}
            />
          </div>

          {error && (
            <div role="alert" className="text-sm text-error-text bg-error/10 rounded-lg px-3 py-2">
              {error.message}
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
              variant="ghost"
              className="text-error-text hover:bg-error/20"
              disabled={isLoading || !isFormValid}
            >
              {isLoading ? 'Deleting...' : 'Delete Account'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
