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
 * Authenticates user with username/email and password
 */
export const LOGIN_MUTATION = gql`
  mutation Login($usernameOrEmail: String!, $password: String!, $rememberMe: Boolean) {
    login(usernameOrEmail: $usernameOrEmail, password: $password, rememberMe: $rememberMe) {
      user {
        id
        username
        email
        displayName
        avatarUrl
        role
        emailVerified
        createdAt
        updatedAt
        lastLoginAt
      }
      accessToken
      refreshToken
      expiresIn
    }
  }
`

/**
 * Register mutation
 * Creates a new user account
 */
export const REGISTER_MUTATION = gql`
  mutation Register($username: String!, $email: String!, $password: String!, $displayName: String) {
    register(username: $username, email: $email, password: $password, displayName: $displayName) {
      user {
        id
        username
        email
        displayName
        avatarUrl
        role
        emailVerified
        createdAt
        updatedAt
        lastLoginAt
      }
      accessToken
      refreshToken
      expiresIn
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
 */
export const REFRESH_TOKEN_MUTATION = gql`
  mutation RefreshToken {
    refreshToken {
      user {
        id
        username
        email
        displayName
        avatarUrl
        role
        emailVerified
        createdAt
        updatedAt
        lastLoginAt
      }
      accessToken
      refreshToken
      expiresIn
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
      username
      email
      displayName
      avatarUrl
      role
      emailVerified
      createdAt
      updatedAt
      lastLoginAt
    }
  }
`
