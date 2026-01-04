/**
 * @resonance/shared-types
 *
 * Shared TypeScript type definitions for the Resonance music streaming platform.
 * This package provides type definitions used across the frontend and for API contracts.
 */

// API types - responses, errors, pagination
export type {
  ApiError,
  ValidationError,
  AuthError,
  RateLimitError,
  PaginatedResponse,
  ApiResponse,
  HealthCheckResponse,
  ServiceHealth,
  PaginationParams,
  SortParams,
  SearchParams,
  StreamRequest,
  StreamMetadata,
} from './api.js';

// Library types - tracks, albums, artists, playlists
export type {
  Artist,
  Album,
  AlbumType,
  CoverColors,
  Track,
  AudioFormat,
  SyncedLyricLine,
  Playlist,
  PlaylistTrack,
  SmartPlaylist,
  SmartPlaylistRule,
  SmartPlaylistField,
  SmartPlaylistOperator,
  SmartPlaylistSort,
  Recommendation,
  RecommendationSource,
  MoodAnalysis,
  ChatMessage,
  ChatAction,
  SearchResults,
  SearchFilters,
} from './library.js';

// Player types - playback state, queue, devices, audio settings
export type {
  PlaybackState,
  RepeatMode,
  PlaybackQuality,
  QueueItem,
  QueueContext,
  QueueAction,
  Device,
  DeviceType,
  DeviceCapabilities,
  EqualizerSettings,
  EqualizerBands,
  EqualizerPreset,
  CrossfadeSettings,
  NormalizationSettings,
  PlaybackHistoryEntry,
  ListeningStats,
} from './player.js';

// User types - accounts, sessions, preferences
export type {
  User,
  UserRole,
  UserProfile,
  Session,
  AuthTokens,
  LoginRequest,
  RegisterRequest,
  UserPreferences,
  AudioPreferences,
  UIPreferences,
  PrivacyPreferences,
  NotificationPreferences,
  IntegrationPreferences,
  ActivityItem,
  ActivityType,
  Follow,
} from './user.js';

// WebSocket types - real-time sync and presence
export type {
  WebSocketMessage,
  WebSocketError,
  SyncMessage,
  PlaybackSyncMessage,
  QueueSyncMessage,
  VolumeSyncMessage,
  SeekSyncMessage,
  DeviceSyncMessage,
  TransferSyncMessage,
  PresenceMessage,
  UserPresenceMessage,
  ListeningActivityMessage,
  TypingIndicatorMessage,
  NotificationMessage,
  NewFollowerNotification,
  PlaylistUpdateNotification,
  NewReleaseNotification,
  SystemNotification,
  ClientCommand,
  SubscribeCommand,
  UnsubscribeCommand,
  PlaybackCommand,
  QueueCommand,
  PresenceCommand,
  ChannelType,
  ConnectionState,
  ConnectionEstablished,
  HeartbeatMessage,
  HeartbeatResponse,
} from './websocket.js';

// GraphQL types - query/mutation placeholders
export type {
  QueryTypes,
  TracksQueryArgs,
  AlbumsQueryArgs,
  ArtistsQueryArgs,
  PlaylistsQueryArgs,
  SearchQueryArgs,
  RecommendationsQueryArgs,
  RadioQueryArgs,
  StatsPeriod,
  MutationTypes,
  LoginMutationArgs,
  RegisterMutationArgs,
  AuthPayload,
  UpdateProfileArgs,
  CreatePlaylistArgs,
  UpdatePlaylistArgs,
  AddTracksToPlaylistArgs,
  RemoveTracksFromPlaylistArgs,
  ReorderPlaylistTracksArgs,
  PlayMutationArgs,
  SubscriptionTypes,
  NowPlayingUpdate,
  PlaylistUpdate,
  PaginationInput,
  SortInput,
  DateRangeInput,
  OperationResult,
  GraphQLError,
} from './graphql.js';

// Admin types - dashboard and user management
export type {
  SystemStats,
  AdminUserListItem,
  AdminUserList,
  AdminSession,
  AdminUserDetail,
  AdminOperationResult,
  InvalidateSessionsResult,
  UpdateUserRoleInput,
} from './admin.js';

// Preferences types - flat GraphQL preferences structure
export type {
  Theme,
  AudioQuality,
  GraphQLUserPreferences,
  UpdatePreferencesInput,
  UserWithPreferences,
  UpdatePreferencesResponse,
  ResetPreferencesResponse,
  GetPreferencesResponse,
  UserPreferencesQueryResponse,
} from './preferences.js';

export {
  DEFAULT_GRAPHQL_PREFERENCES,
  VALID_THEMES,
  VALID_QUALITIES,
  MAX_CROSSFADE_MS,
  isValidTheme,
  isValidQuality,
  isValidCrossfade,
} from './preferences.js';
