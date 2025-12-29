/**
 * TanStack Query hooks for library data fetching
 *
 * These hooks provide type-safe data fetching for the music library.
 * They handle loading states, caching, and error handling automatically.
 */

import { useQuery, UseQueryOptions } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import { libraryKeys } from '../lib/queryKeys'
import {
  ARTIST_QUERY,
  ARTISTS_QUERY,
  SEARCH_ARTISTS_QUERY,
  ALBUM_QUERY,
  ALBUMS_QUERY,
  RECENT_ALBUMS_QUERY,
  SEARCH_ALBUMS_QUERY,
  TRACK_QUERY,
  TRACKS_QUERY,
  TOP_TRACKS_QUERY,
  SEARCH_TRACKS_QUERY,
  TRACKS_BY_ALBUM_QUERY,
  PLAYLIST_QUERY,
  MY_PLAYLISTS_QUERY,
  PUBLIC_PLAYLISTS_QUERY,
} from '../lib/graphql/library'
import type {
  GqlArtist,
  GqlAlbum,
  GqlTrack,
  GqlPlaylist,
  ArtistQueryResponse,
  ArtistsQueryResponse,
  SearchArtistsQueryResponse,
  AlbumQueryResponse,
  AlbumsQueryResponse,
  RecentAlbumsQueryResponse,
  SearchAlbumsQueryResponse,
  TrackQueryResponse,
  TracksQueryResponse,
  TopTracksQueryResponse,
  SearchTracksQueryResponse,
  TracksByAlbumQueryResponse,
  PlaylistQueryResponse,
  MyPlaylistsQueryResponse,
  PublicPlaylistsQueryResponse,
} from '../types/library'

// Default stale times
const STALE_TIME_SHORT = 30 * 1000 // 30 seconds for frequently changing data
const STALE_TIME_MEDIUM = 5 * 60 * 1000 // 5 minutes for most data
const STALE_TIME_LONG = 15 * 60 * 1000 // 15 minutes for rarely changing data

// ============================================================================
// Artist Hooks
// ============================================================================

/**
 * Fetch a single artist by ID with albums and top tracks
 */
export function useArtist(
  id: string | undefined,
  options?: Omit<UseQueryOptions<GqlArtist | null, Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.artists.detail(id ?? ''),
    queryFn: async () => {
      const response = await graphqlClient.request<ArtistQueryResponse>(
        ARTIST_QUERY,
        { id }
      )
      return response.artist
    },
    enabled: Boolean(id),
    staleTime: STALE_TIME_MEDIUM,
    ...options,
  })
}

/**
 * Fetch paginated list of artists
 */
export function useArtists(
  params: { limit?: number; offset?: number } = {},
  options?: Omit<UseQueryOptions<GqlArtist[], Error>, 'queryKey' | 'queryFn'>
) {
  const { limit = 50, offset = 0 } = params

  return useQuery({
    queryKey: libraryKeys.artists.list({ limit, offset }),
    queryFn: async () => {
      const response = await graphqlClient.request<ArtistsQueryResponse>(
        ARTISTS_QUERY,
        { limit, offset }
      )
      return response.artists
    },
    staleTime: STALE_TIME_MEDIUM,
    ...options,
  })
}

/**
 * Search artists by name
 */
export function useSearchArtists(
  query: string,
  limit = 20,
  options?: Omit<UseQueryOptions<GqlArtist[], Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.artists.search(query, limit),
    queryFn: async () => {
      const response = await graphqlClient.request<SearchArtistsQueryResponse>(
        SEARCH_ARTISTS_QUERY,
        { query, limit }
      )
      return response.searchArtists
    },
    enabled: query.length >= 2,
    staleTime: STALE_TIME_SHORT,
    ...options,
  })
}

// ============================================================================
// Album Hooks
// ============================================================================

/**
 * Fetch a single album by ID with tracks
 */
export function useAlbum(
  id: string | undefined,
  options?: Omit<UseQueryOptions<GqlAlbum | null, Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.albums.detail(id ?? ''),
    queryFn: async () => {
      const response = await graphqlClient.request<AlbumQueryResponse>(
        ALBUM_QUERY,
        { id }
      )
      return response.album
    },
    enabled: Boolean(id),
    staleTime: STALE_TIME_MEDIUM,
    ...options,
  })
}

/**
 * Fetch paginated list of albums
 */
export function useAlbums(
  params: { limit?: number; offset?: number } = {},
  options?: Omit<UseQueryOptions<GqlAlbum[], Error>, 'queryKey' | 'queryFn'>
) {
  const { limit = 50, offset = 0 } = params

  return useQuery({
    queryKey: libraryKeys.albums.list({ limit, offset }),
    queryFn: async () => {
      const response = await graphqlClient.request<AlbumsQueryResponse>(
        ALBUMS_QUERY,
        { limit, offset }
      )
      return response.albums
    },
    staleTime: STALE_TIME_MEDIUM,
    ...options,
  })
}

/**
 * Fetch recently added albums
 */
export function useRecentAlbums(
  limit = 20,
  options?: Omit<UseQueryOptions<GqlAlbum[], Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.albums.recent(limit),
    queryFn: async () => {
      const response = await graphqlClient.request<RecentAlbumsQueryResponse>(
        RECENT_ALBUMS_QUERY,
        { limit }
      )
      return response.recentAlbums
    },
    staleTime: STALE_TIME_SHORT, // Recent albums change more frequently
    ...options,
  })
}

/**
 * Search albums by title
 */
export function useSearchAlbums(
  query: string,
  limit = 20,
  options?: Omit<UseQueryOptions<GqlAlbum[], Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.albums.search(query, limit),
    queryFn: async () => {
      const response = await graphqlClient.request<SearchAlbumsQueryResponse>(
        SEARCH_ALBUMS_QUERY,
        { query, limit }
      )
      return response.searchAlbums
    },
    enabled: query.length >= 2,
    staleTime: STALE_TIME_SHORT,
    ...options,
  })
}

// ============================================================================
// Track Hooks
// ============================================================================

/**
 * Fetch a single track by ID
 */
export function useTrack(
  id: string | undefined,
  options?: Omit<UseQueryOptions<GqlTrack | null, Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.tracks.detail(id ?? ''),
    queryFn: async () => {
      const response = await graphqlClient.request<TrackQueryResponse>(
        TRACK_QUERY,
        { id }
      )
      return response.track
    },
    enabled: Boolean(id),
    staleTime: STALE_TIME_MEDIUM,
    ...options,
  })
}

/**
 * Fetch paginated list of tracks
 */
export function useTracks(
  params: { limit?: number; offset?: number } = {},
  options?: Omit<UseQueryOptions<GqlTrack[], Error>, 'queryKey' | 'queryFn'>
) {
  const { limit = 50, offset = 0 } = params

  return useQuery({
    queryKey: libraryKeys.tracks.list({ limit, offset }),
    queryFn: async () => {
      const response = await graphqlClient.request<TracksQueryResponse>(
        TRACKS_QUERY,
        { limit, offset }
      )
      return response.tracks
    },
    staleTime: STALE_TIME_MEDIUM,
    ...options,
  })
}

/**
 * Fetch top played tracks globally
 */
export function useTopTracks(
  limit = 50,
  options?: Omit<UseQueryOptions<GqlTrack[], Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.tracks.top(limit),
    queryFn: async () => {
      const response = await graphqlClient.request<TopTracksQueryResponse>(
        TOP_TRACKS_QUERY,
        { limit }
      )
      return response.topTracks
    },
    staleTime: STALE_TIME_SHORT, // Play counts change frequently
    ...options,
  })
}

/**
 * Search tracks by title
 */
export function useSearchTracks(
  query: string,
  limit = 20,
  options?: Omit<UseQueryOptions<GqlTrack[], Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.tracks.search(query, limit),
    queryFn: async () => {
      const response = await graphqlClient.request<SearchTracksQueryResponse>(
        SEARCH_TRACKS_QUERY,
        { query, limit }
      )
      return response.searchTracks
    },
    enabled: query.length >= 2,
    staleTime: STALE_TIME_SHORT,
    ...options,
  })
}

/**
 * Fetch tracks by album ID
 */
export function useTracksByAlbum(
  albumId: string | undefined,
  params: { limit?: number; offset?: number } = {},
  options?: Omit<UseQueryOptions<GqlTrack[], Error>, 'queryKey' | 'queryFn'>
) {
  const { limit = 100, offset = 0 } = params
  const isEnabled = Boolean(albumId)

  return useQuery({
    queryKey: libraryKeys.tracks.byAlbum(albumId ?? '__disabled__', { limit, offset }),
    queryFn: async () => {
      const response = await graphqlClient.request<TracksByAlbumQueryResponse>(
        TRACKS_BY_ALBUM_QUERY,
        { albumId, limit, offset }
      )
      return response.tracksByAlbum
    },
    enabled: isEnabled,
    staleTime: STALE_TIME_LONG, // Album tracks rarely change
    ...options,
  })
}

// ============================================================================
// Playlist Hooks
// ============================================================================

/**
 * Fetch a single playlist by ID with tracks
 */
export function usePlaylist(
  id: string | undefined,
  options?: Omit<UseQueryOptions<GqlPlaylist | null, Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.playlists.detail(id ?? ''),
    queryFn: async () => {
      const response = await graphqlClient.request<PlaylistQueryResponse>(
        PLAYLIST_QUERY,
        { id }
      )
      return response.playlist
    },
    enabled: Boolean(id),
    staleTime: STALE_TIME_SHORT, // Playlists can change frequently
    ...options,
  })
}

/**
 * Fetch playlists owned by the authenticated user
 */
export function useMyPlaylists(
  params: { limit?: number; offset?: number } = {},
  options?: Omit<UseQueryOptions<GqlPlaylist[], Error>, 'queryKey' | 'queryFn'>
) {
  const { limit = 50, offset = 0 } = params

  return useQuery({
    queryKey: libraryKeys.playlists.mine({ limit, offset }),
    queryFn: async () => {
      const response = await graphqlClient.request<MyPlaylistsQueryResponse>(
        MY_PLAYLISTS_QUERY,
        { limit, offset }
      )
      return response.myPlaylists
    },
    staleTime: STALE_TIME_SHORT,
    ...options,
  })
}

/**
 * Fetch public playlists
 */
export function usePublicPlaylists(
  params: { limit?: number; offset?: number } = {},
  options?: Omit<UseQueryOptions<GqlPlaylist[], Error>, 'queryKey' | 'queryFn'>
) {
  const { limit = 50, offset = 0 } = params

  return useQuery({
    queryKey: libraryKeys.playlists.public({ limit, offset }),
    queryFn: async () => {
      const response = await graphqlClient.request<PublicPlaylistsQueryResponse>(
        PUBLIC_PLAYLISTS_QUERY,
        { limit, offset }
      )
      return response.publicPlaylists
    },
    staleTime: STALE_TIME_MEDIUM,
    ...options,
  })
}
