/**
 * TanStack Query hooks for admin data fetching
 */

import { useQuery, useMutation, useQueryClient, UseQueryOptions } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import {
  ADMIN_SYSTEM_STATS_QUERY,
  ADMIN_USERS_QUERY,
  ADMIN_USER_QUERY,
  ADMIN_UPDATE_USER_ROLE_MUTATION,
  ADMIN_DELETE_USER_MUTATION,
  ADMIN_INVALIDATE_SESSIONS_MUTATION,
} from '../lib/graphql/admin'
import type {
  AdminSystemStatsResponse,
  AdminUsersResponse,
  AdminUserResponse,
  AdminUpdateUserRoleResponse,
  AdminDeleteUserResponse,
  AdminInvalidateSessionsResponse,
  GqlSystemStats,
  GqlAdminUserList,
  GqlAdminUserDetail,
  GqlAdminUserListItem,
} from '../types/admin'
import type { UserRole } from '@resonance/shared-types'

// Query keys for admin data
export const adminKeys = {
  all: ['admin'] as const,
  stats: () => [...adminKeys.all, 'stats'] as const,
  users: (params?: { limit?: number; offset?: number; search?: string }) =>
    [...adminKeys.all, 'users', params] as const,
  user: (userId: string) => [...adminKeys.all, 'user', userId] as const,
}

// Stale time for admin data
const STALE_TIME = 30 * 1000 // 30 seconds

// ============================================================================
// System Stats
// ============================================================================

export function useAdminSystemStats(
  options?: Omit<UseQueryOptions<GqlSystemStats, Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: adminKeys.stats(),
    queryFn: async () => {
      const data = await graphqlClient.request<AdminSystemStatsResponse>(
        ADMIN_SYSTEM_STATS_QUERY
      )
      return data.adminSystemStats
    },
    staleTime: STALE_TIME,
    ...options,
  })
}

// ============================================================================
// Users List
// ============================================================================

interface UseAdminUsersParams {
  limit?: number
  offset?: number
  search?: string
}

export function useAdminUsers(
  params: UseAdminUsersParams = {},
  options?: Omit<UseQueryOptions<GqlAdminUserList, Error>, 'queryKey' | 'queryFn'>
) {
  const { limit = 20, offset = 0, search } = params

  return useQuery({
    queryKey: adminKeys.users({ limit, offset, search }),
    queryFn: async () => {
      const data = await graphqlClient.request<AdminUsersResponse>(ADMIN_USERS_QUERY, {
        limit,
        offset,
        search: search || null,
      })
      return data.adminUsers
    },
    staleTime: STALE_TIME,
    ...options,
  })
}

// ============================================================================
// User Detail
// ============================================================================

export function useAdminUser(
  userId: string,
  options?: Omit<UseQueryOptions<GqlAdminUserDetail, Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: adminKeys.user(userId),
    queryFn: async () => {
      const data = await graphqlClient.request<AdminUserResponse>(ADMIN_USER_QUERY, {
        userId,
      })
      return data.adminUser
    },
    staleTime: STALE_TIME,
    enabled: !!userId,
    ...options,
  })
}

// ============================================================================
// Mutations
// ============================================================================

export function useUpdateUserRole() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ userId, role }: { userId: string; role: UserRole }) => {
      const data = await graphqlClient.request<AdminUpdateUserRoleResponse>(
        ADMIN_UPDATE_USER_ROLE_MUTATION,
        { input: { userId, role: role.toUpperCase() } }
      )
      return data.adminUpdateUserRole
    },
    onSuccess: (updatedUser: GqlAdminUserListItem) => {
      // Invalidate all users list queries (all paginated/searched variations)
      queryClient.invalidateQueries({ queryKey: ['admin', 'users'] })
      // Update the specific user cache
      queryClient.setQueryData(adminKeys.user(updatedUser.id), (old: GqlAdminUserDetail | undefined) => {
        if (old) {
          return { ...old, user: updatedUser }
        }
        return old
      })
    },
  })
}

export function useDeleteUser() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (userId: string) => {
      const data = await graphqlClient.request<AdminDeleteUserResponse>(
        ADMIN_DELETE_USER_MUTATION,
        { userId }
      )
      return data.adminDeleteUser
    },
    onSuccess: (_result, userId) => {
      // Invalidate all users list queries (all paginated/searched variations)
      queryClient.invalidateQueries({ queryKey: ['admin', 'users'] })
      // Remove user from cache
      queryClient.removeQueries({ queryKey: adminKeys.user(userId) })
    },
  })
}

export function useInvalidateSessions() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (userId: string) => {
      const data = await graphqlClient.request<AdminInvalidateSessionsResponse>(
        ADMIN_INVALIDATE_SESSIONS_MUTATION,
        { userId }
      )
      return data.adminInvalidateSessions
    },
    onSuccess: (_result, userId) => {
      // Invalidate the user detail to refresh session data
      queryClient.invalidateQueries({ queryKey: adminKeys.user(userId) })
      // Invalidate all users list queries (session counts changed)
      queryClient.invalidateQueries({ queryKey: ['admin', 'users'] })
      // Invalidate system stats since active session count changed
      queryClient.invalidateQueries({ queryKey: adminKeys.stats() })
    },
  })
}
