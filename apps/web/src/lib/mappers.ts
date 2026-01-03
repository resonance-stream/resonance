/**
 * Data mappers for transforming GraphQL types to component-friendly types
 *
 * These mappers convert between the GraphQL response types (GqlTrack, GqlAlbum, etc.)
 * and the types used by the player store and UI components.
 */

import type { Track } from '../stores/playerStore'
import type { GqlTrack, GqlAlbum } from '../types/library'
import type { ScoredTrack } from '../types/similarity'

/**
 * Album context for mapping tracks
 * Provides album metadata when tracks don't have embedded album data
 */
export interface AlbumContext {
  id: string
  title: string
  coverArtUrl?: string
}

/**
 * Map a GraphQL track to the player Track type
 *
 * @param gqlTrack - Track from GraphQL API
 * @param albumOverrides - Optional album data to use instead of nested album
 * @returns Track compatible with playerStore
 *
 * @example
 * ```ts
 * const track = mapGqlTrackToPlayerTrack(gqlTrack)
 * playerStore.setTrack(track)
 * ```
 */
export function mapGqlTrackToPlayerTrack(
  gqlTrack: GqlTrack,
  albumOverrides?: AlbumContext
): Track {
  // Use album data from track if available, or use overrides
  const albumId = albumOverrides?.id ?? gqlTrack.album?.id ?? gqlTrack.albumId ?? ''
  const albumTitle = albumOverrides?.title ?? gqlTrack.album?.title ?? 'Unknown Album'
  const coverUrl = albumOverrides?.coverArtUrl ?? gqlTrack.album?.coverArtUrl

  // Get artist name from nested artist or fallback
  const artistName = gqlTrack.artist?.name ?? 'Unknown Artist'

  return {
    id: gqlTrack.id,
    title: gqlTrack.title,
    artist: artistName,
    albumId,
    albumTitle,
    duration: gqlTrack.durationMs / 1000, // Convert ms to seconds
    coverUrl,
  }
}

/**
 * Map multiple GraphQL tracks to player Track array
 *
 * @param gqlTracks - Array of tracks from GraphQL API
 * @param albumOverrides - Optional album data to use for all tracks
 * @returns Array of Tracks compatible with playerStore
 *
 * @example
 * ```ts
 * const tracks = mapGqlTracksToPlayerTracks(album.tracks, {
 *   id: album.id,
 *   title: album.title,
 *   coverArtUrl: album.coverArtUrl,
 * })
 * playerStore.setQueue(tracks)
 * ```
 */
export function mapGqlTracksToPlayerTracks(
  gqlTracks: GqlTrack[],
  albumOverrides?: AlbumContext
): Track[] {
  return gqlTracks.map((track) => mapGqlTrackToPlayerTrack(track, albumOverrides))
}

/**
 * Map album tracks with album context for optimal player experience
 *
 * This is a convenience function for the common case of playing an album.
 *
 * @param album - GraphQL album with tracks
 * @returns Array of Tracks with album context
 *
 * @example
 * ```ts
 * const { data: album } = useAlbum(albumId)
 * if (album?.tracks) {
 *   const tracks = mapAlbumToPlayerTracks(album)
 *   playerStore.setQueue(tracks)
 * }
 * ```
 */
export function mapAlbumToPlayerTracks(album: GqlAlbum): Track[] {
  if (!album.tracks) {
    return []
  }

  const albumContext = {
    id: album.id,
    title: album.title,
    coverArtUrl: album.coverArtUrl,
  }

  return mapGqlTracksToPlayerTracks(album.tracks, albumContext)
}

/**
 * Map a ScoredTrack (from similarity queries) to the player Track type
 *
 * ScoredTrack has minimal data by default, but may include a nested GqlTrack
 * with full details. When the nested track is available, we use it for
 * duration and cover art.
 *
 * @param scoredTrack - ScoredTrack from similarity API
 * @returns Track compatible with playerStore
 *
 * @example
 * ```ts
 * const playerTrack = mapScoredTrackToPlayerTrack(scoredTrack)
 * playerStore.setTrack(playerTrack)
 * ```
 */
export function mapScoredTrackToPlayerTrack(scoredTrack: ScoredTrack): Track {
  const { track } = scoredTrack

  // If we have the full track data, use it for complete information
  if (track) {
    return mapGqlTrackToPlayerTrack(track)
  }

  // Fallback when nested track is not available
  return {
    id: scoredTrack.trackId,
    title: scoredTrack.title,
    artist: scoredTrack.artistName ?? 'Unknown Artist',
    albumId: '',
    albumTitle: scoredTrack.albumTitle ?? 'Unknown Album',
    duration: 0, // Duration unavailable without full track data
    coverUrl: undefined,
  }
}

/**
 * Format duration in seconds to human-readable string
 *
 * @param seconds - Duration in seconds
 * @returns Formatted string like "3:45" or "1:02:30"
 */
export function formatDuration(seconds: number): string {
  // Handle invalid input gracefully
  if (!Number.isFinite(seconds) || seconds < 0) {
    return '0:00'
  }

  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
  }

  return `${minutes}:${secs.toString().padStart(2, '0')}`
}

/**
 * Format milliseconds to human-readable string
 *
 * @param ms - Duration in milliseconds
 * @returns Formatted string like "3:45" or "1:02:30"
 */
export function formatDurationMs(ms: number): string {
  return formatDuration(ms / 1000)
}

/**
 * Generate placeholder cover art as a data URI
 *
 * Returns a consistent placeholder SVG when no cover art is available.
 * Uses an inline data URI to avoid external dependencies.
 *
 * @param type - Type of media (album, artist, playlist)
 * @returns Data URI for placeholder SVG
 */
export function getPlaceholderCoverUrl(
  type: 'album' | 'artist' | 'playlist' = 'album'
): string {
  const label = type.charAt(0).toUpperCase()
  // Return an inline SVG data URI - no external dependencies
  return `data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='128' height='128' viewBox='0 0 128 128'%3E%3Crect fill='%23282828' width='128' height='128'/%3E%3Ctext fill='%23666' font-family='system-ui,sans-serif' font-size='48' x='50%25' y='50%25' dominant-baseline='central' text-anchor='middle'%3E${label}%3C/text%3E%3C/svg%3E`
}

/**
 * Build the streaming URL for a track
 *
 * @param trackId - Track ID to stream
 * @returns Full streaming URL for the audio player, or undefined if trackId is empty
 */
export function buildStreamUrl(trackId: string): string | undefined {
  if (!trackId) {
    return undefined
  }
  return `/api/stream/${encodeURIComponent(trackId)}`
}

/**
 * Build the cover art URL for an album
 *
 * @param coverArtPath - Path to cover art file
 * @returns Full URL to cover art image
 */
export function buildCoverArtUrl(coverArtPath: string | undefined): string | undefined {
  if (!coverArtPath) return undefined
  // Cover art is served from the API
  return `/api/cover/${encodeURIComponent(coverArtPath)}`
}
