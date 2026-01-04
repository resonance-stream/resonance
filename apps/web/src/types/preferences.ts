/**
 * Re-export user preferences types from shared-types package
 *
 * This module re-exports the shared preferences types for use in the frontend.
 * All types are defined in @resonance/shared-types for consistency between
 * the backend GraphQL schema and frontend hooks.
 */

// Re-export all preference types from shared-types
export type {
  Theme,
  AudioQuality,
  GraphQLUserPreferences as UserPreferences,
  UpdatePreferencesInput,
  UserWithPreferences,
  UpdatePreferencesResponse,
  ResetPreferencesResponse,
  GetPreferencesResponse,
  UserPreferencesQueryResponse,
} from '@resonance/shared-types';

// Re-export validation constants and helpers
export {
  DEFAULT_GRAPHQL_PREFERENCES as DEFAULT_PREFERENCES,
  VALID_THEMES,
  VALID_QUALITIES,
  MAX_CROSSFADE_MS,
  isValidTheme,
  isValidQuality,
  isValidCrossfade,
} from '@resonance/shared-types';
