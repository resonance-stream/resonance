import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { Loader2 } from 'lucide-react'
import { useAuthStore } from '../stores/authStore'
import { Button } from '../components/ui/Button'
import { Input } from '../components/ui/Input'
import { Card } from '../components/ui/Card'

export default function Login(): JSX.Element {
  const navigate = useNavigate()
  const { login, status, error, clearError } = useAuthStore()

  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')

  const isLoading = status === 'loading'

  async function handleSubmit(e: FormEvent<HTMLFormElement>): Promise<void> {
    e.preventDefault()
    clearError()

    try {
      await login({ email, password })
      navigate('/')
    } catch {
      // Error is handled by the store
    }
  }

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
            Welcome back
          </h1>
          <p className="mt-2 text-text-secondary">
            Sign in to your account
          </p>
        </div>

        {/* Login form */}
        <Card variant="glass" padding="lg">
          <form onSubmit={handleSubmit} className="space-y-6">
            {/* Error message */}
            {error && (
              <div className="rounded-lg bg-error/20 border border-error/30 p-4 text-error-text">
                <p className="text-sm">{error.message}</p>
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
                placeholder="Enter your password"
                required
                autoComplete="current-password"
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
                  Signing in...
                </span>
              ) : (
                'Sign in'
              )}
            </Button>
          </form>
        </Card>

        {/* Register link */}
        <p className="mt-6 text-center text-text-secondary">
          Don't have an account?{' '}
          <Link
            to="/register"
            className="font-medium text-accent-light hover:text-accent-glow transition-colors"
          >
            Create one
          </Link>
        </p>
      </div>
    </div>
  )
}
