/**
 * TanStack Query hooks for user library path management
 *
 * Provides type-safe data fetching for user-specific library paths:
 * - Query user's library paths
 * - Add new library path
 * - Remove library path
 * - Set primary library path
 * - Update library path label
 */

import { useQuery, useMutation, useQueryClient, UseQueryOptions, UseMutationOptions } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import { libraryPathKeys } from '../lib/queryKeys'
import {
  USER_LIBRARY_PATHS_QUERY,
  ADD_USER_LIBRARY_PATH_MUTATION,
  REMOVE_USER_LIBRARY_PATH_MUTATION,
  SET_USER_PRIMARY_LIBRARY_MUTATION,
  UPDATE_USER_LIBRARY_PATH_MUTATION,
} from '../lib/graphql/libraryPaths'
import type { UserLibraryPath } from '@resonance/shared-types'

// Stale time for library paths (rarely changes)
const STALE_TIME = 5 * 60 * 1000 // 5 minutes

// ============================================================================
// Response Types
// ============================================================================

interface UserLibraryPathsQueryResponse {
  userLibraryPaths: UserLibraryPath[]
}

interface AddUserLibraryPathResponse {
  addUserLibraryPath: UserLibraryPath
}

interface RemoveUserLibraryPathResponse {
  removeUserLibraryPath: boolean
}

interface SetUserPrimaryLibraryResponse {
  setUserPrimaryLibrary: UserLibraryPath
}

interface UpdateUserLibraryPathResponse {
  updateUserLibraryPath: UserLibraryPath
}

// ============================================================================
// Input Types
// ============================================================================

export interface AddLibraryPathInput {
  path: string
  label?: string
}

export interface UpdateLibraryPathInput {
  id: string
  label: string
}

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch current user's library paths
 *
 * Returns list of configured library paths with primary indicator
 */
export function useUserLibraryPaths(
  options?: Omit<UseQueryOptions<UserLibraryPath[], Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryPathKeys.list(),
    queryFn: async () => {
      const response = await graphqlClient.request<UserLibraryPathsQueryResponse>(
        USER_LIBRARY_PATHS_QUERY
      )
      return response.userLibraryPaths
    },
    staleTime: STALE_TIME,
    ...options,
  })
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Add a new library path
 *
 * Automatically invalidates the library paths query on success.
 */
export function useAddLibraryPath(
  options?: Omit<UseMutationOptions<UserLibraryPath, Error, AddLibraryPathInput, unknown>, 'mutationFn'>
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (input: AddLibraryPathInput) => {
      const response = await graphqlClient.request<AddUserLibraryPathResponse>(
        ADD_USER_LIBRARY_PATH_MUTATION,
        { path: input.path, label: input.label }
      )
      return response.addUserLibraryPath
    },
    ...options,
    onSuccess: (data, variables, context) => {
      // Optimistically add to cache
      queryClient.setQueryData<UserLibraryPath[]>(libraryPathKeys.list(), (old) => {
        if (!old) return [data]
        return [...old, data]
      })
      // Call user's onSuccess if provided
      if (options?.onSuccess) {
        ;(options.onSuccess as (data: UserLibraryPath, variables: AddLibraryPathInput, context: unknown) => void)(data, variables, context)
      }
    },
  })
}

/**
 * Remove a library path
 *
 * Automatically invalidates the library paths query on success.
 */
export function useRemoveLibraryPath(
  options?: Omit<UseMutationOptions<boolean, Error, string, { previous: UserLibraryPath[] | undefined }>, 'mutationFn'>
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (id: string) => {
      const response = await graphqlClient.request<RemoveUserLibraryPathResponse>(
        REMOVE_USER_LIBRARY_PATH_MUTATION,
        { id }
      )
      return response.removeUserLibraryPath
    },
    // Optimistic update
    onMutate: async (id) => {
      // Cancel outgoing refetches
      await queryClient.cancelQueries({ queryKey: libraryPathKeys.list() })

      // Snapshot the previous value
      const previous = queryClient.getQueryData<UserLibraryPath[]>(libraryPathKeys.list())

      // Optimistically remove from cache
      queryClient.setQueryData<UserLibraryPath[]>(libraryPathKeys.list(), (old) => {
        if (!old) return []
        return old.filter((path) => path.id !== id)
      })

      return { previous }
    },
    onError: (_err, _id, context) => {
      // Rollback on error
      if (context?.previous) {
        queryClient.setQueryData(libraryPathKeys.list(), context.previous)
      }
    },
    onSettled: () => {
      // Refetch to ensure consistency
      queryClient.invalidateQueries({ queryKey: libraryPathKeys.list() })
    },
    ...options,
  })
}

/**
 * Set a library path as primary
 *
 * Automatically updates the cache with the new primary indicator.
 */
export function useSetPrimaryLibraryPath(
  options?: Omit<UseMutationOptions<UserLibraryPath, Error, string, { previous: UserLibraryPath[] | undefined }>, 'mutationFn'>
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (id: string) => {
      const response = await graphqlClient.request<SetUserPrimaryLibraryResponse>(
        SET_USER_PRIMARY_LIBRARY_MUTATION,
        { id }
      )
      return response.setUserPrimaryLibrary
    },
    // Optimistic update
    onMutate: async (id) => {
      await queryClient.cancelQueries({ queryKey: libraryPathKeys.list() })

      const previous = queryClient.getQueryData<UserLibraryPath[]>(libraryPathKeys.list())

      // Optimistically update primary status
      queryClient.setQueryData<UserLibraryPath[]>(libraryPathKeys.list(), (old) => {
        if (!old) return []
        return old.map((path) => ({
          ...path,
          isPrimary: path.id === id,
        }))
      })

      return { previous }
    },
    onError: (_err, _id, context) => {
      if (context?.previous) {
        queryClient.setQueryData(libraryPathKeys.list(), context.previous)
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: libraryPathKeys.list() })
    },
    ...options,
  })
}

/**
 * Update a library path's label
 *
 * Automatically updates the cache with the new label.
 */
export function useUpdateLibraryPath(
  options?: Omit<UseMutationOptions<UserLibraryPath, Error, UpdateLibraryPathInput, { previous: UserLibraryPath[] | undefined }>, 'mutationFn'>
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (input: UpdateLibraryPathInput) => {
      const response = await graphqlClient.request<UpdateUserLibraryPathResponse>(
        UPDATE_USER_LIBRARY_PATH_MUTATION,
        { id: input.id, label: input.label }
      )
      return response.updateUserLibraryPath
    },
    // Optimistic update
    onMutate: async (input) => {
      await queryClient.cancelQueries({ queryKey: libraryPathKeys.list() })

      const previous = queryClient.getQueryData<UserLibraryPath[]>(libraryPathKeys.list())

      // Optimistically update the label
      queryClient.setQueryData<UserLibraryPath[]>(libraryPathKeys.list(), (old) => {
        if (!old) return []
        return old.map((path) =>
          path.id === input.id ? { ...path, label: input.label } : path
        )
      })

      return { previous }
    },
    onError: (_err, _input, context) => {
      if (context?.previous) {
        queryClient.setQueryData(libraryPathKeys.list(), context.previous)
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: libraryPathKeys.list() })
    },
    ...options,
  })
}
