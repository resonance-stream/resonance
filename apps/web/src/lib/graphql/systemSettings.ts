/**
 * System settings GraphQL queries and mutations (admin-only)
 */

import { gql } from 'graphql-request'

// ============================================================================
// Queries
// ============================================================================

/**
 * Get all system settings (admin-only)
 */
export const SYSTEM_SETTINGS_QUERY = gql`
  query SystemSettings {
    systemSettings {
      service
      enabled
      config
      hasSecret
      lastConnectionTest
      connectionHealthy
      connectionError
    }
  }
`

/**
 * Get a specific system setting (admin-only)
 */
export const SYSTEM_SETTING_QUERY = gql`
  query SystemSetting($service: ServiceType!) {
    systemSetting(service: $service) {
      service
      enabled
      config
      hasSecret
      lastConnectionTest
      connectionHealthy
      connectionError
    }
  }
`

// ============================================================================
// Mutations
// ============================================================================

/**
 * Update a system setting (admin-only)
 */
export const UPDATE_SYSTEM_SETTING_MUTATION = gql`
  mutation UpdateSystemSetting($input: UpdateSystemSettingInput!) {
    updateSystemSetting(input: $input) {
      service
      enabled
      config
      hasSecret
      lastConnectionTest
      connectionHealthy
      connectionError
    }
  }
`

/**
 * Test connection to an external service (admin-only)
 */
export const TEST_SERVICE_CONNECTION_MUTATION = gql`
  mutation TestServiceConnection($service: ServiceType!) {
    testServiceConnection(service: $service) {
      success
      responseTimeMs
      version
      error
    }
  }
`
