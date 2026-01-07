/**
 * AdminStep component for Resonance Setup Wizard
 *
 * Step to create the initial admin user account.
 */

import { useState, type FormEvent } from 'react'
import { Loader2 } from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'
import { Input } from '../ui/Input'
import { useCreateInitialAdmin } from '../../hooks/useSetup'
import { setAuthToken } from '../../lib/api'

interface AdminStepProps {
  /** Callback when step is complete */
  onNext: () => void
  /** Callback to go back to previous step */
  onBack: () => void
  /** Whether admin has already been created */
  hasAdmin: boolean
}

/**
 * Admin account creation step of the setup wizard.
 */
export function AdminStep({ onNext, onBack, hasAdmin }: AdminStepProps): JSX.Element {
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [validationError, setValidationError] = useState<string | null>(null)

  const createAdmin = useCreateInitialAdmin()

  // If admin already exists, show skip option
  if (hasAdmin) {
    return (
      <Card variant="glass" padding="lg">
        <div className="space-y-6">
          <div className="text-center">
            <h1 className="font-display text-2xl text-text-primary">
              Admin Account
            </h1>
            <p className="mt-2 text-text-secondary">
              An admin account already exists. You can proceed to the next step.
            </p>
          </div>

          <div className="rounded-lg bg-accent-light/10 border border-accent-light/20 p-4">
            <p className="text-sm text-text-secondary">
              <strong className="text-accent-light">Already configured!</strong>{' '}
              An administrator account has been created for this instance.
            </p>
          </div>

          <div className="flex gap-3">
            <Button variant="secondary" onClick={onBack} className="flex-1">
              Back
            </Button>
            <Button onClick={onNext} className="flex-1">
              Continue
            </Button>
          </div>
        </div>
      </Card>
    )
  }

  async function handleSubmit(e: FormEvent<HTMLFormElement>): Promise<void> {
    e.preventDefault()
    setValidationError(null)

    // Validate passwords match
    if (password !== confirmPassword) {
      setValidationError('Passwords do not match')
      return
    }

    // Validate password length
    if (password.length < 8) {
      setValidationError('Password must be at least 8 characters')
      return
    }

    try {
      const result = await createAdmin.mutateAsync({
        username,
        email,
        password,
      })

      // Set auth token for subsequent API calls during setup
      // Full authentication will happen when user logs in after setup completes
      setAuthToken(result.accessToken)

      onNext()
    } catch {
      // Error is handled by mutation
    }
  }

  return (
    <Card variant="glass" padding="lg">
      <div className="space-y-6">
        {/* Header */}
        <div className="text-center">
          <h1 className="font-display text-2xl text-text-primary">
            Create Admin Account
          </h1>
          <p className="mt-2 text-text-secondary">
            Set up your administrator account to manage Resonance.
          </p>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Error message */}
          {(validationError || createAdmin.error) && (
            <div className="rounded-lg bg-error/20 border border-error/30 p-4 text-error-text">
              <p className="text-sm">
                {validationError || (createAdmin.error as Error)?.message || 'An error occurred'}
              </p>
            </div>
          )}

          {/* Username */}
          <div>
            <label
              htmlFor="username"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Username
            </label>
            <Input
              id="username"
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="admin"
              required
              autoComplete="username"
              disabled={createAdmin.isPending}
            />
          </div>

          {/* Email */}
          <div>
            <label
              htmlFor="email"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Email
            </label>
            <Input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="admin@example.com"
              required
              autoComplete="email"
              disabled={createAdmin.isPending}
            />
          </div>

          {/* Password */}
          <div>
            <label
              htmlFor="password"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Password
            </label>
            <Input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Minimum 8 characters"
              required
              autoComplete="new-password"
              disabled={createAdmin.isPending}
            />
          </div>

          {/* Confirm Password */}
          <div>
            <label
              htmlFor="confirmPassword"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Confirm Password
            </label>
            <Input
              id="confirmPassword"
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              placeholder="Re-enter your password"
              required
              autoComplete="new-password"
              disabled={createAdmin.isPending}
            />
          </div>

          {/* Actions */}
          <div className="flex gap-3 pt-2">
            <Button
              type="button"
              variant="secondary"
              onClick={onBack}
              disabled={createAdmin.isPending}
              className="flex-1"
            >
              Back
            </Button>
            <Button
              type="submit"
              disabled={createAdmin.isPending}
              className="flex-1"
            >
              {createAdmin.isPending ? (
                <span className="flex items-center justify-center gap-2">
                  <Loader2 className="h-5 w-5 animate-spin" />
                  Creating...
                </span>
              ) : (
                'Create Admin'
              )}
            </Button>
          </div>
        </form>
      </div>
    </Card>
  )
}

export default AdminStep
