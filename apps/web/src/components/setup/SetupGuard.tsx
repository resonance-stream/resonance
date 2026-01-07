/**
 * SetupGuard component for Resonance
 *
 * Guards routes that require setup to be completed.
 * Redirects to /setup if setup is not complete (unless already on /setup).
 * Redirects to / if setup is complete and user is on /setup.
 */

import { Navigate, useLocation } from 'react-router-dom'
import { useSetupStatus } from '../../hooks/useSetup'
import type { ReactNode } from 'react'

interface SetupGuardProps {
  /** Child components to render */
  children: ReactNode
}

/**
 * Loading spinner shown while checking setup status
 */
function SetupLoadingFallback(): JSX.Element {
  return (
    <div className="flex h-screen items-center justify-center bg-background">
      <div className="flex flex-col items-center gap-4">
        <div className="h-12 w-12 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        <p className="text-text-secondary">Checking setup status...</p>
      </div>
    </div>
  )
}

/**
 * Guard component that checks if setup is complete.
 *
 * Behavior:
 * - If setup not complete and not on /setup route: redirect to /setup
 * - If setup complete and on /setup route: redirect to /
 * - Otherwise: render children
 *
 * Usage:
 * ```tsx
 * // In App.tsx, wrap the entire routes with SetupGuard
 * <SetupGuard>
 *   <Routes>
 *     ...
 *   </Routes>
 * </SetupGuard>
 * ```
 */
export function SetupGuard({ children }: SetupGuardProps): JSX.Element {
  const location = useLocation()
  const { data: setupStatus, isLoading, isError } = useSetupStatus()

  const isOnSetupPage = location.pathname === '/setup'

  // Show loading while fetching setup status
  if (isLoading) {
    return <SetupLoadingFallback />
  }

  // On error, allow access (fail open) to prevent being locked out
  // The actual routes will handle auth as needed
  if (isError) {
    return <>{children}</>
  }

  // If setup is not complete, redirect to setup (unless already there)
  if (setupStatus && !setupStatus.isComplete && !isOnSetupPage) {
    return <Navigate to="/setup" replace />
  }

  // If setup is complete and on setup page, redirect to home
  if (setupStatus?.isComplete && isOnSetupPage) {
    return <Navigate to="/" replace />
  }

  // Setup check passed, render children
  return <>{children}</>
}

export default SetupGuard
