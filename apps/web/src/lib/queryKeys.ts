/**
 * Query key factory for TanStack Query
 *
 * Provides a consistent, hierarchical key structure for cache management.
 * Keys are organized by entity type and operation.
 *
 * @example
 * ```ts
 * // Invalidate all artist queries
 * queryClient.invalidateQueries({ queryKey: libraryKeys.artists.all() })
 *
 * // Invalidate specific artist
 * queryClient.invalidateQueries({ queryKey: libraryKeys.artists.detail('artist-id') })
 * ```
 */

import type { SimilarityMethod } from '../types/similarity'

export const libraryKeys = {
  /** Root key for all library queries */
  all: ['library'] as const,

  // ============ Artists ============
  artists: {
    all: () => [...libraryKeys.all, 'artists'] as const,
    list: (params: { limit?: number; offset?: number } = {}) =>
      [...libraryKeys.artists.all(), 'list', params] as const,
    detail: (id: string) =>
      [...libraryKeys.artists.all(), 'detail', id] as const,
    search: (query: string, limit?: number) =>
      [...libraryKeys.artists.all(), 'search', query, limit] as const,
    byGenre: (genre: string, params: { limit?: number; offset?: number } = {}) =>
      [...libraryKeys.artists.all(), 'genre', genre, params] as const,
  },

  // ============ Albums ============
  albums: {
    all: () => [...libraryKeys.all, 'albums'] as const,
    list: (params: { limit?: number; offset?: number } = {}) =>
      [...libraryKeys.albums.all(), 'list', params] as const,
    detail: (id: string) =>
      [...libraryKeys.albums.all(), 'detail', id] as const,
    recent: (limit?: number) =>
      [...libraryKeys.albums.all(), 'recent', limit] as const,
    search: (query: string, limit?: number) =>
      [...libraryKeys.albums.all(), 'search', query, limit] as const,
    byArtist: (artistId: string, params: { limit?: number; offset?: number } = {}) =>
      [...libraryKeys.albums.all(), 'artist', artistId, params] as const,
  },

  // ============ Tracks ============
  tracks: {
    all: () => [...libraryKeys.all, 'tracks'] as const,
    list: (params: { limit?: number; offset?: number } = {}) =>
      [...libraryKeys.tracks.all(), 'list', params] as const,
    detail: (id: string) =>
      [...libraryKeys.tracks.all(), 'detail', id] as const,
    top: (limit?: number) =>
      [...libraryKeys.tracks.all(), 'top', limit] as const,
    search: (query: string, limit?: number) =>
      [...libraryKeys.tracks.all(), 'search', query, limit] as const,
    seedSearch: (query: string, limit?: number) =>
      [...libraryKeys.tracks.all(), 'seedSearch', query, limit] as const,
    similar: (trackId: string, params: { limit?: number; method?: SimilarityMethod } = {}) =>
      [...libraryKeys.tracks.all(), 'similar', trackId, params] as const,
    byAlbum: (albumId: string, params: { limit?: number; offset?: number } = {}) =>
      [...libraryKeys.tracks.all(), 'album', albumId, params] as const,
    byArtist: (artistId: string, params: { limit?: number; offset?: number } = {}) =>
      [...libraryKeys.tracks.all(), 'artist', artistId, params] as const,
  },

  // ============ Playlists ============
  playlists: {
    all: () => [...libraryKeys.all, 'playlists'] as const,
    detail: (id: string) =>
      [...libraryKeys.playlists.all(), 'detail', id] as const,
    mine: (params: { limit?: number; offset?: number } = {}) =>
      [...libraryKeys.playlists.all(), 'mine', params] as const,
    public: (params: { limit?: number; offset?: number } = {}) =>
      [...libraryKeys.playlists.all(), 'public', params] as const,
  },

  // ============ Search ============
  search: {
    all: () => [...libraryKeys.all, 'search'] as const,
    combined: (query: string, limit?: number) =>
      [...libraryKeys.search.all(), 'combined', query, limit] as const,
  },

  // ============ Discovery (Last.fm) ============
  discovery: {
    all: () => [...libraryKeys.all, 'discovery'] as const,
    similarArtists: (artistName: string, limit?: number) =>
      [...libraryKeys.discovery.all(), 'similar-artists', artistName, limit] as const,
    artistTags: (artistName: string) =>
      [...libraryKeys.discovery.all(), 'artist-tags', artistName] as const,
  },
} as const

/**
 * Query key factory for integration queries
 *
 * Provides cache keys for external service integrations:
 * - ListenBrainz scrobbling
 * - Discord Rich Presence
 */
export const integrationKeys = {
  /** Root key for all integration queries */
  all: ['integrations'] as const,

  /** User's integration settings */
  settings: () => [...integrationKeys.all, 'settings'] as const,
} as const

/** Type for any integration query key */
export type IntegrationQueryKey = ReturnType<
  typeof integrationKeys.settings
>

/**
 * Query key factory for user preferences queries
 *
 * Provides cache keys for user preference management:
 * - User preferences (synced with server)
 */
export const preferencesKeys = {
  /** Root key for all preferences queries */
  all: ['preferences'] as const,

  /** Current user's preferences */
  user: () => [...preferencesKeys.all, 'user'] as const,
} as const

/** Type for any preferences query key */
export type PreferencesQueryKey = ReturnType<
  typeof preferencesKeys.user
>

/**
 * Query key factory for chat queries
 *
 * Provides cache keys for AI chat functionality:
 * - Conversation list
 * - Individual conversation with messages
 */
export const chatKeys = {
  /** Root key for all chat queries */
  all: ['chat'] as const,

  /** All conversations list */
  conversations: () => [...chatKeys.all, 'conversations'] as const,

  /** Single conversation detail */
  conversation: (id: string) => [...chatKeys.all, 'conversation', id] as const,
} as const

/** Type for any chat query key */
export type ChatQueryKey = ReturnType<
  | typeof chatKeys.conversations
  | typeof chatKeys.conversation
>

/**
 * Query key factory for user library path queries
 *
 * Provides cache keys for user-specific library path management:
 * - User's configured library paths
 */
export const libraryPathKeys = {
  /** Root key for all library path queries */
  all: ['libraryPaths'] as const,

  /** Current user's library paths */
  list: () => [...libraryPathKeys.all, 'list'] as const,
} as const

/** Type for any library path query key */
export type LibraryPathQueryKey = ReturnType<
  typeof libraryPathKeys.list
>

/** Type for any library query key */
export type LibraryQueryKey = ReturnType<
  | typeof libraryKeys.artists.all
  | typeof libraryKeys.artists.list
  | typeof libraryKeys.artists.detail
  | typeof libraryKeys.artists.search
  | typeof libraryKeys.artists.byGenre
  | typeof libraryKeys.albums.all
  | typeof libraryKeys.albums.list
  | typeof libraryKeys.albums.detail
  | typeof libraryKeys.albums.recent
  | typeof libraryKeys.albums.search
  | typeof libraryKeys.albums.byArtist
  | typeof libraryKeys.tracks.all
  | typeof libraryKeys.tracks.list
  | typeof libraryKeys.tracks.detail
  | typeof libraryKeys.tracks.top
  | typeof libraryKeys.tracks.search
  | typeof libraryKeys.tracks.seedSearch
  | typeof libraryKeys.tracks.similar
  | typeof libraryKeys.tracks.byAlbum
  | typeof libraryKeys.tracks.byArtist
  | typeof libraryKeys.playlists.all
  | typeof libraryKeys.playlists.detail
  | typeof libraryKeys.playlists.mine
  | typeof libraryKeys.playlists.public
  | typeof libraryKeys.search.all
  | typeof libraryKeys.search.combined
  | typeof libraryKeys.discovery.all
  | typeof libraryKeys.discovery.similarArtists
  | typeof libraryKeys.discovery.artistTags
>
