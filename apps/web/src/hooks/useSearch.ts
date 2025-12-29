/**
 * Combined search hook for searching across all content types
 *
 * Performs a single GraphQL query that searches artists, albums, and tracks
 * simultaneously for efficient search results.
 */

import { useQuery, UseQueryOptions } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import { libraryKeys } from '../lib/queryKeys'
import { COMBINED_SEARCH_QUERY } from '../lib/graphql/library'
import type {
  GqlArtist,
  GqlAlbum,
  GqlTrack,
  CombinedSearchQueryResponse,
} from '../types/library'

/**
 * Combined search results
 */
export interface SearchResults {
  artists: GqlArtist[]
  albums: GqlAlbum[]
  tracks: GqlTrack[]
  /** Total number of results across all types */
  totalCount: number
  /** Whether there are any results */
  hasResults: boolean
}

/**
 * Search across artists, albums, and tracks simultaneously
 *
 * @param query - Search query string (minimum 2 characters to enable search)
 * @param limit - Maximum results per type (default: 10)
 * @param options - Additional TanStack Query options
 *
 * @example
 * ```tsx
 * const { data, isLoading } = useSearch(debouncedQuery)
 *
 * if (data?.hasResults) {
 *   // Show results grouped by type
 * }
 * ```
 */
export function useSearch(
  query: string,
  limit = 10,
  options?: Omit<UseQueryOptions<SearchResults, Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.search.combined(query, limit),
    queryFn: async (): Promise<SearchResults> => {
      const response = await graphqlClient.request<CombinedSearchQueryResponse>(
        COMBINED_SEARCH_QUERY,
        { query, limit }
      )

      const artists = response.searchArtists
      const albums = response.searchAlbums
      const tracks = response.searchTracks
      const totalCount = artists.length + albums.length + tracks.length

      return {
        artists,
        albums,
        tracks,
        totalCount,
        hasResults: totalCount > 0,
      }
    },
    enabled: query.length >= 2,
    staleTime: 30 * 1000, // 30 seconds for search results
    gcTime: 5 * 60 * 1000, // 5 minutes cache
    ...options,
  })
}

/**
 * Search with separate queries for each type
 *
 * Use this when you need to paginate results independently
 * or want more control over individual type queries.
 */
export { useSearchArtists, useSearchAlbums, useSearchTracks } from './useLibrary'
