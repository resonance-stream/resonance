/**
 * Setup wizard GraphQL queries and mutations
 */

import { gql } from 'graphql-request'

// ============================================================================
// Queries
// ============================================================================

/**
 * Query to check if setup has been completed (no auth required)
 */
export const SETUP_STATUS_QUERY = gql`
  query SetupStatus {
    setupStatus {
      isComplete
      hasAdmin
      configuredServices
    }
  }
`

// ============================================================================
// Mutations
// ============================================================================

/**
 * Create the initial admin user during first-time setup
 */
export const CREATE_INITIAL_ADMIN_MUTATION = gql`
  mutation CreateInitialAdmin($input: CreateAdminInput!) {
    createInitialAdmin(input: $input) {
      user {
        id
        email
        displayName
        role
      }
      accessToken
      refreshToken
      expiresIn
    }
  }
`

/**
 * Mark setup as complete
 */
export const COMPLETE_SETUP_MUTATION = gql`
  mutation CompleteSetup {
    completeSetup
  }
`

/**
 * Update a system setting (used during setup for configuring services)
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
 * Test connection to an external service
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
