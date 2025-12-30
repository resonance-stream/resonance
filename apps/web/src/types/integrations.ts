/**
 * TypeScript types for external service integrations
 *
 * These types mirror the GraphQL schema for:
 * - ListenBrainz scrobbling
 * - Discord Rich Presence
 */

// ============================================================================
// GraphQL Response Types
// ============================================================================

/**
 * Integration settings payload from backend
 */
export interface IntegrationsPayload {
  hasListenbrainzToken: boolean
  listenbrainzEnabled: boolean
  listenbrainzUsername: string | null
  discordRpcEnabled: boolean
}

/**
 * Response from integrations query
 */
export interface IntegrationsQueryResponse {
  integrations: IntegrationsPayload
}

/**
 * Response from updateIntegrations mutation
 */
export interface UpdateIntegrationsResponse {
  updateIntegrations: IntegrationsPayload
}

/**
 * Result of a scrobble submission
 */
export interface ScrobbleResult {
  success: boolean
  error: string | null
}

/**
 * Response from submitScrobble mutation
 */
export interface SubmitScrobbleResponse {
  submitScrobble: ScrobbleResult
}

/**
 * Result of ListenBrainz connection test
 */
export interface ConnectionTestResult {
  valid: boolean
  username: string | null
  error: string | null
}

/**
 * Response from testListenbrainzConnection mutation
 */
export interface TestListenbrainzConnectionResponse {
  testListenbrainzConnection: ConnectionTestResult
}

// ============================================================================
// Input Types
// ============================================================================

/**
 * Input for updating integration settings
 */
export interface UpdateIntegrationsInput {
  listenbrainzToken?: string | null
  listenbrainzEnabled?: boolean | null
  discordRpcEnabled?: boolean | null
}

/**
 * Input for submitting a scrobble
 */
export interface ScrobbleInput {
  trackId: string
  playedAt: string // ISO 8601 datetime
  durationPlayed: number // seconds
}
