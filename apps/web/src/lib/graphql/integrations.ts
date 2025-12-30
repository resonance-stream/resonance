/**
 * GraphQL integration operations for Resonance
 *
 * Contains queries and mutations for external service integrations:
 * - ListenBrainz scrobbling
 * - Discord Rich Presence
 */

import { gql } from 'graphql-request'

/**
 * Get current integration settings
 * Returns enabled states and connection status (never exposes tokens)
 */
export const INTEGRATIONS_QUERY = gql`
  query Integrations {
    integrations {
      hasListenbrainzToken
      listenbrainzEnabled
      listenbrainzUsername
      discordRpcEnabled
    }
  }
`

/**
 * Update integration settings
 * Token is validated before being saved; empty string removes token
 */
export const UPDATE_INTEGRATIONS_MUTATION = gql`
  mutation UpdateIntegrations($input: UpdateIntegrationsInput!) {
    updateIntegrations(input: $input) {
      hasListenbrainzToken
      listenbrainzEnabled
      listenbrainzUsername
      discordRpcEnabled
    }
  }
`

/**
 * Submit a scrobble to ListenBrainz
 * Called by the frontend when scrobble threshold is reached
 */
export const SUBMIT_SCROBBLE_MUTATION = gql`
  mutation SubmitScrobble($input: ScrobbleInput!) {
    submitScrobble(input: $input) {
      success
      error
    }
  }
`

/**
 * Test ListenBrainz connection with a token
 * Does not save the token - use updateIntegrations to save
 */
export const TEST_LISTENBRAINZ_CONNECTION_MUTATION = gql`
  mutation TestListenbrainzConnection($token: String!) {
    testListenbrainzConnection(token: $token) {
      valid
      username
      error
    }
  }
`
