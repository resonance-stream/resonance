/**
 * TanStack Query hooks for setup wizard
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import {
  SETUP_STATUS_QUERY,
  CREATE_INITIAL_ADMIN_MUTATION,
  COMPLETE_SETUP_MUTATION,
  UPDATE_SYSTEM_SETTING_MUTATION,
  TEST_SERVICE_CONNECTION_MUTATION,
} from '../lib/graphql/setup'
import type {
  SetupStatus,
  CreateAdminInput,
  ServiceType,
  UpdateSystemSettingInput,
  ConnectionTestResult,
  SystemSettingInfo,
  AuthPayload,
} from '@resonance/shared-types'

// ============================================================================
// Types
// ============================================================================

interface SetupStatusResponse {
  setupStatus: SetupStatus
}

interface CreateInitialAdminResponse {
  createInitialAdmin: AuthPayload
}

interface CompleteSetupResponse {
  completeSetup: boolean
}

interface UpdateSystemSettingResponse {
  updateSystemSetting: SystemSettingInfo
}

interface TestServiceConnectionResponse {
  testServiceConnection: ConnectionTestResult
}

// ============================================================================
// Query Keys
// ============================================================================

export const setupKeys = {
  all: ['setup'] as const,
  status: () => [...setupKeys.all, 'status'] as const,
}

// ============================================================================
// Queries
// ============================================================================

/**
 * Hook to fetch setup status (no auth required)
 * Used by SetupGuard to determine if setup wizard should be shown
 */
export function useSetupStatus() {
  return useQuery({
    queryKey: setupKeys.status(),
    queryFn: async () => {
      const data = await graphqlClient.request<SetupStatusResponse>(SETUP_STATUS_QUERY)
      return data.setupStatus
    },
    // Setup status is relatively static, so we can cache it
    staleTime: 5 * 60 * 1000, // 5 minutes
    retry: 2,
  })
}

// ============================================================================
// Mutations
// ============================================================================

/**
 * Hook to create the initial admin user
 */
export function useCreateInitialAdmin() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (input: CreateAdminInput) => {
      const data = await graphqlClient.request<CreateInitialAdminResponse>(
        CREATE_INITIAL_ADMIN_MUTATION,
        { input }
      )
      return data.createInitialAdmin
    },
    onSuccess: () => {
      // Invalidate setup status since hasAdmin will change
      queryClient.invalidateQueries({ queryKey: setupKeys.status() })
    },
  })
}

/**
 * Hook to complete the setup process
 */
export function useCompleteSetup() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async () => {
      const data = await graphqlClient.request<CompleteSetupResponse>(COMPLETE_SETUP_MUTATION)
      return data.completeSetup
    },
    onSuccess: () => {
      // Invalidate setup status since isComplete will change
      queryClient.invalidateQueries({ queryKey: setupKeys.status() })
    },
  })
}

/**
 * Hook to update a system setting
 */
export function useUpdateSystemSetting() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (input: UpdateSystemSettingInput) => {
      const data = await graphqlClient.request<UpdateSystemSettingResponse>(
        UPDATE_SYSTEM_SETTING_MUTATION,
        { input }
      )
      return data.updateSystemSetting
    },
    onSuccess: () => {
      // Invalidate setup status since configuredServices may change
      queryClient.invalidateQueries({ queryKey: setupKeys.status() })
    },
  })
}

/**
 * Hook to test connection to an external service
 */
export function useTestServiceConnection() {
  return useMutation({
    mutationFn: async (service: ServiceType) => {
      const data = await graphqlClient.request<TestServiceConnectionResponse>(
        TEST_SERVICE_CONNECTION_MUTATION,
        { service }
      )
      return data.testServiceConnection
    },
  })
}
