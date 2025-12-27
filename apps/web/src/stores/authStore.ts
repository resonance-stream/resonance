/**
 * Auth store for Resonance
 *
 * Manages authentication state including:
 * - User session
 * - Access/refresh tokens
 * - Login/register/logout actions
 * - Token refresh
 */

import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { User, AuthPayload } from '@resonance/shared-types'
import type { LoginCredentials, RegisterCredentials, AuthStatus, AuthError } from '../types/auth'
import { graphqlClient, setAuthToken } from '../lib/api'
import {
  LOGIN_MUTATION,
  REGISTER_MUTATION,
  LOGOUT_MUTATION,
  REFRESH_TOKEN_MUTATION,
  ME_QUERY,
} from '../lib/graphql/auth'

interface AuthState {
  // State
  user: User | null
  accessToken: string | null
  refreshToken: string | null
  expiresAt: number | null
  status: AuthStatus
  error: AuthError | null

  // Actions
  login: (credentials: LoginCredentials) => Promise<void>
  register: (credentials: RegisterCredentials) => Promise<void>
  logout: () => Promise<void>
  refreshAccessToken: () => Promise<boolean>
  fetchCurrentUser: () => Promise<void>
  clearError: () => void
  setStatus: (status: AuthStatus) => void
  hydrate: () => void
}

/**
 * Parse GraphQL error into AuthError
 */
function parseAuthError(error: unknown): AuthError {
  if (error instanceof Error) {
    const message = error.message.toLowerCase()

    if (message.includes('invalid credentials') || message.includes('wrong password')) {
      return { code: 'INVALID_CREDENTIALS', message: 'Invalid username or password' }
    }
    if (message.includes('user not found')) {
      return { code: 'USER_NOT_FOUND', message: 'User not found' }
    }
    if (message.includes('email') && message.includes('exists')) {
      return { code: 'EMAIL_ALREADY_EXISTS', message: 'Email already in use' }
    }
    if (message.includes('username') && message.includes('exists')) {
      return { code: 'USERNAME_ALREADY_EXISTS', message: 'Username already taken' }
    }
    if (message.includes('token') && message.includes('expired')) {
      return { code: 'TOKEN_EXPIRED', message: 'Session expired. Please log in again.' }
    }
    if (message.includes('token') && message.includes('invalid')) {
      return { code: 'TOKEN_INVALID', message: 'Invalid session. Please log in again.' }
    }
    if (message.includes('network') || message.includes('fetch')) {
      return { code: 'NETWORK_ERROR', message: 'Network error. Please check your connection.' }
    }

    return { code: 'UNKNOWN_ERROR', message: error.message }
  }

  return { code: 'UNKNOWN_ERROR', message: 'An unexpected error occurred' }
}

/**
 * Calculate token expiration timestamp
 */
function calculateExpiresAt(expiresIn: number): number {
  // expiresIn is in seconds, convert to milliseconds and add to current time
  // Subtract 60 seconds as buffer to refresh before actual expiration
  return Date.now() + (expiresIn - 60) * 1000
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      // Initial state
      user: null,
      accessToken: null,
      refreshToken: null,
      expiresAt: null,
      status: 'idle',
      error: null,

      /**
       * Login with username/email and password
       */
      login: async (credentials: LoginCredentials) => {
        set({ status: 'loading', error: null })

        try {
          const response = await graphqlClient.request<{ login: AuthPayload }>(
            LOGIN_MUTATION,
            {
              usernameOrEmail: credentials.usernameOrEmail,
              password: credentials.password,
              rememberMe: credentials.rememberMe,
            }
          )

          const { user, accessToken, refreshToken, expiresIn } = response.login

          // Set auth header for subsequent requests
          setAuthToken(accessToken)

          set({
            user,
            accessToken,
            refreshToken,
            expiresAt: calculateExpiresAt(expiresIn),
            status: 'authenticated',
            error: null,
          })
        } catch (error) {
          const authError = parseAuthError(error)
          set({ status: 'unauthenticated', error: authError })
          throw authError
        }
      },

      /**
       * Register a new user account
       */
      register: async (credentials: RegisterCredentials) => {
        set({ status: 'loading', error: null })

        try {
          const response = await graphqlClient.request<{ register: AuthPayload }>(
            REGISTER_MUTATION,
            {
              username: credentials.username,
              email: credentials.email,
              password: credentials.password,
              displayName: credentials.displayName,
            }
          )

          const { user, accessToken, refreshToken, expiresIn } = response.register

          // Set auth header for subsequent requests
          setAuthToken(accessToken)

          set({
            user,
            accessToken,
            refreshToken,
            expiresAt: calculateExpiresAt(expiresIn),
            status: 'authenticated',
            error: null,
          })
        } catch (error) {
          const authError = parseAuthError(error)
          set({ status: 'unauthenticated', error: authError })
          throw authError
        }
      },

      /**
       * Logout and clear session
       */
      logout: async () => {
        const { accessToken } = get()

        try {
          // Only call logout mutation if we have a token
          if (accessToken) {
            await graphqlClient.request(LOGOUT_MUTATION)
          }
        } catch {
          // Ignore logout errors - we'll clear local state anyway
        } finally {
          // Clear auth header
          setAuthToken(null)

          // Clear state
          set({
            user: null,
            accessToken: null,
            refreshToken: null,
            expiresAt: null,
            status: 'unauthenticated',
            error: null,
          })
        }
      },

      /**
       * Refresh the access token using the refresh token
       * Returns true if successful, false otherwise
       */
      refreshAccessToken: async () => {
        const { refreshToken } = get()

        if (!refreshToken) {
          set({ status: 'unauthenticated' })
          return false
        }

        try {
          // Temporarily set refresh token as auth header for this request
          setAuthToken(refreshToken)

          const response = await graphqlClient.request<{ refreshToken: AuthPayload }>(
            REFRESH_TOKEN_MUTATION
          )

          const { user, accessToken, refreshToken: newRefreshToken, expiresIn } = response.refreshToken

          // Set new auth header
          setAuthToken(accessToken)

          set({
            user,
            accessToken,
            refreshToken: newRefreshToken,
            expiresAt: calculateExpiresAt(expiresIn),
            status: 'authenticated',
            error: null,
          })

          return true
        } catch {
          // Refresh failed - clear session
          setAuthToken(null)
          set({
            user: null,
            accessToken: null,
            refreshToken: null,
            expiresAt: null,
            status: 'unauthenticated',
            error: { code: 'SESSION_EXPIRED', message: 'Your session has expired. Please log in again.' },
          })
          return false
        }
      },

      /**
       * Fetch the current authenticated user
       */
      fetchCurrentUser: async () => {
        const { accessToken } = get()

        if (!accessToken) {
          set({ status: 'unauthenticated' })
          return
        }

        set({ status: 'loading' })

        try {
          setAuthToken(accessToken)
          const response = await graphqlClient.request<{ me: User }>(ME_QUERY)

          set({
            user: response.me,
            status: 'authenticated',
          })
        } catch {
          // Token might be expired, try refresh
          const refreshed = await get().refreshAccessToken()
          if (!refreshed) {
            set({ status: 'unauthenticated' })
          }
        }
      },

      /**
       * Clear any auth error
       */
      clearError: () => {
        set({ error: null })
      },

      /**
       * Set auth status
       */
      setStatus: (status: AuthStatus) => {
        set({ status })
      },

      /**
       * Hydrate auth state after loading from storage
       * Called on app initialization to restore auth header
       */
      hydrate: () => {
        const { accessToken, expiresAt, refreshAccessToken: refresh } = get()

        if (accessToken && expiresAt) {
          // Check if token is expired
          if (Date.now() >= expiresAt) {
            // Token expired, try to refresh
            void refresh()
          } else {
            // Token still valid, set auth header
            setAuthToken(accessToken)
            set({ status: 'authenticated' })
          }
        } else {
          set({ status: 'unauthenticated' })
        }
      },
    }),
    {
      name: 'resonance-auth',
      // Only persist these fields
      partialize: (state) => ({
        user: state.user,
        accessToken: state.accessToken,
        refreshToken: state.refreshToken,
        expiresAt: state.expiresAt,
      }),
      // Hydrate on rehydration
      onRehydrateStorage: () => (state) => {
        if (state) {
          state.hydrate()
        }
      },
    }
  )
)
