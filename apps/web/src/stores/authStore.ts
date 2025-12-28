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
import type { User, UserRole } from '@resonance/shared-types'
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
 * GraphQL error structure from graphql-request ClientError
 */
interface GraphQLErrorResponse {
  response?: {
    errors?: Array<{
      message?: string
      extensions?: {
        code?: string
        [key: string]: unknown
      }
    }>
  }
}

/**
 * Check if error is a GraphQL ClientError with response data
 */
function isGraphQLError(error: unknown): error is Error & GraphQLErrorResponse {
  return (
    error instanceof Error &&
    'response' in error &&
    typeof (error as GraphQLErrorResponse).response === 'object'
  )
}

/**
 * Parse GraphQL error into AuthError
 * Extracts error details from graphql-request ClientError response when available
 */
function parseAuthError(error: unknown): AuthError {
  // First, try to extract structured error from GraphQL response
  if (isGraphQLError(error)) {
    const firstError = error.response?.errors?.[0]
    if (firstError) {
      const code = firstError.extensions?.code
      const message = firstError.message ?? ''
      const messageLower = message.toLowerCase()

      // Map GraphQL error codes to AuthError codes
      if (code === 'INVALID_CREDENTIALS' || code === 'UNAUTHORIZED') {
        return { code: 'INVALID_CREDENTIALS', message: 'Invalid email or password' }
      }
      if (code === 'USER_NOT_FOUND') {
        return { code: 'USER_NOT_FOUND', message: 'User not found' }
      }
      if (code === 'EMAIL_EXISTS' || code === 'EMAIL_ALREADY_EXISTS') {
        return { code: 'EMAIL_ALREADY_EXISTS', message: 'Email already in use' }
      }
      if (code === 'USERNAME_EXISTS' || code === 'USERNAME_ALREADY_EXISTS') {
        return { code: 'USERNAME_ALREADY_EXISTS', message: 'Username already taken' }
      }
      if (code === 'TOKEN_EXPIRED') {
        return { code: 'TOKEN_EXPIRED', message: 'Session expired. Please log in again.' }
      }
      if (code === 'TOKEN_INVALID' || code === 'INVALID_TOKEN') {
        return { code: 'TOKEN_INVALID', message: 'Invalid session. Please log in again.' }
      }
      if (code === 'RATE_LIMITED') {
        return { code: 'RATE_LIMITED', message: 'Too many attempts. Please try again later.' }
      }

      // Fall back to message-based detection for this error
      if (messageLower.includes('invalid credentials') || messageLower.includes('wrong password')) {
        return { code: 'INVALID_CREDENTIALS', message: 'Invalid email or password' }
      }
      if (messageLower.includes('user not found')) {
        return { code: 'USER_NOT_FOUND', message: 'User not found' }
      }
      if (messageLower.includes('email') && messageLower.includes('exists')) {
        return { code: 'EMAIL_ALREADY_EXISTS', message: 'Email already in use' }
      }
      if (messageLower.includes('username') && messageLower.includes('exists')) {
        return { code: 'USERNAME_ALREADY_EXISTS', message: 'Username already taken' }
      }
      if (messageLower.includes('token') && messageLower.includes('expired')) {
        return { code: 'TOKEN_EXPIRED', message: 'Session expired. Please log in again.' }
      }
      if (messageLower.includes('token') && messageLower.includes('invalid')) {
        return { code: 'TOKEN_INVALID', message: 'Invalid session. Please log in again.' }
      }

      // Return the server's message if available
      if (message) {
        return { code: 'UNKNOWN_ERROR', message }
      }
    }
  }

  // Handle regular Error objects
  if (error instanceof Error) {
    const message = error.message.toLowerCase()

    // Check for network errors
    if (message.includes('network') || message.includes('fetch') || message.includes('failed to fetch')) {
      return { code: 'NETWORK_ERROR', message: 'Network error. Please check your connection.' }
    }

    // Legacy message-based detection for non-GraphQL errors
    if (message.includes('invalid credentials') || message.includes('wrong password')) {
      return { code: 'INVALID_CREDENTIALS', message: 'Invalid email or password' }
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

    return { code: 'UNKNOWN_ERROR', message: error.message }
  }

  return { code: 'UNKNOWN_ERROR', message: 'An unexpected error occurred' }
}

/**
 * Maximum token expiry buffer: 90 days in milliseconds
 * Tokens expiring beyond this are capped to prevent excessive durations
 */
const MAX_EXPIRY_BUFFER_MS = 90 * 24 * 60 * 60 * 1000

/**
 * Parse DateTime string to timestamp with buffer for early refresh
 * Returns null if the date is invalid to prevent NaN propagation
 * Clamps the result to ensure it's within a reasonable range
 */
function parseExpiresAt(expiresAt: string): number | null {
  // Parse ISO8601 DateTime and subtract 60 seconds as buffer
  const timestamp = new Date(expiresAt).getTime()

  // Check for invalid date (NaN)
  if (Number.isNaN(timestamp)) {
    return null
  }

  const bufferedExpiry = timestamp - 60 * 1000
  const now = Date.now()

  // Clamp to ensure the expiry is in the future (at least 1 second from now)
  // This prevents issues with tokens that are already expired or about to expire
  const minExpiry = now + 1000

  // Also cap to a maximum reasonable duration (90 days from now)
  const maxExpiry = now + MAX_EXPIRY_BUFFER_MS

  return Math.max(minExpiry, Math.min(bufferedExpiry, maxExpiry))
}

/**
 * Auth payload response from login/register mutations
 * Fields are flattened from AuthPayloadUser via #[graphql(flatten)]
 */
interface AuthPayloadResponse {
  id: string
  email: string
  displayName: string
  avatarUrl: string | null
  role: string
  emailVerified: boolean
  createdAt: string
  updatedAt: string
  accessToken: string
  refreshToken: string
  expiresAt: string
  tokenType: string
}

/**
 * Valid user roles
 */
const VALID_ROLES: UserRole[] = ['admin', 'user', 'guest']

/**
 * Minimum valid timestamp (2020-01-01)
 * Timestamps before this are likely invalid
 */
const MIN_VALID_TIMESTAMP = new Date('2020-01-01').getTime()

/**
 * Maximum valid timestamp (100 years from now)
 * Timestamps beyond this are likely invalid
 */
const MAX_VALID_TIMESTAMP = Date.now() + 100 * 365 * 24 * 60 * 60 * 1000

/**
 * Validate that a string is a valid ISO8601 timestamp within a reasonable range
 * Rejects NaN, dates before 2020, and dates more than 100 years in the future
 */
function isValidTimestamp(value: string): boolean {
  const timestamp = new Date(value).getTime()
  if (Number.isNaN(timestamp)) {
    return false
  }
  // Reject timestamps outside reasonable bounds
  if (timestamp < MIN_VALID_TIMESTAMP || timestamp > MAX_VALID_TIMESTAMP) {
    return false
  }
  return true
}

/**
 * Extract user object from flattened auth payload response
 * Validates role and timestamps to prevent invalid state
 */
function extractUserFromPayload(payload: AuthPayloadResponse): User | null {
  // Convert role to lowercase and validate
  const roleLower = payload.role.toLowerCase()
  if (!VALID_ROLES.includes(roleLower as UserRole)) {
    console.error('Invalid role in auth payload:', payload.role)
    return null
  }
  const role = roleLower as UserRole

  // Validate timestamps
  if (!isValidTimestamp(payload.createdAt) || !isValidTimestamp(payload.updatedAt)) {
    console.error('Invalid timestamps in auth payload')
    return null
  }

  // Use email prefix as username (backend doesn't return username yet)
  const username = payload.email.split('@')[0] ?? payload.email

  return {
    id: payload.id,
    username,
    email: payload.email,
    displayName: payload.displayName,
    avatarUrl: payload.avatarUrl ?? undefined, // Convert null to undefined
    role,
    emailVerified: payload.emailVerified,
    createdAt: payload.createdAt,
    updatedAt: payload.updatedAt,
  }
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
       * Login with email and password
       */
      login: async (credentials: LoginCredentials) => {
        set({ status: 'loading', error: null })

        try {
          const response = await graphqlClient.request<{ login: AuthPayloadResponse }>(
            LOGIN_MUTATION,
            {
              input: {
                email: credentials.email,
                password: credentials.password,
              },
            }
          )

          const payload = response.login
          const user = extractUserFromPayload(payload)
          const expiresAt = parseExpiresAt(payload.expiresAt)

          // Validate payload before accepting
          if (!user || expiresAt === null) {
            const authError: AuthError = {
              code: 'UNKNOWN_ERROR',
              message: 'Invalid response from server',
            }
            set({ status: 'unauthenticated', error: authError })
            throw authError
          }

          // Set auth header for subsequent requests
          setAuthToken(payload.accessToken)

          set({
            user,
            accessToken: payload.accessToken,
            refreshToken: payload.refreshToken,
            expiresAt,
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
          const response = await graphqlClient.request<{ register: AuthPayloadResponse }>(
            REGISTER_MUTATION,
            {
              input: {
                email: credentials.email,
                password: credentials.password,
                display_name: credentials.displayName ?? credentials.email.split('@')[0],
              },
            }
          )

          const payload = response.register
          const user = extractUserFromPayload(payload)
          const expiresAt = parseExpiresAt(payload.expiresAt)

          // Validate payload before accepting
          if (!user || expiresAt === null) {
            const authError: AuthError = {
              code: 'UNKNOWN_ERROR',
              message: 'Invalid response from server',
            }
            set({ status: 'unauthenticated', error: authError })
            throw authError
          }

          // Set auth header for subsequent requests
          setAuthToken(payload.accessToken)

          set({
            user,
            accessToken: payload.accessToken,
            refreshToken: payload.refreshToken,
            expiresAt,
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
          // Response type for RefreshPayload (no user, has expiresAt timestamp)
          interface RefreshPayloadResponse {
            accessToken: string
            refreshToken: string
            expiresAt: string
            tokenType: string
          }

          // Pass refresh token as mutation variable (not as Authorization header)
          const response = await graphqlClient.request<{ refreshToken: RefreshPayloadResponse }>(
            REFRESH_TOKEN_MUTATION,
            { refreshToken }
          )

          const { accessToken, refreshToken: newRefreshToken, expiresAt } = response.refreshToken
          const parsedExpiresAt = parseExpiresAt(expiresAt)

          // Validate expiresAt before accepting
          if (parsedExpiresAt === null) {
            throw new Error('Invalid expiry time in refresh response')
          }

          // Set new auth header
          setAuthToken(accessToken)

          set({
            accessToken,
            refreshToken: newRefreshToken,
            expiresAt: parsedExpiresAt,
            status: 'authenticated',
            error: null,
          })

          return true
        } catch (error) {
          // Check if this is a network error - don't log out on transient failures
          const authError = parseAuthError(error)
          if (authError.code === 'NETWORK_ERROR') {
            // Keep the session intact so user can retry when connectivity returns
            set({
              status: 'authenticated',
              error: authError,
            })
            return false
          }

          // Non-network refresh failure - clear session
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
          // Fully clear state when no token
          setAuthToken(null)
          set({
            user: null,
            accessToken: null,
            refreshToken: null,
            expiresAt: null,
            status: 'unauthenticated',
          })
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
          if (refreshed) {
            // Retry user fetch with new access token
            try {
              const { accessToken: newToken } = get()
              if (newToken) {
                setAuthToken(newToken)
                const response = await graphqlClient.request<{ me: User }>(ME_QUERY)
                set({
                  user: response.me,
                  status: 'authenticated',
                })
                return
              }
            } catch (retryError) {
              const authError = parseAuthError(retryError)

              // Don't log the user out on transient network errors
              if (authError.code === 'NETWORK_ERROR') {
                set({ status: 'authenticated', error: authError })
                return
              }
            }
          }

          // Ensure we don't keep sending a stale Authorization header / stale tokens
          setAuthToken(null)
          set({
            user: null,
            accessToken: null,
            refreshToken: null,
            expiresAt: null,
            status: 'unauthenticated',
          })
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
       * Fully clears invalid persisted sessions to prevent bad state
       * Note: User object is not required - it can be fetched after hydration
       */
      hydrate: () => {
        const { accessToken, expiresAt, refreshToken, fetchCurrentUser: fetchUser, refreshAccessToken: refresh } = get()

        // Validate that essential token fields exist and are valid
        // Note: user is not required - it can be fetched after hydration
        const hasValidTokens =
          accessToken &&
          refreshToken &&
          typeof expiresAt === 'number' &&
          !Number.isNaN(expiresAt)

        if (!hasValidTokens) {
          // Fully clear invalid persisted session
          setAuthToken(null)
          set({
            user: null,
            accessToken: null,
            refreshToken: null,
            expiresAt: null,
            status: 'unauthenticated',
            error: null,
          })
          return
        }

        // Check if token is expired
        if (Date.now() >= expiresAt) {
          // Token expired, set loading state and try to refresh
          set({ status: 'loading' })
          void refresh()
        } else {
          // Token still valid, set auth header and fetch user if needed
          setAuthToken(accessToken)
          set({ status: 'authenticated' })

          // If we don't have a user, fetch it
          const { user } = get()
          if (!user) {
            void fetchUser()
          }
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
