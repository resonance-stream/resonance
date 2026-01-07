/**
 * GraphQL query and mutation type placeholders for Resonance
 *
 * This file contains placeholder types that will be replaced/augmented
 * by auto-generated types from GraphQL codegen once the schema is defined.
 *
 * Run `pnpm codegen` from the web app to generate types from the schema.
 */

import type { Track, Album, Artist, Playlist, SearchResults, SearchFilters, Recommendation } from './library.js';
import type { User, UserProfile, UserPreferences } from './user.js';
import type { PlaybackState, Device, QueueItem, ListeningStats } from './player.js';
import type { PaginatedResponse } from './api.js';

// ============================================================================
// Query Types
// ============================================================================

/**
 * Root query type placeholder
 * These will be generated from the GraphQL schema
 */
export interface QueryTypes {
  // User queries
  me: User;
  user: (args: { id: string }) => UserProfile | null;
  userPreferences: UserPreferences;

  // Library queries
  track: (args: { id: string }) => Track | null;
  tracks: (args: TracksQueryArgs) => PaginatedResponse<Track>;
  album: (args: { id: string }) => Album | null;
  albums: (args: AlbumsQueryArgs) => PaginatedResponse<Album>;
  artist: (args: { id: string }) => Artist | null;
  artists: (args: ArtistsQueryArgs) => PaginatedResponse<Artist>;
  playlist: (args: { id: string }) => Playlist | null;
  playlists: (args: PlaylistsQueryArgs) => PaginatedResponse<Playlist>;

  // Search
  search: (args: SearchQueryArgs) => SearchResults;

  // Playback
  playbackState: PlaybackState;
  queue: QueueItem[];
  devices: Device[];

  // Recommendations
  recommendations: (args: RecommendationsQueryArgs) => Recommendation[];
  radio: (args: RadioQueryArgs) => Track[];

  // Stats
  listeningStats: (args: { period?: StatsPeriod }) => ListeningStats;
  recentlyPlayed: (args: { limit?: number }) => Track[];
}

/**
 * Arguments for tracks query
 */
export interface TracksQueryArgs {
  albumId?: string;
  artistId?: string;
  playlistId?: string;
  favoriteOnly?: boolean;
  offset?: number;
  limit?: number;
  sortBy?: 'title' | 'artist' | 'album' | 'added' | 'plays';
  sortOrder?: 'asc' | 'desc';
}

/**
 * Arguments for albums query
 */
export interface AlbumsQueryArgs {
  artistId?: string;
  year?: number;
  genre?: string;
  offset?: number;
  limit?: number;
  sortBy?: 'title' | 'artist' | 'year' | 'added';
  sortOrder?: 'asc' | 'desc';
}

/**
 * Arguments for artists query
 */
export interface ArtistsQueryArgs {
  genre?: string;
  offset?: number;
  limit?: number;
  sortBy?: 'name' | 'added' | 'plays';
  sortOrder?: 'asc' | 'desc';
}

/**
 * Arguments for playlists query
 */
export interface PlaylistsQueryArgs {
  ownerId?: string;
  publicOnly?: boolean;
  offset?: number;
  limit?: number;
  sortBy?: 'name' | 'created' | 'updated';
  sortOrder?: 'asc' | 'desc';
}

/**
 * Arguments for search query
 */
export interface SearchQueryArgs {
  query: string;
  filters?: SearchFilters;
  offset?: number;
  limit?: number;
}

/**
 * Arguments for recommendations query
 */
export interface RecommendationsQueryArgs {
  seedTrackIds?: string[];
  seedArtistIds?: string[];
  seedGenres?: string[];
  mood?: string;
  limit?: number;
}

/**
 * Arguments for radio query
 */
export interface RadioQueryArgs {
  seedType: 'track' | 'artist' | 'album' | 'playlist';
  seedId: string;
  limit?: number;
}

/**
 * Time period for statistics
 */
export type StatsPeriod = 'week' | 'month' | 'year' | 'all_time';

// ============================================================================
// Mutation Types
// ============================================================================

/**
 * Root mutation type placeholder
 */
export interface MutationTypes {
  // Auth mutations
  login: (args: LoginMutationArgs) => AuthPayload;
  register: (args: RegisterMutationArgs) => AuthPayload;
  logout: () => boolean;
  refreshToken: () => AuthPayload;

  // User mutations
  updateProfile: (args: UpdateProfileArgs) => User;
  updatePreferences: (args: { preferences: Partial<UserPreferences> }) => UserPreferences;

  // Library mutations
  favoriteTrack: (args: { trackId: string }) => Track;
  unfavoriteTrack: (args: { trackId: string }) => Track;
  rateTrack: (args: { trackId: string; rating: number }) => Track;

  // Playlist mutations
  createPlaylist: (args: CreatePlaylistArgs) => Playlist;
  updatePlaylist: (args: UpdatePlaylistArgs) => Playlist;
  deletePlaylist: (args: { id: string }) => boolean;
  addTracksToPlaylist: (args: AddTracksToPlaylistArgs) => Playlist;
  removeTracksFromPlaylist: (args: RemoveTracksFromPlaylistArgs) => Playlist;
  reorderPlaylistTracks: (args: ReorderPlaylistTracksArgs) => Playlist;

  // Playback mutations
  play: (args: PlayMutationArgs) => PlaybackState;
  pause: () => PlaybackState;
  seek: (args: { position: number }) => PlaybackState;
  setVolume: (args: { volume: number }) => PlaybackState;
  setShuffle: (args: { enabled: boolean }) => PlaybackState;
  setRepeat: (args: { mode: 'off' | 'track' | 'queue' }) => PlaybackState;
  skipToNext: () => PlaybackState;
  skipToPrevious: () => PlaybackState;
  transferPlayback: (args: { deviceId: string; play?: boolean }) => PlaybackState;

  // Queue mutations
  addToQueue: (args: { trackIds: string[]; position?: 'next' | 'last' }) => QueueItem[];
  removeFromQueue: (args: { queueItemIds: string[] }) => QueueItem[];
  clearQueue: () => boolean;
  reorderQueue: (args: { queueItemId: string; toIndex: number }) => QueueItem[];

  // Social mutations
  followUser: (args: { userId: string }) => UserProfile;
  unfollowUser: (args: { userId: string }) => UserProfile;
  followPlaylist: (args: { playlistId: string }) => Playlist;
  unfollowPlaylist: (args: { playlistId: string }) => Playlist;
}

/**
 * Login mutation arguments
 */
export interface LoginMutationArgs {
  usernameOrEmail: string;
  password: string;
  rememberMe?: boolean;
}

/**
 * Register mutation arguments
 */
export interface RegisterMutationArgs {
  username: string;
  email: string;
  password: string;
  displayName?: string;
}

/**
 * Auth payload returned from auth mutations
 */
export interface AuthPayload {
  user: User;
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
}

/**
 * Update profile arguments
 */
export interface UpdateProfileArgs {
  displayName?: string;
  bio?: string;
  avatarUrl?: string;
}

/**
 * Create playlist arguments
 */
export interface CreatePlaylistArgs {
  name: string;
  description?: string;
  isPublic?: boolean;
  isCollaborative?: boolean;
  trackIds?: string[];
}

/**
 * Update playlist arguments
 */
export interface UpdatePlaylistArgs {
  id: string;
  name?: string;
  description?: string;
  isPublic?: boolean;
  isCollaborative?: boolean;
  coverUrl?: string;
}

/**
 * Add tracks to playlist arguments
 */
export interface AddTracksToPlaylistArgs {
  playlistId: string;
  trackIds: string[];
  position?: number;
}

/**
 * Remove tracks from playlist arguments
 */
export interface RemoveTracksFromPlaylistArgs {
  playlistId: string;
  positions: number[];
}

/**
 * Reorder playlist tracks arguments
 */
export interface ReorderPlaylistTracksArgs {
  playlistId: string;
  fromPosition: number;
  toPosition: number;
}

/**
 * Play mutation arguments
 */
export interface PlayMutationArgs {
  trackId?: string;
  contextType?: 'album' | 'artist' | 'playlist' | 'queue';
  contextId?: string;
  position?: number;
  shuffle?: boolean;
}

// ============================================================================
// Subscription Types
// ============================================================================

/**
 * Root subscription type placeholder
 */
export interface SubscriptionTypes {
  /** Playback state changes */
  playbackStateChanged: PlaybackState;
  /** Queue updates */
  queueChanged: QueueItem[];
  /** Device status changes */
  deviceChanged: Device;
  /** Now playing updates (for activity feed) */
  nowPlayingChanged: NowPlayingUpdate;
  /** Playlist updates (for collaborative playlists) */
  playlistUpdated: PlaylistUpdate;
}

/**
 * Now playing update for subscriptions
 */
export interface NowPlayingUpdate {
  userId: string;
  track: Track | null;
  isPlaying: boolean;
}

/**
 * Playlist update for subscriptions
 */
export interface PlaylistUpdate {
  playlistId: string;
  updateType: 'tracks_added' | 'tracks_removed' | 'tracks_reordered' | 'metadata_changed';
  updatedBy: string;
}

// ============================================================================
// Input Types
// ============================================================================

/**
 * Pagination input for queries
 */
export interface PaginationInput {
  offset?: number;
  limit?: number;
}

/**
 * Sort input for queries
 */
export interface SortInput {
  field: string;
  order: 'asc' | 'desc';
}

/**
 * Date range input for queries
 */
export interface DateRangeInput {
  start?: string;
  end?: string;
}

// ============================================================================
// Utility Types
// ============================================================================

/**
 * GraphQL operation result type
 */
export type OperationResult<T> =
  | { data: T; errors?: never }
  | { data?: never; errors: GraphQLError[] };

/**
 * GraphQL error type
 */
export interface GraphQLError {
  message: string;
  locations?: Array<{ line: number; column: number }>;
  path?: Array<string | number>;
  extensions?: Record<string, unknown>;
}

// ============================================================================
// System Settings Types (for Setup Wizard and Admin Configuration)
// ============================================================================

/**
 * External service type enum
 */
export type ServiceType =
  | 'OLLAMA'
  | 'LIDARR'
  | 'LASTFM'
  | 'MEILISEARCH'
  | 'MUSIC_LIBRARY';

/**
 * First-run setup status for the setup wizard
 */
export interface SetupStatus {
  /** Whether the first-run setup has been completed */
  isComplete: boolean;
  /** Whether at least one admin user exists */
  hasAdmin: boolean;
  /** List of services that have been configured */
  configuredServices: ServiceType[];
}

/**
 * System setting information (safe for admin viewing)
 *
 * Note: This type never exposes actual secrets - only indicates whether
 * secrets have been configured via `hasSecret`.
 */
export interface SystemSettingInfo {
  /** The service type this setting configures */
  service: ServiceType;
  /** Whether this service is enabled */
  enabled: boolean;
  /** Non-sensitive configuration (URLs, ports, options) as JSON */
  config: Record<string, unknown>;
  /** Whether encrypted secrets are configured (never exposes actual secrets) */
  hasSecret: boolean;
  /** Last time a connection test was performed */
  lastConnectionTest: string | null;
  /** Result of the last connection test */
  connectionHealthy: boolean | null;
  /** Error message from the last connection test (if failed) */
  connectionError: string | null;
}

/**
 * Input for creating the initial admin user during setup
 */
export interface CreateAdminInput {
  /** Admin username (for display) */
  username: string;
  /** Admin email address */
  email: string;
  /** Admin password (minimum 8 characters) */
  password: string;
}

/**
 * Input for updating a system setting
 */
export interface UpdateSystemSettingInput {
  /** The service to update */
  service: ServiceType;
  /** Whether to enable or disable the service */
  enabled?: boolean;
  /** Non-sensitive configuration as JSON string */
  config?: string;
  /** Secret value (API key, password, etc.) - will be encrypted before storage */
  secret?: string;
}

/**
 * Result of testing a service connection
 */
export interface ConnectionTestResult {
  /** Whether the connection test was successful */
  success: boolean;
  /** Response time in milliseconds (if successful) */
  responseTimeMs: number | null;
  /** Version of the service (if available) */
  version: string | null;
  /** Error message (if failed) */
  error: string | null;
}

/**
 * User library path configuration
 */
export interface UserLibraryPath {
  /** Unique identifier for this path */
  id: string;
  /** The file system path */
  path: string;
  /** User-friendly label (e.g., "NAS Music", "Local Collection") */
  label: string | null;
  /** Whether this is the user's primary library path */
  isPrimary: boolean;
  /** When this path was added (ISO 8601 timestamp) */
  createdAt: string;
}

// ============================================================================
// System Settings Query Types
// ============================================================================

/**
 * Extended query types including system settings queries
 */
export interface SystemSettingsQueryTypes {
  /** Get setup status (unauthenticated - for setup wizard) */
  setupStatus: SetupStatus;
  /** Get all system settings (admin-only) */
  systemSettings: SystemSettingInfo[];
  /** Get a specific system setting (admin-only) */
  systemSetting: (args: { service: ServiceType }) => SystemSettingInfo | null;
}

// ============================================================================
// System Settings Mutation Types
// ============================================================================

/**
 * Extended mutation types including system settings mutations
 */
export interface SystemSettingsMutationTypes {
  /** Create the initial admin user during setup (no auth required if no users exist) */
  createInitialAdmin: (args: { input: CreateAdminInput }) => AuthPayload;
  /** Mark first-run setup as complete (admin-only) */
  completeSetup: () => boolean;
  /** Update a system setting (admin-only) */
  updateSystemSetting: (args: { input: UpdateSystemSettingInput }) => SystemSettingInfo;
  /** Test connection to an external service (admin-only) */
  testServiceConnection: (args: { service: ServiceType }) => ConnectionTestResult;
  /** Add a user library path (authenticated) */
  addUserLibraryPath: (args: { path: string; label?: string }) => UserLibraryPath;
  /** Remove a user library path (authenticated) */
  removeUserLibraryPath: (args: { id: string }) => boolean;
  /** Set a library path as the user's primary (authenticated) */
  setUserPrimaryLibrary: (args: { id: string }) => UserLibraryPath;
}
