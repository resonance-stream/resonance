/**
 * GraphQL library queries for Resonance
 *
 * Contains queries for browsing the music library:
 * - Artists: list, search, by ID
 * - Albums: list, search, recent, by ID
 * - Tracks: list, search, top tracks, by ID
 * - Playlists: user playlists, public playlists, by ID
 */

import { gql } from 'graphql-request'

// ============ Artist Queries ============

/**
 * Get a single artist by ID with albums and top tracks
 */
export const ARTIST_QUERY = gql`
  query Artist($id: ID!) {
    artist(id: $id) {
      id
      name
      sortName
      mbid
      biography
      imageUrl
      genres
      albumCount
      createdAt
      updatedAt
      albums(limit: 50) {
        id
        title
        releaseDate
        releaseYear
        albumType
        coverArtUrl
        totalTracks
      }
      topTracks(limit: 10) {
        id
        title
        durationMs
        formattedDuration
        trackNumber
        streamUrl
        album {
          id
          title
          coverArtUrl
        }
      }
    }
  }
`

/**
 * List all artists with pagination
 */
export const ARTISTS_QUERY = gql`
  query Artists($limit: Int, $offset: Int) {
    artists(limit: $limit, offset: $offset) {
      id
      name
      imageUrl
      genres
      albumCount
    }
  }
`

/**
 * Search artists by name
 */
export const SEARCH_ARTISTS_QUERY = gql`
  query SearchArtists($query: String!, $limit: Int) {
    searchArtists(query: $query, limit: $limit) {
      id
      name
      imageUrl
      genres
    }
  }
`

/**
 * Get artists by genre
 */
export const ARTISTS_BY_GENRE_QUERY = gql`
  query ArtistsByGenre($genre: String!, $limit: Int, $offset: Int) {
    artistsByGenre(genre: $genre, limit: $limit, offset: $offset) {
      id
      name
      imageUrl
      genres
      albumCount
    }
  }
`

// ============ Album Queries ============

/**
 * Get a single album by ID with tracks
 */
export const ALBUM_QUERY = gql`
  query Album($id: ID!) {
    album(id: $id) {
      id
      title
      artistId
      mbid
      releaseDate
      releaseYear
      albumType
      genres
      totalTracks
      totalDurationMs
      formattedDuration
      coverArtPath
      coverArtUrl
      coverArtColors {
        primary
        secondary
        accent
        vibrant
        muted
      }
      createdAt
      updatedAt
      artist {
        id
        name
      }
      tracks(limit: 100) {
        id
        title
        trackNumber
        discNumber
        durationMs
        formattedDuration
        fileFormat
        bitRate
        sampleRate
        bitDepth
        isHires
        isLossless
        explicit
        playCount
        streamUrl
      }
    }
  }
`

/**
 * List all albums with pagination
 */
export const ALBUMS_QUERY = gql`
  query Albums($limit: Int, $offset: Int) {
    albums(limit: $limit, offset: $offset) {
      id
      title
      artistId
      releaseYear
      albumType
      coverArtUrl
      artist {
        id
        name
      }
    }
  }
`

/**
 * Get albums by artist
 */
export const ALBUMS_BY_ARTIST_QUERY = gql`
  query AlbumsByArtist($artistId: ID!, $limit: Int, $offset: Int) {
    albumsByArtist(artistId: $artistId, limit: $limit, offset: $offset) {
      id
      title
      releaseYear
      albumType
      coverArtUrl
      totalTracks
    }
  }
`

/**
 * Search albums by title
 */
export const SEARCH_ALBUMS_QUERY = gql`
  query SearchAlbums($query: String!, $limit: Int) {
    searchAlbums(query: $query, limit: $limit) {
      id
      title
      coverArtUrl
      releaseYear
      albumType
      artist {
        id
        name
      }
    }
  }
`

/**
 * Get recently added albums
 */
export const RECENT_ALBUMS_QUERY = gql`
  query RecentAlbums($limit: Int) {
    recentAlbums(limit: $limit) {
      id
      title
      coverArtUrl
      releaseYear
      artist {
        id
        name
      }
    }
  }
`

// ============ Track Queries ============

/**
 * Get a single track by ID
 */
export const TRACK_QUERY = gql`
  query Track($id: ID!) {
    track(id: $id) {
      id
      title
      artistId
      albumId
      mbid
      fileFormat
      durationMs
      formattedDuration
      bitRate
      sampleRate
      bitDepth
      isHires
      isLossless
      trackNumber
      discNumber
      genres
      explicit
      lyrics
      hasSyncedLyrics
      audioFeatures {
        bpm
        key
        mode
        loudness
        energy
        danceability
        valence
        acousticness
        instrumentalness
        speechiness
      }
      aiMood
      aiTags
      aiDescription
      playCount
      skipCount
      lastPlayedAt
      streamUrl
      createdAt
      updatedAt
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

/**
 * List all tracks with pagination
 */
export const TRACKS_QUERY = gql`
  query Tracks($limit: Int, $offset: Int) {
    tracks(limit: $limit, offset: $offset) {
      id
      title
      durationMs
      formattedDuration
      fileFormat
      isHires
      isLossless
      playCount
      streamUrl
      artist {
        id
        name
      }
      album {
        id
        title
        coverArtUrl
      }
    }
  }
`

/**
 * Get tracks by album
 */
export const TRACKS_BY_ALBUM_QUERY = gql`
  query TracksByAlbum($albumId: ID!, $limit: Int, $offset: Int) {
    tracksByAlbum(albumId: $albumId, limit: $limit, offset: $offset) {
      id
      title
      trackNumber
      discNumber
      durationMs
      formattedDuration
      fileFormat
      isHires
      isLossless
      explicit
      streamUrl
    }
  }
`

/**
 * Get tracks by artist
 */
export const TRACKS_BY_ARTIST_QUERY = gql`
  query TracksByArtist($artistId: ID!, $limit: Int, $offset: Int) {
    tracksByArtist(artistId: $artistId, limit: $limit, offset: $offset) {
      id
      title
      durationMs
      formattedDuration
      playCount
      streamUrl
      album {
        id
        title
        coverArtUrl
      }
    }
  }
`

/**
 * Search tracks by title
 */
export const SEARCH_TRACKS_QUERY = gql`
  query SearchTracks($query: String!, $limit: Int) {
    searchTracks(query: $query, limit: $limit) {
      id
      title
      durationMs
      formattedDuration
      streamUrl
      artist {
        id
        name
      }
      album {
        id
        title
        coverArtUrl
      }
    }
  }
`

/**
 * Get top played tracks globally
 */
export const TOP_TRACKS_QUERY = gql`
  query TopTracks($limit: Int) {
    topTracks(limit: $limit) {
      id
      title
      durationMs
      formattedDuration
      playCount
      streamUrl
      artist {
        id
        name
      }
      album {
        id
        title
        coverArtUrl
      }
    }
  }
`

// ============ Playlist Queries ============

/**
 * Get a single playlist by ID with tracks
 */
export const PLAYLIST_QUERY = gql`
  query Playlist($id: ID!) {
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
      tracks(limit: 200) {
        position
        addedBy
        addedAt
        track {
          id
          title
          durationMs
          formattedDuration
          fileFormat
          isHires
          isLossless
          streamUrl
          artist {
            id
            name
          }
          album {
            id
            title
            coverArtUrl
          }
        }
      }
    }
  }
`

/**
 * Get playlists owned by the authenticated user
 */
export const MY_PLAYLISTS_QUERY = gql`
  query MyPlaylists($limit: Int, $offset: Int) {
    myPlaylists(limit: $limit, offset: $offset) {
      id
      name
      description
      imageUrl
      playlistType
      trackCount
      totalDurationMs
      formattedDuration
      updatedAt
    }
  }
`

/**
 * Browse public playlists
 */
export const PUBLIC_PLAYLISTS_QUERY = gql`
  query PublicPlaylists($limit: Int, $offset: Int) {
    publicPlaylists(limit: $limit, offset: $offset) {
      id
      name
      description
      imageUrl
      playlistType
      trackCount
    }
  }
`

// ============ Combined Search Query ============

/**
 * Search across all content types at once
 */
export const COMBINED_SEARCH_QUERY = gql`
  query CombinedSearch($query: String!, $limit: Int) {
    searchArtists(query: $query, limit: $limit) {
      id
      name
      imageUrl
      genres
    }
    searchAlbums(query: $query, limit: $limit) {
      id
      title
      coverArtUrl
      releaseYear
      artist {
        id
        name
      }
    }
    searchTracks(query: $query, limit: $limit) {
      id
      title
      durationMs
      formattedDuration
      streamUrl
      artist {
        id
        name
      }
      album {
        id
        title
        coverArtUrl
      }
    }
  }
`
