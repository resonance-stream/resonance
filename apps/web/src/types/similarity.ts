/**
 * GraphQL response types for similarity queries
 *
 * These types match the async-graphql schema from the Rust backend exactly.
 * Used by similarity hooks for finding similar tracks using various methods.
 *
 * @see apps/api/src/graphql/types/search.rs for backend schema
 */

import type { GqlTrack } from './library'

// ============================================================================
// Enums (matching backend GraphQL enums - async-graphql uses PascalCase)
// ============================================================================

/**
 * Similarity method to use when finding similar tracks
 *
 * Each method uses different data sources and algorithms:
 * - Combined: Weighted blend of all methods (50% semantic, 30% acoustic, 20% categorical)
 * - Semantic: AI embeddings similarity using pgvector cosine distance
 * - Acoustic: Audio features like BPM, energy, loudness, valence, danceability
 * - Categorical: Genre and mood tag matching using weighted Jaccard similarity
 */
export type SimilarityMethod = 'Combined' | 'Semantic' | 'Acoustic' | 'Categorical'

/**
 * The similarity algorithm that produced a match
 *
 * Indicates which algorithm was primarily responsible for finding
 * a particular similar track in the results.
 */
export type SimilarityType = 'Semantic' | 'Acoustic' | 'Categorical' | 'Combined'

// ============================================================================
// Core Entity Types
// ============================================================================

/**
 * A track with its relevance/similarity score
 *
 * Used for semantic search results and mood-based discovery.
 * The score represents how well the track matches the query.
 */
export interface ScoredTrack {
  /** Track UUID */
  trackId: string
  /** Track title */
  title: string
  /** Artist name (if available) */
  artistName: string | null
  /** Album title (if available) */
  albumTitle: string | null
  /** Relevance/similarity score (0.0 - 1.0, higher = more relevant) */
  score: number
  /** Full track details (when requested via resolver) */
  track?: GqlTrack | null
}

/**
 * A track with its similarity score and the method used to find it
 *
 * Extends ScoredTrack with information about which similarity
 * algorithm produced this match.
 */
export type SimilarTrack = ScoredTrack & {
  /** The type of similarity used for this match */
  similarityType: SimilarityType
}

// ============================================================================
// Query Response Types
// ============================================================================

/**
 * Response for similarTracks query (returns ScoredTrack without similarityType)
 */
export interface SimilarTracksResponse {
  similarTracks: ScoredTrack[]
}

/**
 * Response for similarTracksByMethod query (returns SimilarTrack with similarityType)
 */
export interface SimilarTracksByMethodResponse {
  similarTracksByMethod: SimilarTrack[]
}

/**
 * Response for similarTracks query with nested track data
 */
export interface SimilarTracksWithDetailsResponse {
  similarTracks: (SimilarTrack & { track: GqlTrack | null })[]
}

/**
 * Response for semantic search query
 */
export interface SemanticSearchResponse {
  semanticSearch: {
    /** Matching tracks with relevance scores */
    tracks: ScoredTrack[]
    /** How the AI interpreted the query (for display to user) */
    interpretation: string | null
  }
}

/**
 * Response for mood-based track discovery
 */
export interface TracksByMoodResponse {
  tracksByMood: ScoredTrack[]
}

// ============================================================================
// Query Variable Types
// ============================================================================

/**
 * Variables for similarTracks query
 */
export interface SimilarTracksVariables {
  /** ID of the track to find similar tracks for */
  trackId: string
  /** Maximum number of similar tracks to return */
  limit?: number
  /** Similarity method to use (defaults to Combined) */
  method?: SimilarityMethod
}

/**
 * Variables for semantic search query
 */
export interface SemanticSearchVariables {
  /** Natural language search query */
  query: string
  /** Maximum number of results to return */
  limit?: number
}

/**
 * Variables for tracksByMood query
 */
export interface TracksByMoodVariables {
  /** Mood to search for (e.g., "happy", "energetic", "melancholic") */
  mood: string
  /** Maximum number of tracks to return */
  limit?: number
}
