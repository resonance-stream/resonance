/**
 * Music library types for Resonance - tracks, albums, artists, playlists
 */

// ============================================================================
// Core Entity Types
// ============================================================================

/**
 * Artist information
 */
export interface Artist {
  /** Unique artist ID */
  id: string;
  /** Artist name */
  name: string;
  /** Artist name for sorting (e.g., "Beatles, The") */
  sortName?: string;
  /** MusicBrainz artist ID */
  musicBrainzId?: string;
  /** Lidarr artist ID for library management */
  lidarrId?: number;
  /** Artist image URL */
  imageUrl?: string;
  /** Artist biography/description */
  biography?: string;
  /** Associated genres */
  genres: string[];
  /** Total album count */
  albumCount?: number;
  /** Total track count */
  trackCount?: number;
  /** When artist was added to library */
  addedAt?: string;
}

/**
 * Album information
 */
export interface Album {
  /** Unique album ID */
  id: string;
  /** Album title */
  title: string;
  /** Album title for sorting */
  sortTitle?: string;
  /** Primary artist ID */
  artistId: string;
  /** Primary artist name (denormalized for display) */
  artistName: string;
  /** MusicBrainz release ID */
  musicBrainzId?: string;
  /** Album release date (ISO 8601) */
  releaseDate?: string;
  /** Release year (for easier filtering) */
  year?: number;
  /** Album cover art URL */
  coverUrl?: string;
  /** Extracted cover art colors for theming */
  coverColors?: CoverColors;
  /** Album genres */
  genres: string[];
  /** Total number of tracks */
  trackCount: number;
  /** Total number of discs */
  discCount: number;
  /** Total duration in seconds */
  duration: number;
  /** Album type */
  albumType: AlbumType;
  /** When album was added to library */
  addedAt?: string;
}

/**
 * Album type classification
 */
export type AlbumType = 'album' | 'single' | 'ep' | 'compilation' | 'live' | 'remix' | 'soundtrack' | 'other';

/**
 * Cover art color palette
 */
export interface CoverColors {
  /** Primary/dominant color (hex) */
  primary: string;
  /** Secondary color (hex) */
  secondary: string;
  /** Accent color (hex) */
  accent: string;
  /** Whether to use light text on primary color */
  isDark: boolean;
}

/**
 * Track/song information
 */
export interface Track {
  /** Unique track ID */
  id: string;
  /** Track title */
  title: string;
  /** Track title for sorting */
  sortTitle?: string;
  /** Primary artist ID */
  artistId: string;
  /** Primary artist name (denormalized for display) */
  artistName: string;
  /** Album ID */
  albumId: string;
  /** Album title (denormalized for display) */
  albumTitle: string;
  /** Track number on disc */
  trackNumber: number;
  /** Disc number */
  discNumber: number;
  /** Track duration in seconds */
  duration: number;
  /** File path on server */
  filePath: string;
  /** File size in bytes */
  fileSize: number;
  /** Audio format */
  format: AudioFormat;
  /** Bitrate in kbps (for lossy formats) */
  bitrate?: number;
  /** Sample rate in Hz */
  sampleRate?: number;
  /** Bit depth (for lossless) */
  bitDepth?: number;
  /** Number of audio channels */
  channels?: number;
  /** Cover art URL (may differ from album cover) */
  coverUrl?: string;
  /** Plain text lyrics */
  lyrics?: string;
  /** Time-synced lyrics */
  syncedLyrics?: SyncedLyricLine[];
  /** AI-generated tags */
  aiTags?: string[];
  /** AI-detected mood */
  mood?: string;
  /** ReplayGain track gain (dB) */
  replayGainTrack?: number;
  /** ReplayGain album gain (dB) */
  replayGainAlbum?: number;
  /** User play count */
  playCount?: number;
  /** Last played timestamp */
  lastPlayedAt?: string;
  /** User rating (1-5) */
  rating?: number;
  /** Whether track is in user's favorites */
  isFavorite?: boolean;
  /** When track was added to library */
  addedAt?: string;
}

/**
 * Supported audio formats
 */
export type AudioFormat = 'flac' | 'alac' | 'wav' | 'aiff' | 'mp3' | 'aac' | 'm4a' | 'ogg' | 'opus' | 'wma';

/**
 * Single line of time-synced lyrics
 */
export interface SyncedLyricLine {
  /** Timestamp in seconds */
  time: number;
  /** Lyric text for this line */
  text: string;
}

// ============================================================================
// Playlist Types
// ============================================================================

/**
 * User-created playlist
 */
export interface Playlist {
  /** Unique playlist ID */
  id: string;
  /** Playlist name */
  name: string;
  /** Playlist description */
  description?: string;
  /** Owner user ID */
  ownerId: string;
  /** Owner display name */
  ownerName: string;
  /** Cover image URL */
  coverUrl?: string;
  /** Whether playlist is publicly visible */
  isPublic: boolean;
  /** Whether other users can add tracks */
  isCollaborative: boolean;
  /** Total track count */
  trackCount: number;
  /** Total duration in seconds */
  duration: number;
  /** When playlist was created */
  createdAt: string;
  /** When playlist was last modified */
  updatedAt: string;
  /** Playlist followers count (for public playlists) */
  followerCount?: number;
}

/**
 * Track within a playlist with ordering info
 */
export interface PlaylistTrack {
  /** Position in playlist (0-indexed) */
  position: number;
  /** Track ID */
  trackId: string;
  /** Full track data */
  track?: Track;
  /** Who added this track */
  addedBy: string;
  /** When track was added */
  addedAt: string;
}

/**
 * Smart playlist with auto-updating rules
 */
export interface SmartPlaylist extends Omit<Playlist, 'trackCount' | 'isCollaborative'> {
  /** Whether this is a smart playlist */
  isSmart: true;
  /** Rules for matching tracks */
  rules: SmartPlaylistRule[];
  /** Match all rules (AND) or any rule (OR) */
  matchMode: 'all' | 'any';
  /** Maximum number of tracks to include */
  limit?: number;
  /** How to sort matched tracks */
  sortBy?: SmartPlaylistSort;
  /** Sort direction */
  sortOrder?: 'asc' | 'desc';
}

/**
 * Single rule for a smart playlist
 */
export interface SmartPlaylistRule {
  /** Field to match against */
  field: SmartPlaylistField;
  /** Comparison operator */
  operator: SmartPlaylistOperator;
  /** Value to compare against */
  value: string | number | [number, number];
}

/**
 * Fields available for smart playlist rules
 */
export type SmartPlaylistField =
  | 'artist'
  | 'album'
  | 'title'
  | 'genre'
  | 'year'
  | 'mood'
  | 'plays'
  | 'rating'
  | 'added'
  | 'lastPlayed'
  | 'duration'
  | 'format'
  | 'aiTag';

/**
 * Operators for smart playlist rules
 */
export type SmartPlaylistOperator =
  | 'equals'
  | 'notEquals'
  | 'contains'
  | 'notContains'
  | 'startsWith'
  | 'endsWith'
  | 'greaterThan'
  | 'lessThan'
  | 'between'
  | 'inLast' // For date fields (days)
  | 'notInLast';

/**
 * Sort options for smart playlists
 */
export type SmartPlaylistSort =
  | 'random'
  | 'added'
  | 'lastPlayed'
  | 'plays'
  | 'rating'
  | 'title'
  | 'artist'
  | 'album'
  | 'year'
  | 'duration';

// ============================================================================
// AI/Recommendation Types
// ============================================================================

/**
 * AI-powered track recommendation
 */
export interface Recommendation {
  /** Recommended track ID */
  trackId: string;
  /** Recommendation score (0-1) */
  score: number;
  /** Human-readable reason for recommendation */
  reason: string;
  /** Source of recommendation */
  source: RecommendationSource;
}

/**
 * How the recommendation was generated
 */
export type RecommendationSource =
  | 'collaborative' // Based on similar users
  | 'content-based' // Based on audio features
  | 'ai' // Based on AI/LLM analysis
  | 'popularity' // Based on global popularity
  | 'recent' // Recently added/popular
  | 'radio'; // Radio station algorithm

/**
 * AI mood analysis result
 */
export interface MoodAnalysis {
  /** Primary detected mood */
  primary: string;
  /** Secondary mood if detected */
  secondary?: string;
  /** Energy level (0-1) */
  energy: number;
  /** Positivity/negativity (0-1, 0.5 is neutral) */
  valence: number;
  /** Danceability score (0-1) */
  danceability: number;
  /** Acoustic vs electronic (0-1) */
  acousticness?: number;
  /** Instrumental vs vocal (0-1) */
  instrumentalness?: number;
}

// ============================================================================
// AI Chat Types
// ============================================================================

/**
 * Chat message for AI assistant
 */
export interface ChatMessage {
  /** Message ID */
  id: string;
  /** Who sent the message */
  role: 'user' | 'assistant';
  /** Message content */
  content: string;
  /** When message was sent */
  timestamp: string;
  /** Actions the user can take based on this message */
  actions?: ChatAction[];
}

/**
 * Actionable item from AI assistant
 */
export interface ChatAction {
  /** Action type */
  type: 'play' | 'queue' | 'playlist' | 'search' | 'navigate';
  /** Display label for the action */
  label: string;
  /** Action-specific data */
  data: Record<string, unknown>;
}

// ============================================================================
// Search Types
// ============================================================================

/**
 * Combined search results across all entity types
 */
export interface SearchResults {
  /** Matching tracks */
  tracks: Track[];
  /** Matching albums */
  albums: Album[];
  /** Matching artists */
  artists: Artist[];
  /** Matching playlists */
  playlists: Playlist[];
  /** Total result counts by type */
  totals: {
    tracks: number;
    albums: number;
    artists: number;
    playlists: number;
  };
}

/**
 * Search filters
 */
export interface SearchFilters {
  /** Filter by type */
  types?: Array<'track' | 'album' | 'artist' | 'playlist'>;
  /** Filter by genre */
  genres?: string[];
  /** Filter by year range */
  years?: { min?: number; max?: number };
  /** Filter by duration range (seconds) */
  duration?: { min?: number; max?: number };
  /** Only show favorites */
  favoritesOnly?: boolean;
}
