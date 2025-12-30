/**
 * TanStack Query hooks for integration settings
 *
 * Provides type-safe data fetching for external service integrations:
 * - ListenBrainz scrobbling
 * - Discord Rich Presence
 */

import { useQuery, useMutation, useQueryClient, UseQueryOptions, UseMutationOptions } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import { integrationKeys } from '../lib/queryKeys'
import {
  INTEGRATIONS_QUERY,
  UPDATE_INTEGRATIONS_MUTATION,
  SUBMIT_SCROBBLE_MUTATION,
  TEST_LISTENBRAINZ_CONNECTION_MUTATION,
} from '../lib/graphql/integrations'
import type {
  IntegrationsPayload,
  IntegrationsQueryResponse,
  UpdateIntegrationsResponse,
  UpdateIntegrationsInput,
  SubmitScrobbleResponse,
  ScrobbleInput,
  ScrobbleResult,
  TestListenbrainzConnectionResponse,
  ConnectionTestResult,
} from '../types/integrations'

// Stale time for integration settings (rarely changes)
const STALE_TIME = 5 * 60 * 1000 // 5 minutes

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch current integration settings
 *
 * Returns enabled states and connection info (never exposes tokens)
 */
export function useIntegrations(
  options?: Omit<UseQueryOptions<IntegrationsPayload, Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: integrationKeys.settings(),
    queryFn: async () => {
      const response = await graphqlClient.request<IntegrationsQueryResponse>(
        INTEGRATIONS_QUERY
      )
      return response.integrations
    },
    staleTime: STALE_TIME,
    ...options,
  })
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Update integration settings
 *
 * Automatically invalidates the integrations query on success.
 */
export function useUpdateIntegrations(
  options?: Omit<UseMutationOptions<IntegrationsPayload, Error, UpdateIntegrationsInput>, 'mutationFn'>
) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (input: UpdateIntegrationsInput) => {
      const response = await graphqlClient.request<UpdateIntegrationsResponse>(
        UPDATE_INTEGRATIONS_MUTATION,
        { input }
      )
      return response.updateIntegrations
    },
    onSuccess: (data) => {
      // Update the cache with new data
      queryClient.setQueryData(integrationKeys.settings(), data)
    },
    ...options,
  })
}

/**
 * Submit a scrobble to ListenBrainz
 *
 * Called when scrobble threshold is reached during playback.
 */
export function useSubmitScrobble(
  options?: Omit<UseMutationOptions<ScrobbleResult, Error, ScrobbleInput>, 'mutationFn'>
) {
  return useMutation({
    mutationFn: async (input: ScrobbleInput) => {
      const response = await graphqlClient.request<SubmitScrobbleResponse>(
        SUBMIT_SCROBBLE_MUTATION,
        { input }
      )
      return response.submitScrobble
    },
    ...options,
  })
}

/**
 * Test ListenBrainz connection with a token
 *
 * Use this to validate a token before saving with useUpdateIntegrations.
 */
export function useTestListenbrainzConnection(
  options?: Omit<UseMutationOptions<ConnectionTestResult, Error, string>, 'mutationFn'>
) {
  return useMutation({
    mutationFn: async (token: string) => {
      const response = await graphqlClient.request<TestListenbrainzConnectionResponse>(
        TEST_LISTENBRAINZ_CONNECTION_MUTATION,
        { token }
      )
      return response.testListenbrainzConnection
    },
    ...options,
  })
}
