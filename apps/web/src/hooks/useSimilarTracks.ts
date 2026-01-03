/**
 * Similar tracks hooks for finding musically similar tracks
 *
 * These hooks provide track similarity features using different algorithms:
 * - Combined: Weighted blend of all similarity methods
 * - Semantic: AI embeddings similarity
 * - Acoustic: Audio features (BPM, energy, loudness, valence, danceability)
 * - Categorical: Genre and mood tag matching
 *
 * @example
 * ```tsx
 * // Find similar tracks using combined algorithm (default)
 * const { data: similar } = useSimilarTracks('track-uuid', 10)
 *
 * // Find similar tracks using a specific method
 * const { data: acoustic } = useSimilarTracksByMethod('track-uuid', 'Acoustic', 10)
 * ```
 */

import { useQuery, UseQueryOptions } from '@tanstack/react-query'
import { graphqlClient } from '../lib/api'
import { libraryKeys } from '../lib/queryKeys'
import {
  SIMILAR_TRACKS_QUERY,
  SIMILAR_TRACKS_BY_METHOD_QUERY,
} from '../lib/graphql/similarity'
import type {
  ScoredTrack,
  SimilarTrack,
  SimilarTracksResponse,
  SimilarTracksByMethodResponse,
  SimilarityMethod,
} from '../types/similarity'

// Re-export types for consumers
export type { ScoredTrack, SimilarTrack, SimilarityMethod } from '../types/similarity'

/**
 * Find tracks similar to a given track using combined similarity
 *
 * Uses a weighted blend of semantic (50%), acoustic (30%), and categorical (20%)
 * similarity to find the most similar tracks in your library.
 *
 * @param trackId - UUID of the track to find similar tracks for
 * @param limit - Maximum number of results (default: 10, max: 50)
 * @param options - Additional TanStack Query options
 *
 * @example
 * ```tsx
 * function SimilarTracksSection({ trackId }: { trackId: string }) {
 *   const { data, isLoading, error } = useSimilarTracks(trackId, 10)
 *
 *   if (isLoading) return <Spinner />
 *   if (error) return <Error message="Could not load similar tracks" />
 *
 *   return (
 *     <div>
 *       {data?.map((track) => (
 *         <TrackCard
 *           key={track.trackId}
 *           title={track.title}
 *           artist={track.artistName}
 *           similarity={Math.round(track.score * 100)}
 *         />
 *       ))}
 *     </div>
 *   )
 * }
 * ```
 */
export function useSimilarTracks(
  trackId: string,
  limit = 10,
  options?: Omit<UseQueryOptions<ScoredTrack[], Error>, 'queryKey' | 'queryFn'>
) {
  // Clamp limit to valid range (1-50)
  const clampedLimit = Math.min(Math.max(limit, 1), 50)

  return useQuery({
    queryKey: libraryKeys.tracks.similar(trackId, { limit: clampedLimit }),
    queryFn: async (): Promise<ScoredTrack[]> => {
      const response = await graphqlClient.request<SimilarTracksResponse>(
        SIMILAR_TRACKS_QUERY,
        { trackId, limit: clampedLimit }
      )
      return response.similarTracks ?? []
    },
    enabled: !!trackId.trim(),
    staleTime: 5 * 60 * 1000, // 5 minutes - similarity data is relatively stable
    gcTime: 30 * 60 * 1000, // 30 minutes cache
    ...options,
  })
}

/**
 * Find tracks similar to a given track using a specific similarity method
 *
 * Available methods:
 * - Combined: Weighted blend (50% semantic, 30% acoustic, 20% categorical)
 * - Semantic: AI embeddings similarity (requires tracks to have embeddings)
 * - Acoustic: Audio features (BPM, energy, loudness, valence, danceability)
 * - Categorical: Genre and mood tag matching
 *
 * @param trackId - UUID of the track to find similar tracks for
 * @param method - Similarity method to use
 * @param limit - Maximum number of results (default: 10, max: 50)
 * @param options - Additional TanStack Query options
 *
 * @example
 * ```tsx
 * function AcousticSimilarTracks({ trackId }: { trackId: string }) {
 *   const { data, isLoading } = useSimilarTracksByMethod(trackId, 'Acoustic', 10)
 *
 *   if (isLoading) return <Spinner />
 *
 *   return (
 *     <div>
 *       <h3>Sonically Similar Tracks</h3>
 *       {data?.map((track) => (
 *         <TrackCard
 *           key={track.trackId}
 *           title={track.title}
 *           artist={track.artistName}
 *           similarityType={track.similarityType}
 *           similarity={Math.round(track.score * 100)}
 *         />
 *       ))}
 *     </div>
 *   )
 * }
 * ```
 */
export function useSimilarTracksByMethod(
  trackId: string,
  method: SimilarityMethod,
  limit = 10,
  options?: Omit<UseQueryOptions<SimilarTrack[], Error>, 'queryKey' | 'queryFn'>
) {
  // Clamp limit to valid range (1-50)
  const clampedLimit = Math.min(Math.max(limit, 1), 50)

  return useQuery({
    queryKey: libraryKeys.tracks.similar(trackId, { limit: clampedLimit, method }),
    queryFn: async (): Promise<SimilarTrack[]> => {
      const response = await graphqlClient.request<SimilarTracksByMethodResponse>(
        SIMILAR_TRACKS_BY_METHOD_QUERY,
        { trackId, method, limit: clampedLimit }
      )
      return response.similarTracksByMethod ?? []
    },
    enabled: !!trackId.trim(),
    staleTime: 5 * 60 * 1000, // 5 minutes - similarity data is relatively stable
    gcTime: 30 * 60 * 1000, // 30 minutes cache
    ...options,
  })
}
