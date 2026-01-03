/**
 * Utility to fetch track metadata from GraphQL API
 *
 * Used by cross-device sync when a remote device starts playing a track
 * that the local device doesn't have in its current state.
 */

import { graphqlClient } from '../lib/api'
import { TRACK_QUERY } from '../lib/graphql/library'
import { mapGqlTrackToPlayerTrack } from '../lib/mappers'
import type { Track } from '../stores/playerStore'
import type { TrackQueryResponse, IdQueryVariables } from '../types/library'

/**
 * Fetch a track by ID from the GraphQL API
 *
 * @param trackId - The track ID to fetch
 * @returns The track in playerStore format, or null if not found
 *
 * @example
 * ```ts
 * // When a remote device starts playing an unknown track
 * const track = await fetchTrackById(syncState.track_id)
 * if (track) {
 *   playerStore.setTrack(track)
 * }
 * ```
 */
export async function fetchTrackById(trackId: string): Promise<Track | null> {
  if (!trackId) {
    return null
  }

  try {
    const response = await graphqlClient.request<TrackQueryResponse, IdQueryVariables>(
      TRACK_QUERY,
      { id: trackId }
    )

    if (!response.track) {
      return null
    }

    return mapGqlTrackToPlayerTrack(response.track)
  } catch (error) {
    console.error('[fetchTrackById] Failed to fetch track:', error)
    return null
  }
}
