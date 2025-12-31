/**
 * Admin dashboard types for the frontend
 */

import type { UserRole } from '@resonance/shared-types'

// ============================================================================
// GraphQL Response Types
// ============================================================================

export interface GqlSystemStats {
  userCount: number
  trackCount: number
  albumCount: number
  artistCount: number
  totalDurationMs: number
  totalFileSizeBytes: number
  activeSessionCount: number
  totalDurationFormatted: string
  totalFileSizeFormatted: string
}

export interface GqlAdminUserListItem {
  id: string
  email: string
  displayName: string
  avatarUrl?: string | null
  role: UserRole
  emailVerified: boolean
  lastSeenAt?: string | null
  createdAt: string
  sessionCount: number
}

export interface GqlAdminUserList {
  users: GqlAdminUserListItem[]
  totalCount: number
  hasNextPage: boolean
}

export interface GqlAdminSession {
  id: string
  deviceType?: string | null
  deviceName?: string | null
  ipAddress?: string | null
  userAgent?: string | null
  isActive: boolean
  lastActiveAt: string
  createdAt: string
}

export interface GqlAdminUserDetail {
  user: GqlAdminUserListItem
  sessions: GqlAdminSession[]
}

export interface GqlAdminOperationResult {
  success: boolean
  message?: string | null
}

export interface GqlInvalidateSessionsResult {
  success: boolean
  sessionsInvalidated: number
}

// ============================================================================
// Query Response Types
// ============================================================================

export interface AdminSystemStatsResponse {
  adminSystemStats: GqlSystemStats
}

export interface AdminUsersResponse {
  adminUsers: GqlAdminUserList
}

export interface AdminUserResponse {
  adminUser: GqlAdminUserDetail
}

// ============================================================================
// Mutation Response Types
// ============================================================================

export interface AdminUpdateUserRoleResponse {
  adminUpdateUserRole: GqlAdminUserListItem
}

export interface AdminDeleteUserResponse {
  adminDeleteUser: GqlAdminOperationResult
}

export interface AdminInvalidateSessionsResponse {
  adminInvalidateSessions: GqlInvalidateSessionsResult
}
