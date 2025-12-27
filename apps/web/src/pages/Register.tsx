import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'

export default function Register(): JSX.Element {
  const navigate = useNavigate()
  const { register, status, error, clearError } = useAuthStore()

  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [validationError, setValidationError] = useState<string | null>(null)

  const isLoading = status === 'loading'

  async function handleSubmit(e: FormEvent<HTMLFormElement>): Promise<void> {
    e.preventDefault()
    clearError()
    setValidationError(null)

    // Client-side validation
    if (password !== confirmPassword) {
      setValidationError('Passwords do not match')
      return
    }

    if (password.length < 8) {
      setValidationError('Password must be at least 8 characters')
      return
    }

    try {
      await register({
        username,
        email,
        password,
        displayName: displayName || undefined,
      })
      navigate('/')
    } catch {
      // Error is handled by the store
    }
  }

  const displayedError = validationError || error?.message

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      <div className="w-full max-w-md animate-fade-in">
        {/* Logo and heading */}
        <div className="mb-8 text-center">
          <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-xl bg-gradient-to-br from-primary to-accent">
            <svg
              className="h-8 w-8 text-white"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"
              />
            </svg>
          </div>
          <h1 className="text-3xl font-bold text-text-primary">Create account</h1>
          <p className="mt-2 text-text-secondary">
            Join Resonance and start listening
          </p>
        </div>

        {/* Register form */}
        <div className="card">
          <form onSubmit={handleSubmit} className="space-y-5">
            {/* Error message */}
            {displayedError && (
              <div className="rounded-lg bg-red-500/10 border border-red-500/20 p-4 text-red-400">
                <p className="text-sm">{displayedError}</p>
              </div>
            )}

            {/* Username field */}
            <div>
              <label
                htmlFor="username"
                className="mb-2 block text-sm font-medium text-text-secondary"
              >
                Username
              </label>
              <input
                id="username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="input w-full"
                placeholder="Choose a username"
                required
                autoComplete="username"
                disabled={isLoading}
                pattern="[a-zA-Z0-9_-]+"
                title="Username can only contain letters, numbers, underscores, and hyphens"
              />
            </div>

            {/* Email field */}
            <div>
              <label
                htmlFor="email"
                className="mb-2 block text-sm font-medium text-text-secondary"
              >
                Email
              </label>
              <input
                id="email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="input w-full"
                placeholder="Enter your email"
                required
                autoComplete="email"
                disabled={isLoading}
              />
            </div>

            {/* Display name field (optional) */}
            <div>
              <label
                htmlFor="displayName"
                className="mb-2 block text-sm font-medium text-text-secondary"
              >
                Display Name{' '}
                <span className="text-text-muted">(optional)</span>
              </label>
              <input
                id="displayName"
                type="text"
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                className="input w-full"
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
              <input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="input w-full"
                placeholder="Create a password (min. 8 characters)"
                required
                minLength={8}
                autoComplete="new-password"
                disabled={isLoading}
              />
            </div>

            {/* Confirm password field */}
            <div>
              <label
                htmlFor="confirmPassword"
                className="mb-2 block text-sm font-medium text-text-secondary"
              >
                Confirm Password
              </label>
              <input
                id="confirmPassword"
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                className="input w-full"
                placeholder="Confirm your password"
                required
                minLength={8}
                autoComplete="new-password"
                disabled={isLoading}
              />
            </div>

            {/* Submit button */}
            <button
              type="submit"
              disabled={isLoading}
              className="btn-primary w-full disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isLoading ? (
                <span className="flex items-center justify-center gap-2">
                  <svg
                    className="h-5 w-5 animate-spin"
                    fill="none"
                    viewBox="0 0 24 24"
                  >
                    <circle
                      className="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      strokeWidth="4"
                    />
                    <path
                      className="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    />
                  </svg>
                  Creating account...
                </span>
              ) : (
                'Create account'
              )}
            </button>
          </form>
        </div>

        {/* Login link */}
        <p className="mt-6 text-center text-text-secondary">
          Already have an account?{' '}
          <Link
            to="/login"
            className="font-medium text-primary hover:text-primary-hover transition-colors"
          >
            Sign in
          </Link>
        </p>
      </div>
    </div>
  )
}
