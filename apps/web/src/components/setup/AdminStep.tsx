/**
 * AdminStep component for Resonance Setup Wizard
 *
 * Step to create the initial admin user account with password strength validation.
 */

import { useState, useMemo, type FormEvent } from 'react'
import { Loader2, Check } from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'
import { Input } from '../ui/Input'
import { useCreateInitialAdmin } from '../../hooks/useSetup'
import { setAuthToken } from '../../lib/api'
import { useAuthStore } from '../../stores/authStore'
import { cn } from '../../lib/utils'

// ============================================================================
// Types
// ============================================================================

interface AdminStepProps {
  /** Callback when step is complete */
  onNext: () => void
  /** Callback to go back to previous step */
  onBack: () => void
  /** Whether admin has already been created */
  hasAdmin: boolean
}

/** Password complexity validation result */
interface PasswordValidation {
  isValid: boolean
  hasMinLength: boolean
  hasUppercase: boolean
  hasLowercase: boolean
  hasNumber: boolean
}

// ============================================================================
// Password Validation
// ============================================================================

/**
 * Validate password complexity requirements
 */
function validatePasswordComplexity(password: string): PasswordValidation {
  const hasMinLength = password.length >= 8
  const hasUppercase = /[A-Z]/.test(password)
  const hasLowercase = /[a-z]/.test(password)
  const hasNumber = /[0-9]/.test(password)

  return {
    isValid: hasMinLength && hasUppercase && hasLowercase && hasNumber,
    hasMinLength,
    hasUppercase,
    hasLowercase,
    hasNumber,
  }
}

/**
 * Calculate password strength score (0-4) based on complexity
 */
function calculatePasswordStrength(validation: PasswordValidation): number {
  let score = 0
  if (validation.hasMinLength) score++
  if (validation.hasUppercase) score++
  if (validation.hasLowercase) score++
  if (validation.hasNumber) score++
  return score
}

/**
 * Get strength label and color based on score
 */
function getStrengthInfo(score: number): { label: string; colorClass: string } {
  switch (score) {
    case 0:
      return { label: 'Very Weak', colorClass: 'bg-error' }
    case 1:
      return { label: 'Weak', colorClass: 'bg-error' }
    case 2:
      return { label: 'Fair', colorClass: 'bg-warning' }
    case 3:
      return { label: 'Good', colorClass: 'bg-accent-light' }
    case 4:
      return { label: 'Strong', colorClass: 'bg-success' }
    default:
      return { label: '', colorClass: 'bg-background-tertiary' }
  }
}

// ============================================================================
// Sub-components
// ============================================================================

interface RequirementProps {
  met: boolean
  label: string
}

/**
 * Single password requirement indicator
 */
function Requirement({ met, label }: RequirementProps): JSX.Element {
  return (
    <span
      className={cn(
        'flex items-center gap-1 text-xs transition-colors',
        met ? 'text-success-text' : 'text-text-muted'
      )}
    >
      {met ? <Check size={12} /> : <span className="w-3 text-center">-</span>}
      {label}
    </span>
  )
}

interface PasswordStrengthIndicatorProps {
  validation: PasswordValidation
  showRequirements: boolean
}

/**
 * Password strength meter with requirements checklist
 */
function PasswordStrengthIndicator({
  validation,
  showRequirements,
}: PasswordStrengthIndicatorProps): JSX.Element | null {
  if (!showRequirements) return null

  const strength = calculatePasswordStrength(validation)
  const { label, colorClass } = getStrengthInfo(strength)

  return (
    <div className="mt-3 p-3 rounded-lg bg-background-tertiary/50">
      {/* Strength bar */}
      <div className="flex items-center gap-2 mb-3">
        <div className="flex-1 h-1.5 bg-background-tertiary rounded-full overflow-hidden flex gap-0.5">
          {[0, 1, 2, 3].map((index) => (
            <div
              key={index}
              className={cn(
                'flex-1 h-full transition-colors duration-200',
                index < strength ? colorClass : 'bg-background-tertiary'
              )}
            />
          ))}
        </div>
        <span className="text-xs font-medium text-text-muted w-16 text-right">
          {label}
        </span>
      </div>

      {/* Requirements checklist */}
      <p className="text-xs text-text-muted mb-2">Password requirements:</p>
      <div className="grid grid-cols-2 gap-2">
        <Requirement met={validation.hasMinLength} label="8+ characters" />
        <Requirement met={validation.hasUppercase} label="Uppercase (A-Z)" />
        <Requirement met={validation.hasLowercase} label="Lowercase (a-z)" />
        <Requirement met={validation.hasNumber} label="Number (0-9)" />
      </div>
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

/**
 * Admin account creation step of the setup wizard.
 *
 * Features:
 * - Username validation (min 3 characters)
 * - Email validation
 * - Password strength indicator with real-time validation
 * - Password confirmation matching
 * - Stores auth tokens in authStore on success
 */
export function AdminStep({ onNext, onBack, hasAdmin }: AdminStepProps): JSX.Element {
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [validationError, setValidationError] = useState<string | null>(null)

  const createAdmin = useCreateInitialAdmin()

  // Memoize password validation to avoid recalculating on every render
  const passwordValidation = useMemo(
    () => validatePasswordComplexity(password),
    [password]
  )

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

    // Validate username
    if (username.trim().length < 3) {
      setValidationError('Username must be at least 3 characters')
      return
    }

    // Validate email format (basic check - browser also validates)
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
    if (!emailRegex.test(email)) {
      setValidationError('Please enter a valid email address')
      return
    }

    // Validate password complexity
    if (!passwordValidation.isValid) {
      const errors: string[] = []
      if (!passwordValidation.hasMinLength) errors.push('at least 8 characters')
      if (!passwordValidation.hasUppercase) errors.push('one uppercase letter')
      if (!passwordValidation.hasLowercase) errors.push('one lowercase letter')
      if (!passwordValidation.hasNumber) errors.push('one number')
      setValidationError(`Password must contain: ${errors.join(', ')}`)
      return
    }

    // Validate passwords match
    if (password !== confirmPassword) {
      setValidationError('Passwords do not match')
      return
    }

    try {
      const result = await createAdmin.mutateAsync({
        username: username.trim(),
        email: email.trim(),
        password,
      })

      // Set auth token for subsequent API calls during setup
      setAuthToken(result.accessToken)

      // Store tokens in auth store for persistence
      // The auth store will handle token refresh and user fetching
      useAuthStore.setState({
        accessToken: result.accessToken,
        refreshToken: result.refreshToken,
        status: 'authenticated',
        error: null,
      })

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
              htmlFor="admin-username"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Username
            </label>
            <Input
              id="admin-username"
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="admin"
              required
              minLength={3}
              autoComplete="username"
              disabled={createAdmin.isPending}
            />
            <p className="mt-1 text-xs text-text-muted">
              Minimum 3 characters
            </p>
          </div>

          {/* Email */}
          <div>
            <label
              htmlFor="admin-email"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Email
            </label>
            <Input
              id="admin-email"
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
              htmlFor="admin-password"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Password
            </label>
            <Input
              id="admin-password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Create a strong password"
              required
              minLength={8}
              autoComplete="new-password"
              disabled={createAdmin.isPending}
            />
            {/* Password strength indicator */}
            <PasswordStrengthIndicator
              validation={passwordValidation}
              showRequirements={password.length > 0}
            />
          </div>

          {/* Confirm Password */}
          <div>
            <label
              htmlFor="admin-confirm-password"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Confirm Password
            </label>
            <Input
              id="admin-confirm-password"
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              placeholder="Re-enter your password"
              required
              minLength={8}
              autoComplete="new-password"
              disabled={createAdmin.isPending}
            />
            {/* Password match indicator */}
            {confirmPassword.length > 0 && (
              <p
                className={cn(
                  'mt-1 text-xs flex items-center gap-1',
                  password === confirmPassword
                    ? 'text-success-text'
                    : 'text-error-text'
                )}
              >
                {password === confirmPassword ? (
                  <>
                    <Check size={12} />
                    Passwords match
                  </>
                ) : (
                  'Passwords do not match'
                )}
              </p>
            )}
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
