/**
 * TanStack Query hooks for user preferences management
 *
 * Provides type-safe mutations for user preference CRUD operations
 * with optimistic updates for responsive UI experience.
 *
 * Features:
 * - Optimistic updates for instant UI feedback
 * - Automatic rollback on mutation failure
 * - Cache synchronization with server state
 * - Settings store sync for local playback settings
 */

import {
  useQuery,
  useMutation,
  useQueryClient,
  UseQueryOptions,
  UseMutationOptions,
} from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import { preferencesKeys } from '../lib/queryKeys'
import {
  UPDATE_PREFERENCES_MUTATION,
  RESET_PREFERENCES_MUTATION,
  USER_PREFERENCES_QUERY,
} from '../lib/graphql/preferences'
import { useSettingsStore } from '../stores/settingsStore'
import type {
  UserPreferences,
  UpdatePreferencesInput,
  UpdatePreferencesResponse,
  ResetPreferencesResponse,
  UserPreferencesQueryResponse,
} from '../types/preferences'

// Re-export types for convenience
export type { UserPreferences, UpdatePreferencesInput }

// Default stale time for preferences (5 minutes)
const STALE_TIME = 5 * 60 * 1000

// ============================================================================
// Query Hook
// ============================================================================

/**
 * Fetch user preferences
 *
 * Returns the current user's preferences from the server.
 * Used to sync server preferences with local settings.
 *
 * @example
 * ```tsx
 * const { data: preferences, isLoading } = useUserPreferences()
 * console.log(preferences?.theme) // "dark" | "light"
 * ```
 */
export function useUserPreferences(
  options?: Omit<
    UseQueryOptions<UserPreferences | null, Error>,
    'queryKey' | 'queryFn'
  >
) {
  return useQuery({
    queryKey: preferencesKeys.user(),
    queryFn: async () => {
      const response = await graphqlClient.request<UserPreferencesQueryResponse>(
        USER_PREFERENCES_QUERY
      )
      return response.me?.preferences ?? null
    },
    staleTime: STALE_TIME,
    ...options,
  })
}

// ============================================================================
// Mutation Hooks
// ============================================================================

type UpdatePreferencesData = UpdatePreferencesResponse['updatePreferences']
type ResetPreferencesData = ResetPreferencesResponse['resetPreferences']

/**
 * Update user preferences with optimistic updates
 *
 * Provides immediate UI feedback while syncing with the server.
 * Automatically rolls back on failure and syncs with settings store.
 *
 * @example
 * ```tsx
 * const updatePreferences = useUpdatePreferences()
 *
 * // Update single preference
 * updatePreferences.mutate({ theme: 'light' })
 *
 * // Update multiple preferences
 * updatePreferences.mutate({
 *   quality: 'lossless',
 *   gaplessPlayback: true,
 *   crossfadeDurationMs: 3000,
 * })
 * ```
 */
/** Context type for optimistic update rollback */
interface UpdatePreferencesContext {
  previousPreferences: UserPreferences | undefined
}

export function useUpdatePreferences(
  options?: Omit<
    UseMutationOptions<UpdatePreferencesData, Error, UpdatePreferencesInput, UpdatePreferencesContext>,
    'mutationFn'
  >
) {
  const queryClient = useQueryClient()
  const syncToSettingsStore = useSyncToSettingsStore()

  return useMutation<UpdatePreferencesData, Error, UpdatePreferencesInput, UpdatePreferencesContext>({
    mutationFn: async (input: UpdatePreferencesInput) => {
      const response = await graphqlClient.request<UpdatePreferencesResponse>(
        UPDATE_PREFERENCES_MUTATION,
        { input }
      )
      return response.updatePreferences
    },
    // Optimistic update
    onMutate: async (input): Promise<UpdatePreferencesContext> => {
      // Cancel any outgoing refetches to avoid overwriting optimistic update
      await queryClient.cancelQueries({ queryKey: preferencesKeys.user() })

      // Snapshot the previous value
      const previousPreferences = queryClient.getQueryData<UserPreferences>(
        preferencesKeys.user()
      )

      // Optimistically update to the new value
      if (previousPreferences) {
        const optimisticPreferences: UserPreferences = {
          ...previousPreferences,
          ...(input.theme !== undefined && { theme: input.theme as 'dark' | 'light' }),
          ...(input.quality !== undefined && { quality: input.quality as UserPreferences['quality'] }),
          ...(input.crossfadeDurationMs !== undefined && { crossfadeDurationMs: input.crossfadeDurationMs }),
          ...(input.gaplessPlayback !== undefined && { gaplessPlayback: input.gaplessPlayback }),
          ...(input.normalizeVolume !== undefined && { normalizeVolume: input.normalizeVolume }),
          ...(input.showExplicit !== undefined && { showExplicit: input.showExplicit }),
          ...(input.privateSession !== undefined && { privateSession: input.privateSession }),
          ...(input.discordRpc !== undefined && { discordRpc: input.discordRpc }),
          ...(input.listenbrainzScrobble !== undefined && { listenbrainzScrobble: input.listenbrainzScrobble }),
        }

        queryClient.setQueryData<UserPreferences>(
          preferencesKeys.user(),
          optimisticPreferences
        )
      }

      // Return context with the previous value
      return { previousPreferences }
    },
    // On error, roll back to the previous value
    onError: (_error, _input, context) => {
      if (context?.previousPreferences) {
        queryClient.setQueryData<UserPreferences>(
          preferencesKeys.user(),
          context.previousPreferences
        )
      }
    },
    // On success, update the cache with server response and sync to settings store
    onSuccess: (data) => {
      // Update cache with server-confirmed data
      queryClient.setQueryData<UserPreferences>(
        preferencesKeys.user(),
        data.preferences
      )

      // Sync playback-related preferences to settings store
      syncToSettingsStore(data.preferences)
    },
    // Always refetch after error or success
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: preferencesKeys.user() })
    },
    ...options,
  })
}

/** Context type for reset preferences rollback */
interface ResetPreferencesContext {
  previousPreferences: UserPreferences | undefined
}

/**
 * Reset all user preferences to default values
 *
 * Resets all preferences to server defaults and syncs with settings store.
 *
 * @example
 * ```tsx
 * const resetPreferences = useResetPreferences()
 * resetPreferences.mutate()
 * ```
 */
export function useResetPreferences(
  options?: Omit<UseMutationOptions<ResetPreferencesData, Error, void, ResetPreferencesContext>, 'mutationFn'>
) {
  const queryClient = useQueryClient()
  const syncToSettingsStore = useSyncToSettingsStore()
  const resetSettingsStore = useSettingsStore((s) => s.resetToDefaults)

  return useMutation<ResetPreferencesData, Error, void, ResetPreferencesContext>({
    mutationFn: async () => {
      const response = await graphqlClient.request<ResetPreferencesResponse>(
        RESET_PREFERENCES_MUTATION
      )
      return response.resetPreferences
    },
    // Optimistic update: set to defaults
    onMutate: async (): Promise<ResetPreferencesContext> => {
      await queryClient.cancelQueries({ queryKey: preferencesKeys.user() })

      const previousPreferences = queryClient.getQueryData<UserPreferences>(
        preferencesKeys.user()
      )

      // Optimistically reset to defaults
      const defaultPrefs: UserPreferences = {
        theme: 'dark',
        quality: 'high',
        crossfadeDurationMs: 0,
        gaplessPlayback: true,
        normalizeVolume: false,
        showExplicit: true,
        privateSession: false,
        discordRpc: true,
        listenbrainzScrobble: false,
      }

      queryClient.setQueryData<UserPreferences>(preferencesKeys.user(), defaultPrefs)

      // Also reset local settings store optimistically
      resetSettingsStore()

      return { previousPreferences }
    },
    onError: (_error, _input, context) => {
      if (context?.previousPreferences) {
        queryClient.setQueryData<UserPreferences>(
          preferencesKeys.user(),
          context.previousPreferences
        )
      }
    },
    onSuccess: (data) => {
      queryClient.setQueryData<UserPreferences>(
        preferencesKeys.user(),
        data.preferences
      )
      syncToSettingsStore(data.preferences)
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: preferencesKeys.user() })
    },
    ...options,
  })
}

// ============================================================================
// Settings Store Sync
// ============================================================================

/**
 * Returns a function to sync server preferences to the local settings store
 *
 * The settings store handles local playback settings (crossfade, gapless, etc.)
 * This function syncs server preferences to keep them in sync.
 */
function useSyncToSettingsStore() {
  const settingsStore = useSettingsStore()

  return (preferences: UserPreferences) => {
    // Sync playback settings
    const crossfadeEnabled = preferences.crossfadeDurationMs > 0
    const crossfadeDuration = Math.round(preferences.crossfadeDurationMs / 1000)

    if (settingsStore.playback.crossfadeEnabled !== crossfadeEnabled) {
      settingsStore.setCrossfadeEnabled(crossfadeEnabled)
    }
    if (settingsStore.playback.crossfadeDuration !== crossfadeDuration) {
      settingsStore.setCrossfadeDuration(crossfadeDuration)
    }
    if (settingsStore.playback.gaplessEnabled !== preferences.gaplessPlayback) {
      settingsStore.setGaplessEnabled(preferences.gaplessPlayback)
    }
    if (settingsStore.playback.normalizeVolume !== preferences.normalizeVolume) {
      settingsStore.setNormalizeVolume(preferences.normalizeVolume)
    }

    // Map server quality to local quality format
    // Server uses: low, medium, high, lossless
    // Local uses: auto, low, normal, high, lossless
    const qualityMap: Record<string, 'auto' | 'low' | 'normal' | 'high' | 'lossless'> = {
      low: 'low',
      medium: 'normal',
      high: 'high',
      lossless: 'lossless',
    }
    const localQuality = qualityMap[preferences.quality] ?? 'high'
    if (settingsStore.audioQuality.quality !== localQuality) {
      settingsStore.setAudioQuality(localQuality)
    }
  }
}

// ============================================================================
// Convenience Hooks
// ============================================================================

/**
 * Get a single preference value with loading state
 *
 * @example
 * ```tsx
 * const [theme, isLoading] = usePreference('theme')
 * ```
 */
export function usePreference<K extends keyof UserPreferences>(
  key: K
): [UserPreferences[K] | undefined, boolean] {
  const { data, isLoading } = useUserPreferences()
  return [data?.[key], isLoading]
}

/**
 * Update a single preference value
 *
 * Returns a function that updates a specific preference.
 *
 * @example
 * ```tsx
 * const updateTheme = useSetPreference('theme')
 * updateTheme('light')
 * ```
 */
export function useSetPreference<K extends keyof UpdatePreferencesInput>(
  key: K
): (value: UpdatePreferencesInput[K]) => void {
  const updatePreferences = useUpdatePreferences()

  return (value: UpdatePreferencesInput[K]) => {
    updatePreferences.mutate({ [key]: value })
  }
}

/**
 * Toggle a boolean preference
 *
 * @example
 * ```tsx
 * const toggleGapless = useTogglePreference('gaplessPlayback')
 * <button onClick={toggleGapless}>Toggle Gapless</button>
 * ```
 */
export function useTogglePreference(
  key: keyof Pick<
    UserPreferences,
    | 'gaplessPlayback'
    | 'normalizeVolume'
    | 'showExplicit'
    | 'privateSession'
    | 'discordRpc'
    | 'listenbrainzScrobble'
  >
): () => void {
  const { data } = useUserPreferences()
  const updatePreferences = useUpdatePreferences()

  return () => {
    if (data) {
      updatePreferences.mutate({ [key]: !data[key] })
    }
  }
}
