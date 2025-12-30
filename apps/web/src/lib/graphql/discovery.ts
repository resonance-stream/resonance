/**
 * GraphQL discovery queries for Resonance
 *
 * Contains queries for artist discovery via Last.fm:
 * - Similar artists with local library status
 * - Artist genre/style tags
 */

import { gql } from 'graphql-request'

/**
 * Get similar artists from Last.fm, enriched with local library info
 *
 * Returns artists musically similar to the given artist, with:
 * - Similarity score (0-1)
 * - Whether they're in your local library
 * - Local artist ID and track count if in library
 */
export const SIMILAR_ARTISTS_QUERY = gql`
  query SimilarArtists($artistName: String!, $limit: Int) {
    similarArtists(artistName: $artistName, limit: $limit) {
      name
      mbid
      matchScore
      inLibrary
      localArtistId
      trackCount
    }
  }
`

/**
 * Get genre/style tags for an artist from Last.fm
 *
 * Returns tags like "rock", "alternative", "electronic" with their
 * popularity count on Last.fm.
 */
export const ARTIST_TAGS_QUERY = gql`
  query ArtistTags($artistName: String!) {
    artistTags(artistName: $artistName) {
      name
      count
    }
  }
`
