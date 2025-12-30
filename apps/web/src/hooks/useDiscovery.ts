/**
 * Discovery hooks for finding similar artists and tags via Last.fm
 *
 * These hooks provide artist discovery features:
 * - Find similar artists (with local library status)
 * - Get artist genre/style tags
 *
 * @example
 * ```tsx
 * // Find artists similar to Radiohead
 * const { data: similar } = useSimilarArtists('Radiohead', 10)
 *
 * // Get genre tags for an artist
 * const { data: tags } = useArtistTags('Radiohead')
 * ```
 */

import { useQuery, UseQueryOptions } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import { libraryKeys } from '../lib/queryKeys'
import { SIMILAR_ARTISTS_QUERY, ARTIST_TAGS_QUERY } from '../lib/graphql/discovery'
import type {
  SimilarArtist,
  ArtistTag,
  SimilarArtistsResponse,
  ArtistTagsResponse,
} from '../types/discovery'

// Re-export types for consumers
export type { SimilarArtist, ArtistTag } from '../types/discovery'

/**
 * Find similar artists using Last.fm
 *
 * Returns artists musically similar to the given artist, enriched with
 * information about whether they're already in your local library.
 *
 * @param artistName - Name of the artist to find similar artists for
 * @param limit - Maximum number of results (default: 10, max: 50)
 * @param options - Additional TanStack Query options
 *
 * @example
 * ```tsx
 * function SimilarArtistsSection({ artistName }: { artistName: string }) {
 *   const { data, isLoading, error } = useSimilarArtists(artistName, 10)
 *
 *   if (isLoading) return <Spinner />
 *   if (error) return <Error message="Could not load similar artists" />
 *
 *   return (
 *     <div>
 *       {data?.map((artist) => (
 *         <ArtistCard
 *           key={artist.name}
 *           name={artist.name}
 *           inLibrary={artist.inLibrary}
 *           similarity={Math.round(artist.matchScore * 100)}
 *         />
 *       ))}
 *     </div>
 *   )
 * }
 * ```
 */
export function useSimilarArtists(
  artistName: string,
  limit = 10,
  options?: Omit<UseQueryOptions<SimilarArtist[], Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.discovery.similarArtists(artistName, limit),
    queryFn: async (): Promise<SimilarArtist[]> => {
      const response = await graphqlClient.request<SimilarArtistsResponse>(
        SIMILAR_ARTISTS_QUERY,
        { artistName, limit }
      )
      return response.similarArtists
    },
    enabled: !!artistName.trim(),
    staleTime: 5 * 60 * 1000, // 5 minutes - Last.fm data doesn't change often
    gcTime: 30 * 60 * 1000, // 30 minutes cache
    ...options,
  })
}

/**
 * Get genre/style tags for an artist from Last.fm
 *
 * Returns tags like "rock", "alternative", "electronic" that describe
 * the artist's musical style, sorted by popularity.
 *
 * @param artistName - Name of the artist to get tags for
 * @param options - Additional TanStack Query options
 *
 * @example
 * ```tsx
 * function GenreTags({ artistName }: { artistName: string }) {
 *   const { data: tags } = useArtistTags(artistName)
 *
 *   return (
 *     <div className="flex gap-2">
 *       {tags?.slice(0, 5).map((tag) => (
 *         <Badge key={tag.name}>{tag.name}</Badge>
 *       ))}
 *     </div>
 *   )
 * }
 * ```
 */
export function useArtistTags(
  artistName: string,
  options?: Omit<UseQueryOptions<ArtistTag[], Error>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: libraryKeys.discovery.artistTags(artistName),
    queryFn: async (): Promise<ArtistTag[]> => {
      const response = await graphqlClient.request<ArtistTagsResponse>(
        ARTIST_TAGS_QUERY,
        { artistName }
      )
      return response.artistTags
    },
    enabled: !!artistName.trim(),
    staleTime: 10 * 60 * 1000, // 10 minutes - tags are very stable
    gcTime: 60 * 60 * 1000, // 1 hour cache
    ...options,
  })
}
