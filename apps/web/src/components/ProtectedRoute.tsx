/**
 * ProtectedRoute component for Resonance
 *
 * Protects routes that require authentication.
 * Redirects to login page if user is not authenticated.
 * Optionally requires admin role for certain routes.
 */

import { useState, useEffect } from 'react'
import { Navigate, useLocation } from 'react-router-dom'
import { useAuthStore } from '../stores/authStore'
import type { ReactNode } from 'react'

interface ProtectedRouteProps {
  /** Child components to render if authenticated */
  children: ReactNode
  /** Require admin role for access */
  requireAdmin?: boolean
  /** Custom redirect path (defaults to /login) */
  redirectTo?: string
}

/**
 * Loading spinner shown while checking auth state
 */
function AuthLoadingFallback(): JSX.Element {
  return (
    <div className="flex h-screen items-center justify-center bg-background">
      <div className="flex flex-col items-center gap-4">
        <div className="h-12 w-12 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        <p className="text-text-secondary">Checking authentication...</p>
      </div>
    </div>
  )
}

/**
 * Wrapper component that protects routes requiring authentication.
 *
 * Usage:
 * ```tsx
 * <Route
 *   path="/library"
 *   element={
 *     <ProtectedRoute>
 *       <Library />
 *     </ProtectedRoute>
 *   }
 * />
 *
 * // For admin-only routes:
 * <Route
 *   path="/admin"
 *   element={
 *     <ProtectedRoute requireAdmin>
 *       <AdminPanel />
 *     </ProtectedRoute>
 *   }
 * />
 * ```
 */
export function ProtectedRoute({
  children,
  requireAdmin = false,
  redirectTo = '/login',
}: ProtectedRouteProps): JSX.Element {
  const location = useLocation()
  const status = useAuthStore((state) => state.status)
  const user = useAuthStore((state) => state.user)

  // Track hydration state to prevent infinite loading
  const [hasHydrated, setHasHydrated] = useState(() => useAuthStore.persist.hasHydrated())

  useEffect(() => {
    const unsub = useAuthStore.persist.onFinishHydration(() => setHasHydrated(true))
    return unsub
  }, [])

  // Show loading only while Zustand persistence is hydrating or during active loading
  if (!hasHydrated || status === 'loading') {
    return <AuthLoadingFallback />
  }

  // If hydration is complete but we're still idle, treat as unauthenticated
  if (status === 'idle' || status === 'unauthenticated' || !user) {
    return (
      <Navigate
        to={redirectTo}
        state={{ from: location.pathname }}
        replace
      />
    )
  }

  // Check admin requirement
  if (requireAdmin && user.role !== 'admin') {
    // User is authenticated but not admin - redirect to home
    return <Navigate to="/" replace />
  }

  // Authenticated (and admin if required) - render children
  return <>{children}</>
}

export default ProtectedRoute
