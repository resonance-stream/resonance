/**
 * GraphQL auth operations for Resonance
 *
 * Contains mutations and queries for authentication:
 * - Login
 * - Register
 * - Logout
 * - Refresh token
 * - Get current user
 * - Account settings (change password, update email/profile)
 */

import { gql } from 'graphql-request'

// =============================================================================
// Fragments
// =============================================================================

/**
 * Core user fields fragment for consistent field selection
 * Used across login, register, me query, and account update mutations
 */
export const USER_CORE_FIELDS = gql`
  fragment UserCoreFields on User {
    id
    email
    displayName
    avatarUrl
    role
    emailVerified
    createdAt
    updatedAt
    passwordUpdatedAt
  }
`

/**
 * Login mutation
 * Authenticates user with email and password
 */
export const LOGIN_MUTATION = gql`
  mutation Login($input: LoginInput!) {
    login(input: $input) {
      id
      email
      displayName
      avatarUrl
      role
      emailVerified
      createdAt
      updatedAt
      accessToken
      refreshToken
      expiresAt
      tokenType
    }
  }
`

/**
 * Register mutation
 * Creates a new user account
 */
export const REGISTER_MUTATION = gql`
  mutation Register($input: RegisterInput!) {
    register(input: $input) {
      id
      email
      displayName
      avatarUrl
      role
      emailVerified
      createdAt
      updatedAt
      accessToken
      refreshToken
      expiresAt
      tokenType
    }
  }
`

/**
 * Logout mutation
 * Invalidates the current session/tokens
 */
export const LOGOUT_MUTATION = gql`
  mutation Logout {
    logout
  }
`

/**
 * Refresh token mutation
 * Exchanges a refresh token for new access/refresh tokens
 * Note: Uses snake_case for input field to match backend RefreshTokenInput
 */
export const REFRESH_TOKEN_MUTATION = gql`
  mutation RefreshToken($refreshToken: String!) {
    refreshToken(input: { refresh_token: $refreshToken }) {
      accessToken
      refreshToken
      expiresAt
      tokenType
    }
  }
`

/**
 * Get current user query
 * Retrieves the authenticated user's profile
 */
export const ME_QUERY = gql`
  query Me {
    me {
      id
      email
      displayName
      avatarUrl
      role
      emailVerified
      lastSeenAt
      createdAt
      updatedAt
      passwordUpdatedAt
    }
  }
`

// =============================================================================
// Account Settings Mutations
// =============================================================================

/**
 * Change password mutation
 * Requires current password for verification.
 * Invalidates all other sessions after successful change.
 */
export const CHANGE_PASSWORD_MUTATION = gql`
  mutation ChangePassword($input: ChangePasswordInput!) {
    changePassword(input: $input) {
      success
      sessionsInvalidated
    }
  }
`

/**
 * Update email mutation
 * Requires current password for verification.
 * Resets email_verified to false after change.
 */
export const UPDATE_EMAIL_MUTATION = gql`
  ${USER_CORE_FIELDS}
  mutation UpdateEmail($input: UpdateEmailInput!) {
    updateEmail(input: $input) {
      ...UserCoreFields
    }
  }
`

/**
 * Update profile mutation
 * Updates display name and/or avatar URL.
 * At least one field must be provided.
 */
export const UPDATE_PROFILE_MUTATION = gql`
  ${USER_CORE_FIELDS}
  mutation UpdateProfile($input: UpdateProfileInput!) {
    updateProfile(input: $input) {
      ...UserCoreFields
    }
  }
`

/**
 * Delete account mutation
 * Permanently deletes the user's account after password verification.
 * This action is irreversible.
 */
export const DELETE_ACCOUNT_MUTATION = gql`
  mutation DeleteAccount($input: DeleteAccountInput!) {
    deleteAccount(input: $input)
  }
`
