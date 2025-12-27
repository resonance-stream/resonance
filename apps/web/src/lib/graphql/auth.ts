/**
 * GraphQL auth operations for Resonance
 *
 * Contains mutations and queries for authentication:
 * - Login
 * - Register
 * - Logout
 * - Refresh token
 * - Get current user
 */

import { gql } from 'graphql-request'

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
    }
  }
`
