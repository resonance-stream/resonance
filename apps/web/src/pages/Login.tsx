import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'

export default function Login(): JSX.Element {
  const navigate = useNavigate()
  const { login, status, error, clearError } = useAuthStore()

  const [usernameOrEmail, setUsernameOrEmail] = useState('')
  const [password, setPassword] = useState('')
  const [rememberMe, setRememberMe] = useState(false)

  const isLoading = status === 'loading'

  async function handleSubmit(e: FormEvent<HTMLFormElement>): Promise<void> {
    e.preventDefault()
    clearError()

    try {
      await login({ usernameOrEmail, password, rememberMe })
      navigate('/')
    } catch {
      // Error is handled by the store
    }
  }

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
          <h1 className="text-3xl font-bold text-text-primary">Welcome back</h1>
          <p className="mt-2 text-text-secondary">
            Sign in to your Resonance account
          </p>
        </div>

        {/* Login form */}
        <div className="card">
          <form onSubmit={handleSubmit} className="space-y-6">
            {/* Error message */}
            {error && (
              <div className="rounded-lg bg-red-500/10 border border-red-500/20 p-4 text-red-400">
                <p className="text-sm">{error.message}</p>
              </div>
            )}

            {/* Username or Email field */}
            <div>
              <label
                htmlFor="usernameOrEmail"
                className="mb-2 block text-sm font-medium text-text-secondary"
              >
                Username or Email
              </label>
              <input
                id="usernameOrEmail"
                type="text"
                value={usernameOrEmail}
                onChange={(e) => setUsernameOrEmail(e.target.value)}
                className="input w-full"
                placeholder="Enter your username or email"
                required
                autoComplete="username"
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
                placeholder="Enter your password"
                required
                autoComplete="current-password"
                disabled={isLoading}
              />
            </div>

            {/* Remember me checkbox */}
            <div className="flex items-center">
              <input
                id="rememberMe"
                type="checkbox"
                checked={rememberMe}
                onChange={(e) => setRememberMe(e.target.checked)}
                className="h-4 w-4 rounded border-background-tertiary bg-background-secondary text-primary focus:ring-primary/50"
                disabled={isLoading}
              />
              <label
                htmlFor="rememberMe"
                className="ml-2 text-sm text-text-secondary"
              >
                Remember me
              </label>
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
                  Signing in...
                </span>
              ) : (
                'Sign in'
              )}
            </button>
          </form>
        </div>

        {/* Register link */}
        <p className="mt-6 text-center text-text-secondary">
          Don't have an account?{' '}
          <Link
            to="/register"
            className="font-medium text-primary hover:text-primary-hover transition-colors"
          >
            Create one
          </Link>
        </p>
      </div>
    </div>
  )
}
