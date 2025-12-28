/**
 * Frontend authentication types for Resonance
 *
 * Re-exports shared types and adds frontend-specific auth types.
 */

// Re-export auth types from shared-types for convenience
export type {
  User,
  UserRole,
  AuthTokens,
  LoginRequest,
  RegisterRequest,
  Session,
} from '@resonance/shared-types'

export type {
  AuthPayload,
  LoginMutationArgs,
  RegisterMutationArgs,
} from '@resonance/shared-types'

/**
 * Login form credentials
 */
export interface LoginCredentials {
  /** User's email address */
  email: string
  /** User password */
  password: string
}

/**
 * Registration form credentials
 */
export interface RegisterCredentials {
  /** Email address */
  email: string
  /** Password */
  password: string
  /** Display name (optional, defaults to email prefix) */
  displayName?: string
}

/**
 * Auth state status
 */
export type AuthStatus = 'idle' | 'loading' | 'authenticated' | 'unauthenticated'

/**
 * Auth error response
 */
export interface AuthError {
  /** Error code for programmatic handling */
  code: AuthErrorCode
  /** Human-readable error message */
  message: string
  /** Field-specific errors for forms */
  fieldErrors?: Record<string, string>
}

/**
 * Auth error codes
 */
export type AuthErrorCode =
  | 'INVALID_CREDENTIALS'
  | 'USER_NOT_FOUND'
  | 'EMAIL_ALREADY_EXISTS'
  | 'USERNAME_ALREADY_EXISTS'
  | 'INVALID_EMAIL'
  | 'WEAK_PASSWORD'
  | 'TOKEN_EXPIRED'
  | 'TOKEN_INVALID'
  | 'SESSION_EXPIRED'
  | 'RATE_LIMITED'
  | 'NETWORK_ERROR'
  | 'UNKNOWN_ERROR'
