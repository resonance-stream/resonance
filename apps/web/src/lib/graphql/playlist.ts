/**
 * Smart Playlist GraphQL Operations
 *
 * GraphQL queries and mutations for smart playlist management.
 * Maps to backend schema in apps/api/src/graphql/mutation/playlist.rs
 */

import { gql } from 'graphql-request'

// ============================================================================
// Mutations
// ============================================================================

/**
 * Create a new playlist (manual or smart)
 *
 * For smart playlists, provide smartRules with match mode, rules, and optional
 * limit/sort settings.
 */
export const CREATE_PLAYLIST_MUTATION = gql`
  mutation CreatePlaylist($input: CreatePlaylistInput!) {
    createPlaylist(input: $input) {
      id
      userId
      name
      description
      isPublic
      playlistType
      trackCount
      totalDurationMs
      formattedDuration
      createdAt
    }
  }
`

/**
 * Refresh a smart playlist to re-evaluate rules against current library
 *
 * This will regenerate the track list based on the playlist's smart rules.
 */
export const REFRESH_SMART_PLAYLIST_MUTATION = gql`
  mutation RefreshSmartPlaylist($id: ID!) {
    refreshSmartPlaylist(id: $id) {
      id
      trackCount
      totalDurationMs
      formattedDuration
      updatedAt
    }
  }
`

/**
 * Update an existing playlist's metadata or rules
 */
export const UPDATE_PLAYLIST_MUTATION = gql`
  mutation UpdatePlaylist($id: ID!, $input: UpdatePlaylistInput!) {
    updatePlaylist(id: $id, input: $input) {
      id
      name
      description
      isPublic
      playlistType
      trackCount
      totalDurationMs
      formattedDuration
      updatedAt
    }
  }
`

/**
 * Delete a playlist
 */
export const DELETE_PLAYLIST_MUTATION = gql`
  mutation DeletePlaylist($id: ID!) {
    deletePlaylist(id: $id)
  }
`

/**
 * Add tracks to a manual playlist
 */
export const ADD_TRACKS_TO_PLAYLIST_MUTATION = gql`
  mutation AddTracksToPlaylist($playlistId: ID!, $input: AddTracksInput!) {
    addTracksToPlaylist(playlistId: $playlistId, input: $input) {
      id
      trackCount
      totalDurationMs
      formattedDuration
      updatedAt
    }
  }
`

/**
 * Remove tracks from a manual playlist
 */
export const REMOVE_TRACKS_FROM_PLAYLIST_MUTATION = gql`
  mutation RemoveTracksFromPlaylist($playlistId: ID!, $input: RemoveTracksInput!) {
    removeTracksFromPlaylist(playlistId: $playlistId, input: $input) {
      id
      trackCount
      totalDurationMs
      formattedDuration
      updatedAt
    }
  }
`

// ============================================================================
// Queries
// ============================================================================

/**
 * Get a single playlist by ID with its tracks and smart rules
 *
 * Named differently from library.ts PLAYLIST_QUERY to avoid import conflicts.
 * This version includes smart rules for the smart playlist editor.
 */
export const PLAYLIST_WITH_SMART_RULES_QUERY = gql`
  query PlaylistWithSmartRules($id: ID!) {
    playlist(id: $id) {
      id
      userId
      name
      description
      imageUrl
      isPublic
      isCollaborative
      playlistType
      trackCount
      totalDurationMs
      formattedDuration
      createdAt
      updatedAt
      smartRules {
        matchMode
        rules {
          field
          operator
          value
        }
        limit
        sortBy
        sortOrder
      }
      tracks {
        position
        addedBy
        addedAt
        track {
          id
          title
          durationMs
          formattedDuration
          streamUrl
          trackNumber
          discNumber
          genres
          aiMood
          aiTags
          fileFormat
          isHires
          isLossless
          album {
            id
            title
            coverArtUrl
          }
          artist {
            id
            name
          }
        }
      }
    }
  }
`

/**
 * Search tracks for use as seed tracks in similar_to rules
 */
export const SEARCH_TRACKS_FOR_SEEDS_QUERY = gql`
  query SearchTracksForSeeds($query: String!, $limit: Int) {
    searchTracks(query: $query, limit: $limit) {
      id
      title
      durationMs
      formattedDuration
      genres
      album {
        id
        title
        coverArtUrl
      }
      artist {
        id
        name
      }
    }
  }
`
