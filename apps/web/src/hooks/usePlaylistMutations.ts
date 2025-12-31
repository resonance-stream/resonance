/**
 * TanStack Query mutation hooks for playlist operations
 *
 * Provides type-safe mutations for creating, updating, and managing playlists.
 * Handles cache invalidation automatically on success.
 */

import { useMutation, useQueryClient, UseMutationOptions } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import { libraryKeys } from '../lib/queryKeys'
import {
  CREATE_PLAYLIST_MUTATION,
  REFRESH_SMART_PLAYLIST_MUTATION,
  UPDATE_PLAYLIST_MUTATION,
  DELETE_PLAYLIST_MUTATION,
  ADD_TRACKS_TO_PLAYLIST_MUTATION,
  REMOVE_TRACKS_FROM_PLAYLIST_MUTATION,
} from '../lib/graphql/playlist'
import type {
  CreatePlaylistInput,
  CreatePlaylistResponse,
  RefreshSmartPlaylistResponse,
  UpdatePlaylistInput,
  UpdatePlaylistResponse,
  UpdatePlaylistVariables,
  DeletePlaylistResponse,
  AddTracksInput,
  AddTracksResponse,
  AddTracksToPlaylistVariables,
  RemoveTracksInput,
  RemoveTracksResponse,
  RemoveTracksFromPlaylistVariables,
} from '../types/playlist'

// Re-export types for convenience
export type {
  CreatePlaylistInput,
  UpdatePlaylistInput,
  AddTracksInput,
  RemoveTracksInput,
}

// ============================================================================
// Create Playlist
// ============================================================================

type CreatePlaylistData = CreatePlaylistResponse['createPlaylist']

/**
 * Create a new playlist (manual or smart)
 *
 * Automatically invalidates the myPlaylists query on success.
 *
 * NOTE: For smart playlists, validate input using validateSmartPlaylistForm()
 * from '../types/playlist' before calling this mutation.
 *
 * @example
 * ```tsx
 * const createPlaylist = useCreatePlaylist({
 *   onSuccess: (data) => navigate(`/playlist/${data.id}`)
 * })
 *
 * createPlaylist.mutate({
 *   name: 'High Energy',
 *   isPublic: false,
 *   playlistType: 'Smart',
 *   smartRules: {
 *     matchMode: 'all',
 *     rules: [{ field: 'energy', operator: 'greater_than', value: 80 }]
 *   }
 * })
 * ```
 */
export function useCreatePlaylist(
  options?: Omit<
    UseMutationOptions<CreatePlaylistData, Error, CreatePlaylistInput>,
    'mutationFn'
  >
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (input: CreatePlaylistInput) => {
      const response = await graphqlClient.request<CreatePlaylistResponse>(
        CREATE_PLAYLIST_MUTATION,
        { input }
      )
      return response.createPlaylist
    },
    ...options,
    onSuccess: (data, variables, context) => {
      // Invalidate all playlist queries (mine, public) to include new playlist
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.all() })
      // Call user's onSuccess if provided
      if (options?.onSuccess) {
        ;(options.onSuccess as (data: CreatePlaylistData, variables: CreatePlaylistInput, context: unknown) => void)(data, variables, context)
      }
    },
  })
}

// ============================================================================
// Refresh Smart Playlist
// ============================================================================

type RefreshSmartPlaylistData = RefreshSmartPlaylistResponse['refreshSmartPlaylist']

/**
 * Refresh a smart playlist to re-evaluate rules against current library
 *
 * Invalidates both the playlist detail and list queries on success.
 *
 * @example
 * ```tsx
 * const refreshPlaylist = useRefreshSmartPlaylist()
 * refreshPlaylist.mutate(playlistId)
 * ```
 */
export function useRefreshSmartPlaylist(
  options?: Omit<
    UseMutationOptions<RefreshSmartPlaylistData, Error, string>,
    'mutationFn'
  >
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (id: string) => {
      const response = await graphqlClient.request<RefreshSmartPlaylistResponse>(
        REFRESH_SMART_PLAYLIST_MUTATION,
        { id }
      )
      return response.refreshSmartPlaylist
    },
    ...options,
    onSuccess: (data, variables, context) => {
      // Invalidate the specific playlist detail
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.detail(variables) })
      // Invalidate list to update track counts
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.all() })
      if (options?.onSuccess) {
        ;(options.onSuccess as (data: RefreshSmartPlaylistData, variables: string, context: unknown) => void)(data, variables, context)
      }
    },
  })
}

// ============================================================================
// Update Playlist
// ============================================================================

type UpdatePlaylistData = UpdatePlaylistResponse['updatePlaylist']

/**
 * Update an existing playlist's metadata or rules
 *
 * @example
 * ```tsx
 * const updatePlaylist = useUpdatePlaylist()
 * updatePlaylist.mutate({
 *   id: playlistId,
 *   input: { name: 'New Name', isPublic: true }
 * })
 * ```
 */
export function useUpdatePlaylist(
  options?: Omit<
    UseMutationOptions<UpdatePlaylistData, Error, UpdatePlaylistVariables>,
    'mutationFn'
  >
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ id, input }: UpdatePlaylistVariables) => {
      const response = await graphqlClient.request<UpdatePlaylistResponse>(
        UPDATE_PLAYLIST_MUTATION,
        { id, input }
      )
      return response.updatePlaylist
    },
    ...options,
    onSuccess: (data, variables, context) => {
      // Invalidate the specific playlist to refetch with updates
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.detail(variables.id) })
      // Invalidate list to update names/metadata
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.all() })
      if (options?.onSuccess) {
        ;(options.onSuccess as (data: UpdatePlaylistData, variables: UpdatePlaylistVariables, context: unknown) => void)(data, variables, context)
      }
    },
  })
}

// ============================================================================
// Delete Playlist
// ============================================================================

/**
 * Delete a playlist
 *
 * @example
 * ```tsx
 * const deletePlaylist = useDeletePlaylist({
 *   onSuccess: () => navigate('/library')
 * })
 * deletePlaylist.mutate(playlistId)
 * ```
 */
export function useDeletePlaylist(
  options?: Omit<UseMutationOptions<boolean, Error, string>, 'mutationFn'>
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (id: string) => {
      const response = await graphqlClient.request<DeletePlaylistResponse>(
        DELETE_PLAYLIST_MUTATION,
        { id }
      )
      return response.deletePlaylist
    },
    ...options,
    onSuccess: (data, variables, context) => {
      // Remove from cache
      queryClient.removeQueries({ queryKey: libraryKeys.playlists.detail(variables) })
      // Invalidate lists
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.all() })
      if (options?.onSuccess) {
        ;(options.onSuccess as (data: boolean, variables: string, context: unknown) => void)(data, variables, context)
      }
    },
  })
}

// ============================================================================
// Add Tracks to Playlist
// ============================================================================

type AddTracksData = AddTracksResponse['addTracksToPlaylist']

/**
 * Add tracks to a manual playlist
 *
 * @example
 * ```tsx
 * const addTracks = useAddTracksToPlaylist()
 * addTracks.mutate({
 *   playlistId,
 *   input: { trackIds: ['track-1', 'track-2'] }
 * })
 * ```
 */
export function useAddTracksToPlaylist(
  options?: Omit<
    UseMutationOptions<AddTracksData, Error, AddTracksToPlaylistVariables>,
    'mutationFn'
  >
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ playlistId, input }: AddTracksToPlaylistVariables) => {
      const response = await graphqlClient.request<AddTracksResponse>(
        ADD_TRACKS_TO_PLAYLIST_MUTATION,
        { playlistId, input }
      )
      return response.addTracksToPlaylist
    },
    ...options,
    onSuccess: (data, variables, context) => {
      // Invalidate the playlist to refetch with new tracks
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.detail(variables.playlistId) })
      // Invalidate list to update track counts
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.all() })
      if (options?.onSuccess) {
        ;(options.onSuccess as (data: AddTracksData, variables: AddTracksToPlaylistVariables, context: unknown) => void)(data, variables, context)
      }
    },
  })
}

// ============================================================================
// Remove Tracks from Playlist
// ============================================================================

type RemoveTracksData = RemoveTracksResponse['removeTracksFromPlaylist']

/**
 * Remove tracks from a manual playlist
 *
 * @example
 * ```tsx
 * const removeTracks = useRemoveTracksFromPlaylist()
 * removeTracks.mutate({
 *   playlistId,
 *   input: { trackIds: ['track-1'] }
 * })
 * ```
 */
export function useRemoveTracksFromPlaylist(
  options?: Omit<
    UseMutationOptions<RemoveTracksData, Error, RemoveTracksFromPlaylistVariables>,
    'mutationFn'
  >
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ playlistId, input }: RemoveTracksFromPlaylistVariables) => {
      const response = await graphqlClient.request<RemoveTracksResponse>(
        REMOVE_TRACKS_FROM_PLAYLIST_MUTATION,
        { playlistId, input }
      )
      return response.removeTracksFromPlaylist
    },
    ...options,
    onSuccess: (data, variables, context) => {
      // Invalidate the playlist to refetch with updated tracks
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.detail(variables.playlistId) })
      // Invalidate list to update track counts
      queryClient.invalidateQueries({ queryKey: libraryKeys.playlists.all() })
      if (options?.onSuccess) {
        ;(options.onSuccess as (data: RemoveTracksData, variables: RemoveTracksFromPlaylistVariables, context: unknown) => void)(data, variables, context)
      }
    },
  })
}
