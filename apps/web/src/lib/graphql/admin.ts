/**
 * Admin GraphQL queries and mutations
 */

import { gql } from 'graphql-request'

// ============================================================================
// Queries
// ============================================================================

export const ADMIN_SYSTEM_STATS_QUERY = gql`
  query AdminSystemStats {
    adminSystemStats {
      userCount
      trackCount
      albumCount
      artistCount
      totalDurationMs
      totalFileSizeBytes
      activeSessionCount
      totalDurationFormatted
      totalFileSizeFormatted
    }
  }
`

export const ADMIN_USERS_QUERY = gql`
  query AdminUsers($limit: Int!, $offset: Int!, $search: String) {
    adminUsers(limit: $limit, offset: $offset, search: $search) {
      users {
        id
        email
        displayName
        avatarUrl
        role
        emailVerified
        lastSeenAt
        createdAt
        sessionCount
      }
      totalCount
      hasNextPage
    }
  }
`

export const ADMIN_USER_QUERY = gql`
  query AdminUser($userId: UUID!) {
    adminUser(userId: $userId) {
      user {
        id
        email
        displayName
        avatarUrl
        role
        emailVerified
        lastSeenAt
        createdAt
        sessionCount
      }
      sessions {
        id
        deviceType
        deviceName
        ipAddress
        userAgent
        isActive
        lastActiveAt
        createdAt
      }
    }
  }
`

// ============================================================================
// Mutations
// ============================================================================

export const ADMIN_UPDATE_USER_ROLE_MUTATION = gql`
  mutation AdminUpdateUserRole($input: UpdateUserRoleInput!) {
    adminUpdateUserRole(input: $input) {
      id
      email
      displayName
      avatarUrl
      role
      emailVerified
      lastSeenAt
      createdAt
      sessionCount
    }
  }
`

export const ADMIN_DELETE_USER_MUTATION = gql`
  mutation AdminDeleteUser($userId: UUID!) {
    adminDeleteUser(userId: $userId) {
      success
      message
    }
  }
`

export const ADMIN_INVALIDATE_SESSIONS_MUTATION = gql`
  mutation AdminInvalidateSessions($userId: UUID!) {
    adminInvalidateSessions(userId: $userId) {
      success
      sessionsInvalidated
    }
  }
`
