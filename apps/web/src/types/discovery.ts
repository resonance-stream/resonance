/**
 * GraphQL response types for Last.fm discovery queries
 *
 * These types match the async-graphql schema from the Rust backend exactly.
 * Used by discovery hooks for similar artists and artist tags.
 */

// ============================================================================
// Discovery Entity Types (Last.fm)
// ============================================================================

/**
 * Similar artist from Last.fm with local library status
 *
 * Returned by the `similarArtists` GraphQL query, enriched with
 * information about whether the artist exists in the local library.
 */
export interface SimilarArtist {
  /** Artist name */
  name: string
  /** MusicBrainz ID (if available from Last.fm) */
  mbid: string | null
  /** Similarity score (0.0 - 1.0, higher = more similar) */
  matchScore: number
  /** Whether this artist is in the local library */
  inLibrary: boolean
  /** Local artist ID if in library (UUID) */
  localArtistId: string | null
  /** Number of tracks in local library (if in library) */
  trackCount: number | null
}

/**
 * Artist tag/genre from Last.fm
 *
 * Tags describe the artist's musical style and genres.
 * Sorted by popularity (count) from Last.fm.
 */
export interface ArtistTag {
  /** Tag name (e.g., "rock", "alternative", "electronic") */
  name: string
  /** Popularity count on Last.fm (higher = more popular) */
  count: number
}

// ============================================================================
// Query Response Types
// ============================================================================

/**
 * Response for similarArtists query
 */
export interface SimilarArtistsResponse {
  similarArtists: SimilarArtist[]
}

/**
 * Response for artistTags query
 */
export interface ArtistTagsResponse {
  artistTags: ArtistTag[]
}

// ============================================================================
// Query Variable Types
// ============================================================================

/**
 * Variables for similarArtists query
 */
export interface SimilarArtistsVariables {
  artistName: string
  limit?: number
}

/**
 * Variables for artistTags query
 */
export interface ArtistTagsVariables {
  artistName: string
}
