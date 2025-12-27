// Shared types between frontend and backend
// This package provides type definitions that are shared across the monorepo

// ============================================================================
// User Types
// ============================================================================

export interface User {
  id: string
  username: string
  email: string
  avatarUrl?: string
  role: 'user' | 'admin'
  createdAt: string
  updatedAt: string
}

export interface Session {
  id: string
  userId: string
  deviceName: string
  expiresAt: string
}

// ============================================================================
// Library Types
// ============================================================================

export interface Artist {
  id: string
  name: string
  musicBrainzId?: string
  lidarrId?: number
  imageUrl?: string
  biography?: string
  genres: string[]
}

export interface Album {
  id: string
  title: string
  artistId: string
  artistName: string
  musicBrainzId?: string
  releaseDate?: string
  coverUrl?: string
  coverColors?: {
    primary: string
    secondary: string
    accent: string
  }
  genres: string[]
  trackCount: number
  duration: number
}

export interface Track {
  id: string
  title: string
  artistId: string
  artistName: string
  albumId: string
  albumTitle: string
  trackNumber: number
  discNumber: number
  duration: number
  filePath: string
  fileSize: number
  format: 'flac' | 'mp3' | 'aac' | 'ogg' | 'opus'
  bitrate?: number
  sampleRate?: number
  coverUrl?: string
  lyrics?: string
  syncedLyrics?: SyncedLyricLine[]
  aiTags?: string[]
  mood?: string
}

export interface SyncedLyricLine {
  time: number
  text: string
}

// ============================================================================
// Playlist Types
// ============================================================================

export interface Playlist {
  id: string
  name: string
  description?: string
  ownerId: string
  ownerName: string
  coverUrl?: string
  isPublic: boolean
  isCollaborative: boolean
  trackCount: number
  duration: number
  createdAt: string
  updatedAt: string
}

export interface SmartPlaylistRule {
  field: 'artist' | 'album' | 'genre' | 'year' | 'mood' | 'plays' | 'added' | 'rating'
  operator: 'equals' | 'contains' | 'startsWith' | 'greaterThan' | 'lessThan' | 'between'
  value: string | number | [number, number]
}

export interface SmartPlaylist extends Omit<Playlist, 'trackCount'> {
  rules: SmartPlaylistRule[]
  limit?: number
  sortBy?: 'random' | 'recent' | 'plays' | 'added'
}

// ============================================================================
// Playback Types
// ============================================================================

export interface PlaybackState {
  trackId: string | null
  isPlaying: boolean
  position: number
  volume: number
  isMuted: boolean
  shuffle: boolean
  repeat: 'off' | 'track' | 'queue'
  queueIds: string[]
  queueIndex: number
}

export interface Device {
  id: string
  name: string
  type: 'web' | 'mobile' | 'desktop'
  isActive: boolean
  lastSeen: string
}

// ============================================================================
// AI Types
// ============================================================================

export interface Recommendation {
  trackId: string
  score: number
  reason: string
  source: 'collaborative' | 'content-based' | 'ai'
}

export interface MoodAnalysis {
  primary: string
  secondary?: string
  energy: number
  valence: number
  danceability: number
}

export interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: string
  actions?: ChatAction[]
}

export interface ChatAction {
  type: 'play' | 'queue' | 'playlist' | 'search'
  label: string
  data: Record<string, unknown>
}

// ============================================================================
// API Response Types
// ============================================================================

export interface PaginatedResponse<T> {
  items: T[]
  total: number
  offset: number
  limit: number
  hasMore: boolean
}

export interface ApiError {
  code: string
  message: string
  details?: Record<string, unknown>
}
