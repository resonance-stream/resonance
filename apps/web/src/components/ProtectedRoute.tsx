/**
 * ProtectedRoute component for Resonance
 *
 * Protects routes that require authentication.
 * Redirects to login page if user is not authenticated.
 * Optionally requires admin role for certain routes.
 */

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

  // Show loading while checking auth state (initial load or refreshing)
  if (status === 'idle' || status === 'loading') {
    return <AuthLoadingFallback />
  }

  // Not authenticated - redirect to login with return path
  if (status === 'unauthenticated' || !user) {
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
