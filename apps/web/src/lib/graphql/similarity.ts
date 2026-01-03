/**
 * GraphQL similarity queries for Resonance
 *
 * Contains queries for finding similar tracks using different algorithms:
 * - Combined: Weighted blend of all similarity methods
 * - Semantic: AI embeddings similarity
 * - Acoustic: Audio features (BPM, energy, loudness, valence, danceability)
 * - Categorical: Genre and mood tag matching
 */

import { gql } from 'graphql-request'

/**
 * Find tracks similar to a given track using combined similarity.
 *
 * Uses a weighted blend of semantic (50%), acoustic (30%), and categorical (20%)
 * similarity to find the most similar tracks in your library.
 *
 * Returns tracks with a similarity score (0-1).
 */
export const SIMILAR_TRACKS_QUERY = gql`
  query SimilarTracks($trackId: ID!, $limit: Int) {
    similarTracks(trackId: $trackId, limit: $limit) {
      trackId
      title
      artistName
      albumTitle
      score
      track {
        id
        title
        duration
        trackNumber
        artist {
          id
          name
        }
        album {
          id
          title
          artworkUrl
        }
      }
    }
  }
`

/**
 * Find tracks similar to a given track using a specific similarity method.
 *
 * Available methods:
 * - COMBINED: Weighted blend (50% semantic, 30% acoustic, 20% categorical)
 * - SEMANTIC: AI embeddings similarity (requires tracks to have embeddings)
 * - ACOUSTIC: Audio features (BPM, energy, loudness, valence, danceability)
 * - CATEGORICAL: Genre and mood tag matching
 *
 * Returns tracks with a similarity score (0-1) and the similarity type used.
 */
export const SIMILAR_TRACKS_BY_METHOD_QUERY = gql`
  query SimilarTracksByMethod($trackId: ID!, $method: SimilarityMethod!, $limit: Int) {
    similarTracksByMethod(trackId: $trackId, method: $method, limit: $limit) {
      trackId
      title
      artistName
      albumTitle
      score
      similarityType
      track {
        id
        title
        duration
        trackNumber
        artist {
          id
          name
        }
        album {
          id
          title
          artworkUrl
        }
      }
    }
  }
`
