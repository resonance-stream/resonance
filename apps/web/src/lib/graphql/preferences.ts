/**
 * GraphQL mutations for user preferences management
 *
 * Provides mutations for updating and resetting user preferences:
 * - updatePreferences: Update specific preference fields
 * - resetPreferences: Reset all preferences to defaults
 * - getPreferences: Fetch current preferences (convenience query as mutation)
 */

import { gql } from 'graphql-request'

// ============ User Preferences Fragment ============

/**
 * Fragment for user preferences fields
 */
export const USER_PREFERENCES_FRAGMENT = gql`
  fragment UserPreferencesFields on UserPreferencesType {
    theme
    quality
    crossfadeDurationMs
    gaplessPlayback
    normalizeVolume
    showExplicit
    privateSession
    discordRpc
    listenbrainzScrobble
  }
`

// ============ Preferences Mutations ============

/**
 * Update user preferences
 *
 * All fields are optional - only provided fields will be updated.
 * Returns the updated user with new preferences.
 *
 * @example
 * ```graphql
 * mutation UpdatePreferences($input: UpdatePreferencesInput!) {
 *   updatePreferences(input: $input) {
 *     id
 *     preferences { ...UserPreferencesFields }
 *   }
 * }
 * ```
 */
export const UPDATE_PREFERENCES_MUTATION = gql`
  ${USER_PREFERENCES_FRAGMENT}
  mutation UpdatePreferences($input: UpdatePreferencesInput!) {
    updatePreferences(input: $input) {
      id
      preferences {
        ...UserPreferencesFields
      }
    }
  }
`

/**
 * Reset all user preferences to default values
 *
 * Default values:
 * - theme: "dark"
 * - quality: "high"
 * - crossfadeDurationMs: 0
 * - gaplessPlayback: true
 * - normalizeVolume: false
 * - showExplicit: true
 * - privateSession: false
 * - discordRpc: true
 * - listenbrainzScrobble: false
 */
export const RESET_PREFERENCES_MUTATION = gql`
  ${USER_PREFERENCES_FRAGMENT}
  mutation ResetPreferences {
    resetPreferences {
      id
      preferences {
        ...UserPreferencesFields
      }
    }
  }
`

/**
 * Get current user preferences
 *
 * Convenience query exposed as mutation for API consistency
 */
export const GET_PREFERENCES_MUTATION = gql`
  ${USER_PREFERENCES_FRAGMENT}
  mutation GetPreferences {
    getPreferences {
      ...UserPreferencesFields
    }
  }
`

// ============ Query for fetching preferences ============

/**
 * Query to fetch user preferences via the me query
 *
 * This is the standard way to fetch preferences through the user query
 */
export const USER_PREFERENCES_QUERY = gql`
  ${USER_PREFERENCES_FRAGMENT}
  query UserPreferences {
    me {
      id
      preferences {
        ...UserPreferencesFields
      }
    }
  }
`
