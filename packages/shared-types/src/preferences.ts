/**
 * User preferences types for Resonance
 *
 * These types represent the flat preferences structure used in the GraphQL API.
 * They provide type consistency between the backend GraphQL schema and frontend hooks.
 *
 * Note: This differs from the nested UserPreferences in user.ts which represents
 * a more comprehensive settings structure for future use.
 */

// ============================================================================
// Theme and Quality Types
// ============================================================================

/**
 * UI theme options
 */
export type Theme = 'dark' | 'light';

/**
 * Audio quality options
 */
export type AudioQuality = 'low' | 'medium' | 'high' | 'lossless';

// ============================================================================
// GraphQL Preferences Types
// ============================================================================

/**
 * User preferences from GraphQL API
 *
 * This flat structure matches the UserPreferencesType from the backend GraphQL schema.
 * Use this type when working with the preferences GraphQL queries and mutations.
 */
export interface GraphQLUserPreferences {
  /** UI theme: "dark" or "light" */
  theme: Theme;
  /** Audio quality: "low", "medium", "high", "lossless" */
  quality: AudioQuality;
  /** Crossfade duration in milliseconds (0-12000) */
  crossfadeDurationMs: number;
  /** Enable gapless playback between tracks */
  gaplessPlayback: boolean;
  /** Normalize volume across tracks */
  normalizeVolume: boolean;
  /** Show explicit content */
  showExplicit: boolean;
  /** Private listening session (no scrobbling) */
  privateSession: boolean;
  /** Discord Rich Presence integration */
  discordRpc: boolean;
  /** Enable ListenBrainz scrobbling */
  listenbrainzScrobble: boolean;
}

/**
 * Default user preferences
 *
 * These match the backend defaults in UserPreferences::default()
 */
export const DEFAULT_GRAPHQL_PREFERENCES: GraphQLUserPreferences = {
  theme: 'dark',
  quality: 'high',
  crossfadeDurationMs: 0,
  gaplessPlayback: true,
  normalizeVolume: false,
  showExplicit: true,
  privateSession: false,
  discordRpc: true,
  listenbrainzScrobble: false,
};

// ============================================================================
// Input Types
// ============================================================================

/**
 * Input for updating user preferences via GraphQL
 *
 * All fields are optional - only provided fields will be updated.
 * Omitted fields retain their current values.
 */
export interface UpdatePreferencesInput {
  /** UI theme: "dark" or "light" */
  theme?: string;
  /** Audio quality: "low", "medium", "high", "lossless" */
  quality?: string;
  /** Crossfade duration in milliseconds (0-12000) */
  crossfadeDurationMs?: number;
  /** Enable gapless playback between tracks */
  gaplessPlayback?: boolean;
  /** Normalize volume across tracks */
  normalizeVolume?: boolean;
  /** Show explicit content */
  showExplicit?: boolean;
  /** Private listening session (no scrobbling) */
  privateSession?: boolean;
  /** Discord Rich Presence integration */
  discordRpc?: boolean;
  /** Enable ListenBrainz scrobbling */
  listenbrainzScrobble?: boolean;
}

// ============================================================================
// Response Types
// ============================================================================

/**
 * User with preferences from GraphQL mutation responses
 */
export interface UserWithPreferences {
  id: string;
  preferences: GraphQLUserPreferences;
}

/**
 * Response from updatePreferences mutation
 */
export interface UpdatePreferencesResponse {
  updatePreferences: UserWithPreferences;
}

/**
 * Response from resetPreferences mutation
 */
export interface ResetPreferencesResponse {
  resetPreferences: UserWithPreferences;
}

/**
 * Response from getPreferences mutation
 */
export interface GetPreferencesResponse {
  getPreferences: GraphQLUserPreferences;
}

/**
 * Response from userPreferences query (via me query)
 */
export interface UserPreferencesQueryResponse {
  me: UserWithPreferences;
}

// ============================================================================
// Validation Constants and Helpers
// ============================================================================

/** Valid theme values */
export const VALID_THEMES: readonly Theme[] = ['dark', 'light'] as const;

/** Valid audio quality values */
export const VALID_QUALITIES: readonly AudioQuality[] = [
  'low',
  'medium',
  'high',
  'lossless',
] as const;

/** Maximum crossfade duration in milliseconds (12 seconds) */
export const MAX_CROSSFADE_MS = 12_000;

/**
 * Type guard for validating theme values
 */
export function isValidTheme(theme: string): theme is Theme {
  return VALID_THEMES.includes(theme as Theme);
}

/**
 * Type guard for validating audio quality values
 */
export function isValidQuality(quality: string): quality is AudioQuality {
  return VALID_QUALITIES.includes(quality as AudioQuality);
}

/**
 * Validate crossfade duration is within acceptable range
 */
export function isValidCrossfade(durationMs: number): boolean {
  return durationMs >= 0 && durationMs <= MAX_CROSSFADE_MS;
}
