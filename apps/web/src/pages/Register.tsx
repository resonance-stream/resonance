import { useState, useMemo, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { Loader2, Check } from 'lucide-react'
import { useAuthStore } from '../stores/authStore'
import { Button } from '../components/ui/Button'
import { Input } from '../components/ui/Input'
import { Card } from '../components/ui/Card'
import { cn } from '../lib/utils'

/** Password complexity validation result */
interface PasswordValidation {
  isValid: boolean
  hasMinLength: boolean
  hasUppercase: boolean
  hasLowercase: boolean
  hasNumber: boolean
}

/**
 * Validate password complexity
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

interface RequirementProps {
  met: boolean
  label: string
}

function Requirement({ met, label }: RequirementProps) {
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

export default function Register(): JSX.Element {
  const navigate = useNavigate()
  const { register, status, error, clearError } = useAuthStore()

  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [validationError, setValidationError] = useState<string | null>(null)

  const isLoading = status === 'loading'

  const passwordValidation = useMemo(
    () => validatePasswordComplexity(password),
    [password]
  )

  async function handleSubmit(e: FormEvent<HTMLFormElement>): Promise<void> {
    e.preventDefault()
    clearError()
    setValidationError(null)

    if (password !== confirmPassword) {
      setValidationError('Passwords do not match')
      return
    }

    if (!passwordValidation.isValid) {
      const errors: string[] = []
      if (!passwordValidation.hasMinLength) errors.push('at least 8 characters')
      if (!passwordValidation.hasUppercase) errors.push('one uppercase letter')
      if (!passwordValidation.hasLowercase) errors.push('one lowercase letter')
      if (!passwordValidation.hasNumber) errors.push('one number')
      setValidationError(`Password must contain: ${errors.join(', ')}`)
      return
    }

    const emailPrefix = email.split('@')[0] ?? '';
    const finalDisplayName = displayName.trim() || emailPrefix || 'user';

    try {
      await register({ email, password, displayName: finalDisplayName })
      navigate('/')
    } catch {
      // Error is handled by the store
    }
  }

  const displayedError = validationError || error?.message

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      <div className="w-full max-w-md animate-fade-in">
        {/* Logo + Wordmark */}
        <div className="mb-8 text-center">
          <div className="flex flex-col items-center gap-4 mb-6">
            <img
              src="/logo.png"
              alt="resonance logo"
              className="h-16 w-16 rounded-xl shadow-[0_0_30px_rgba(90,106,125,0.3)]"
            />
            <img
              src="/wordmark.png"
              alt="resonance"
              className="h-7 brightness-0 invert opacity-90"
            />
          </div>
          <h1 className="font-display text-display text-text-primary">
            Create account
          </h1>
          <p className="mt-2 text-text-secondary">
            Join and start listening
          </p>
        </div>

        {/* Register form */}
        <Card variant="glass" padding="lg">
          <form onSubmit={handleSubmit} className="space-y-5">
            {/* Error message */}
            {displayedError && (
              <div className="rounded-lg bg-error/20 border border-error/30 p-4 text-error-text">
                <p className="text-sm">{displayedError}</p>
              </div>
            )}

            {/* Email field */}
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
                placeholder="Enter your email"
                required
                autoComplete="email"
                disabled={isLoading}
              />
            </div>

            {/* Display name field */}
            <div>
              <label
                htmlFor="displayName"
                className="mb-2 block text-sm font-medium text-text-secondary"
              >
                Display Name{' '}
                <span className="text-text-muted">(optional)</span>
              </label>
              <Input
                id="displayName"
                type="text"
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                placeholder="How should we call you?"
                autoComplete="name"
                disabled={isLoading}
              />
            </div>

            {/* Password field */}
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
                placeholder="Create a strong password"
                required
                minLength={8}
                autoComplete="new-password"
                disabled={isLoading}
              />
              {/* Password requirements */}
              {password.length > 0 && (
                <div className="mt-3 p-3 rounded-lg bg-background-tertiary/50">
                  <p className="text-xs text-text-muted mb-2">Password requirements:</p>
                  <div className="grid grid-cols-2 gap-2">
                    <Requirement met={passwordValidation.hasMinLength} label="8+ characters" />
                    <Requirement met={passwordValidation.hasUppercase} label="Uppercase (A-Z)" />
                    <Requirement met={passwordValidation.hasLowercase} label="Lowercase (a-z)" />
                    <Requirement met={passwordValidation.hasNumber} label="Number (0-9)" />
                  </div>
                </div>
              )}
            </div>

            {/* Confirm password field */}
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
                placeholder="Confirm your password"
                required
                minLength={8}
                autoComplete="new-password"
                disabled={isLoading}
              />
            </div>

            {/* Submit button */}
            <Button
              type="submit"
              disabled={isLoading}
              className="w-full"
            >
              {isLoading ? (
                <span className="flex items-center justify-center gap-2">
                  <Loader2 className="h-5 w-5 animate-spin" />
                  Creating account...
                </span>
              ) : (
                'Create account'
              )}
            </Button>
          </form>
        </Card>

        {/* Login link */}
        <p className="mt-6 text-center text-text-secondary">
          Already have an account?{' '}
          <Link
            to="/login"
            className="font-medium text-accent-light hover:text-accent-glow transition-colors"
          >
            Sign in
          </Link>
        </p>
      </div>
    </div>
  )
}
