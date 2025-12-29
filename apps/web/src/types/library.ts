/**
 * GraphQL response types for music library queries
 *
 * These types match the async-graphql schema from the Rust backend exactly.
 * They are separate from shared-types because GraphQL field names differ
 * (e.g., coverArtUrl vs coverUrl, mbid vs musicBrainzId).
 */

// ============================================================================
// Enums (matching backend GraphQL enums - async-graphql uses PascalCase)
// ============================================================================

export type AlbumType =
  | 'Album'
  | 'Single'
  | 'Ep'
  | 'Compilation'
  | 'Live'
  | 'Remix'
  | 'Soundtrack'
  | 'Other'

export type AudioFormat =
  | 'Flac'
  | 'Mp3'
  | 'Aac'
  | 'Opus'
  | 'Ogg'
  | 'Wav'
  | 'Alac'
  | 'Other'

export type PlaylistType = 'Manual' | 'Smart' | 'Discover' | 'Radio'

// ============================================================================
// Core Entity Types
// ============================================================================

/**
 * Artist from GraphQL API
 */
export interface GqlArtist {
  id: string
  name: string
  sortName?: string
  mbid?: string
  biography?: string
  imageUrl?: string
  genres: string[]
  albumCount?: number
  createdAt: string
  updatedAt: string
  /** Nested albums (when requested in query) */
  albums?: GqlAlbum[]
  /** Top tracks by play count (when requested) */
  topTracks?: GqlTrack[]
}

/**
 * Album from GraphQL API
 */
export interface GqlAlbum {
  id: string
  title: string
  artistId: string
  mbid?: string
  releaseDate?: string
  releaseYear?: number
  albumType: AlbumType
  genres: string[]
  totalTracks?: number
  totalDurationMs?: number
  formattedDuration?: string
  coverArtPath?: string
  coverArtUrl?: string
  coverArtColors?: GqlCoverArtColors
  createdAt: string
  updatedAt: string
  /** Artist data (when requested via resolver) */
  artist?: GqlArtist
  /** Album tracks (when requested via resolver) */
  tracks?: GqlTrack[]
}

/**
 * Cover art color palette
 */
export interface GqlCoverArtColors {
  primary?: string
  secondary?: string
  accent?: string
  vibrant?: string
  muted?: string
}

/**
 * Track from GraphQL API
 */
export interface GqlTrack {
  id: string
  title: string
  albumId?: string
  artistId: string
  mbid?: string
  fileFormat: AudioFormat
  durationMs: number
  formattedDuration: string
  bitRate?: number
  sampleRate?: number
  bitDepth?: number
  isHires: boolean
  isLossless: boolean
  trackNumber?: number
  discNumber?: number
  genres: string[]
  explicit: boolean
  lyrics?: string
  hasSyncedLyrics: boolean
  audioFeatures?: GqlAudioFeatures
  aiMood: string[]
  aiTags: string[]
  aiDescription?: string
  playCount: number
  skipCount: number
  lastPlayedAt?: string
  streamUrl: string
  createdAt: string
  updatedAt: string
  /** Album data (when requested via resolver) */
  album?: GqlAlbum
  /** Artist data (when requested via resolver) */
  artist?: GqlArtist
}

/**
 * Audio features extracted from track analysis
 */
export interface GqlAudioFeatures {
  bpm?: number
  key?: string
  mode?: string
  loudness?: number
  energy?: number
  danceability?: number
  valence?: number
  acousticness?: number
  instrumentalness?: number
  speechiness?: number
}

/**
 * Playlist from GraphQL API
 */
export interface GqlPlaylist {
  id: string
  userId: string
  name: string
  description?: string
  imageUrl?: string
  isPublic: boolean
  isCollaborative: boolean
  playlistType: PlaylistType
  trackCount: number
  totalDurationMs: number
  formattedDuration: string
  createdAt: string
  updatedAt: string
  /** Playlist tracks with metadata (when requested) */
  tracks?: GqlPlaylistTrackEntry[]
}

/**
 * A track entry in a playlist with position and metadata
 */
export interface GqlPlaylistTrackEntry {
  position: number
  addedBy?: string
  addedAt: string
  track: GqlTrack
}

// ============================================================================
// Query Response Types
// ============================================================================

/**
 * Response for artist queries
 */
export interface ArtistQueryResponse {
  artist: GqlArtist | null
}

export interface ArtistsQueryResponse {
  artists: GqlArtist[]
}

export interface SearchArtistsQueryResponse {
  searchArtists: GqlArtist[]
}

/**
 * Response for album queries
 */
export interface AlbumQueryResponse {
  album: GqlAlbum | null
}

export interface AlbumsQueryResponse {
  albums: GqlAlbum[]
}

export interface RecentAlbumsQueryResponse {
  recentAlbums: GqlAlbum[]
}

export interface SearchAlbumsQueryResponse {
  searchAlbums: GqlAlbum[]
}

/**
 * Response for track queries
 */
export interface TrackQueryResponse {
  track: GqlTrack | null
}

export interface TracksQueryResponse {
  tracks: GqlTrack[]
}

export interface TopTracksQueryResponse {
  topTracks: GqlTrack[]
}

export interface SearchTracksQueryResponse {
  searchTracks: GqlTrack[]
}

/**
 * Response for playlist queries
 */
export interface PlaylistQueryResponse {
  playlist: GqlPlaylist | null
}

export interface MyPlaylistsQueryResponse {
  myPlaylists: GqlPlaylist[]
}

export interface PublicPlaylistsQueryResponse {
  publicPlaylists: GqlPlaylist[]
}

/**
 * Response for combined search
 */
export interface CombinedSearchQueryResponse {
  searchArtists: GqlArtist[]
  searchAlbums: GqlAlbum[]
  searchTracks: GqlTrack[]
}

/**
 * Response for artists by genre query
 */
export interface ArtistsByGenreQueryResponse {
  artistsByGenre: GqlArtist[]
}

/**
 * Response for albums by artist query
 */
export interface AlbumsByArtistQueryResponse {
  albumsByArtist: GqlAlbum[]
}

/**
 * Response for tracks by album query
 */
export interface TracksByAlbumQueryResponse {
  tracksByAlbum: GqlTrack[]
}

/**
 * Response for tracks by artist query
 */
export interface TracksByArtistQueryResponse {
  tracksByArtist: GqlTrack[]
}

// ============================================================================
// Query Variable Types (for type-safe query invocation)
// ============================================================================

export interface IdQueryVariables {
  id: string
}

export interface PaginationVariables {
  limit?: number
  offset?: number
}

export interface SearchQueryVariables {
  query: string
  limit?: number
}

export interface ArtistsByGenreVariables extends PaginationVariables {
  genre: string
}

export interface AlbumsByArtistVariables extends PaginationVariables {
  artistId: string
}

export interface TracksByAlbumVariables extends PaginationVariables {
  albumId: string
}

export interface TracksByArtistVariables extends PaginationVariables {
  artistId: string
}
