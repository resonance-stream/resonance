/**
 * TanStack Query hooks for system settings (admin-only)
 */

import { useQuery, useMutation, useQueryClient, UseQueryOptions } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import {
  SYSTEM_SETTINGS_QUERY,
  TEST_SERVICE_CONNECTION_MUTATION,
  UPDATE_SYSTEM_SETTING_MUTATION,
} from '../lib/graphql/systemSettings'
import type {
  SystemSettingInfo,
  ConnectionTestResult,
  ServiceType,
  SystemSettingsResponse,
  UpdateSystemSettingResponse,
  TestServiceConnectionResponse,
} from '../types/systemSettings'

// Query keys for system settings
export const systemSettingsKeys = {
  all: ['systemSettings'] as const,
  list: () => [...systemSettingsKeys.all, 'list'] as const,
  detail: (service: ServiceType) => [...systemSettingsKeys.all, 'detail', service] as const,
}

// Stale time for settings data
const STALE_TIME = 30 * 1000 // 30 seconds

// ============================================================================
// Queries
// ============================================================================

/**
 * Hook to fetch all system settings (admin-only)
 */
export function useSystemSettings(
  options?: Omit<UseQueryOptions<SystemSettingInfo[], Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: systemSettingsKeys.list(),
    queryFn: async () => {
      const data = await graphqlClient.request<SystemSettingsResponse>(
        SYSTEM_SETTINGS_QUERY
      )
      return data.systemSettings
    },
    staleTime: STALE_TIME,
    ...options,
  })
}

// ============================================================================
// Mutations
// ============================================================================

/**
 * Hook to update a system setting
 */
export function useUpdateSystemSetting() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      service,
      enabled,
      config,
      secret,
    }: {
      service: ServiceType
      enabled?: boolean
      config?: string
      secret?: string
    }) => {
      const data = await graphqlClient.request<UpdateSystemSettingResponse>(
        UPDATE_SYSTEM_SETTING_MUTATION,
        { input: { service, enabled, config, secret } }
      )
      return data.updateSystemSetting
    },
    onSuccess: () => {
      // Invalidate all settings queries to refresh the list
      queryClient.invalidateQueries({ queryKey: systemSettingsKeys.all })
    },
  })
}

/**
 * Hook to test a service connection
 */
export function useTestServiceConnection() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (service: ServiceType): Promise<ConnectionTestResult> => {
      const data = await graphqlClient.request<TestServiceConnectionResponse>(
        TEST_SERVICE_CONNECTION_MUTATION,
        { service }
      )
      return data.testServiceConnection
    },
    onSuccess: () => {
      // Invalidate settings to refresh connection status
      queryClient.invalidateQueries({ queryKey: systemSettingsKeys.all })
    },
  })
}

/**
 * Hook to test all service connections
 */
export function useTestAllConnections() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (services: ServiceType[]) => {
      const results: Record<ServiceType, ConnectionTestResult> = {} as Record<
        ServiceType,
        ConnectionTestResult
      >

      // Test all services in parallel
      const testPromises = services.map(async (service) => {
        try {
          const result = await graphqlClient.request<TestServiceConnectionResponse>(
            TEST_SERVICE_CONNECTION_MUTATION,
            { service }
          )
          results[service] = result.testServiceConnection
        } catch (error) {
          results[service] = {
            success: false,
            responseTimeMs: null,
            version: null,
            error: error instanceof Error ? error.message : 'Unknown error',
          }
        }
      })

      await Promise.all(testPromises)
      return results
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: systemSettingsKeys.all })
    },
  })
}
