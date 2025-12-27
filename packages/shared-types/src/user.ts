/**
 * User, session, and preferences types for Resonance
 */

import type { EqualizerSettings, CrossfadeSettings, NormalizationSettings, PlaybackQuality } from './player.js';

// ============================================================================
// User Types
// ============================================================================

/**
 * User account information
 */
export interface User {
  /** Unique user ID */
  id: string;
  /** Unique username */
  username: string;
  /** User email address */
  email: string;
  /** Display name (may differ from username) */
  displayName?: string;
  /** Avatar image URL */
  avatarUrl?: string;
  /** User role for permissions */
  role: UserRole;
  /** Whether email is verified */
  emailVerified: boolean;
  /** When account was created */
  createdAt: string;
  /** When account was last updated */
  updatedAt: string;
  /** Last login timestamp */
  lastLoginAt?: string;
}

/**
 * User role for access control.
 * Values are always lowercase (the backend may return PascalCase,
 * but the frontend normalizes to lowercase in authStore).
 */
export type UserRole = 'user' | 'admin' | 'guest';

/**
 * Public user profile (for sharing/display)
 */
export interface UserProfile {
  /** User ID */
  id: string;
  /** Username */
  username: string;
  /** Display name */
  displayName?: string;
  /** Avatar URL */
  avatarUrl?: string;
  /** Bio/description */
  bio?: string;
  /** Public playlists count */
  publicPlaylistCount: number;
  /** Follower count */
  followerCount: number;
  /** Following count */
  followingCount: number;
  /** Whether current user follows this user */
  isFollowing?: boolean;
}

// ============================================================================
// Session Types
// ============================================================================

/**
 * Active user session
 */
export interface Session {
  /** Session ID */
  id: string;
  /** User ID this session belongs to */
  userId: string;
  /** Device/browser name */
  deviceName: string;
  /** Device type */
  deviceType: 'web' | 'mobile' | 'desktop';
  /** IP address (masked for privacy) */
  ipAddress?: string;
  /** Approximate location from IP */
  location?: string;
  /** User agent string */
  userAgent?: string;
  /** When session was created */
  createdAt: string;
  /** When session expires */
  expiresAt: string;
  /** Last activity timestamp */
  lastActiveAt: string;
  /** Whether this is the current session */
  isCurrent?: boolean;
}

/**
 * Authentication tokens
 */
export interface AuthTokens {
  /** JWT access token */
  accessToken: string;
  /** Refresh token for obtaining new access tokens */
  refreshToken: string;
  /** Access token expiration (seconds from now) */
  expiresIn: number;
  /** Token type (always "Bearer") */
  tokenType: 'Bearer';
}

/**
 * Login request
 */
export interface LoginRequest {
  /** Username or email */
  usernameOrEmail: string;
  /** Password */
  password: string;
  /** Remember me (longer session) */
  rememberMe?: boolean;
}

/**
 * Registration request
 */
export interface RegisterRequest {
  /** Desired username */
  username: string;
  /** Email address */
  email: string;
  /** Password */
  password: string;
  /** Display name (optional) */
  displayName?: string;
}

// ============================================================================
// Preferences Types
// ============================================================================

/**
 * User preferences/settings
 */
export interface UserPreferences {
  /** Audio playback preferences */
  audio: AudioPreferences;
  /** UI/display preferences */
  ui: UIPreferences;
  /** Privacy preferences */
  privacy: PrivacyPreferences;
  /** Notification preferences */
  notifications: NotificationPreferences;
  /** Integration preferences */
  integrations: IntegrationPreferences;
}

/**
 * Audio-related preferences
 */
export interface AudioPreferences {
  /** Default playback quality */
  defaultQuality: PlaybackQuality;
  /** Streaming quality on cellular (mobile) */
  cellularQuality: PlaybackQuality;
  /** Download quality for offline */
  downloadQuality: PlaybackQuality;
  /** Normalize volume across tracks */
  normalization: NormalizationSettings;
  /** Crossfade between tracks */
  crossfade: CrossfadeSettings;
  /** Enable gapless playback */
  gapless: boolean;
  /** Equalizer settings */
  equalizer: EqualizerSettings;
  /** Auto-play when queue ends */
  autoPlay: boolean;
  /** Auto-play similar tracks when queue ends */
  autoPlaySimilar: boolean;
}

/**
 * UI/display preferences
 */
export interface UIPreferences {
  /** Color theme */
  theme: 'light' | 'dark' | 'system';
  /** Accent color (hex) */
  accentColor?: string;
  /** Whether to use album art colors for theming */
  dynamicColors: boolean;
  /** Default view for library */
  libraryView: 'grid' | 'list';
  /** Items per page in lists */
  pageSize: number;
  /** Show explicit content warning */
  explicitContentWarning: boolean;
  /** Default sorting for various views */
  defaultSorts: {
    artists: 'name' | 'recent' | 'plays';
    albums: 'title' | 'artist' | 'year' | 'recent' | 'plays';
    tracks: 'title' | 'artist' | 'album' | 'recent' | 'plays';
    playlists: 'name' | 'recent' | 'updated';
  };
  /** Show lyrics by default when available */
  showLyrics: boolean;
  /** Language preference */
  language: string;
  /** Compact mode (less whitespace) */
  compactMode: boolean;
}

/**
 * Privacy-related preferences
 */
export interface PrivacyPreferences {
  /** Whether listening activity is public */
  publicListeningActivity: boolean;
  /** Whether playlists are public by default */
  publicPlaylistsByDefault: boolean;
  /** Show in search results */
  discoverableProfile: boolean;
  /** Allow messages from non-followers */
  allowMessagesFromNonFollowers: boolean;
  /** Include in recommendations for others */
  includeInRecommendations: boolean;
  /** Share listening stats with AI for better recommendations */
  shareStatsWithAI: boolean;
}

/**
 * Notification preferences
 */
export interface NotificationPreferences {
  /** Enable push notifications */
  pushEnabled: boolean;
  /** Email notification settings */
  email: {
    /** New followers */
    newFollowers: boolean;
    /** Playlist updates from followed users */
    playlistUpdates: boolean;
    /** New releases from followed artists */
    newReleases: boolean;
    /** Weekly listening report */
    weeklyReport: boolean;
    /** Security alerts */
    securityAlerts: boolean;
  };
  /** In-app notification settings */
  inApp: {
    /** New followers */
    newFollowers: boolean;
    /** Playlist updates */
    playlistUpdates: boolean;
    /** New releases */
    newReleases: boolean;
    /** AI recommendations */
    recommendations: boolean;
  };
}

/**
 * Third-party integration preferences
 */
export interface IntegrationPreferences {
  /** ListenBrainz scrobbling */
  listenbrainz: {
    enabled: boolean;
    username?: string;
  };
  /** Last.fm scrobbling */
  lastfm: {
    enabled: boolean;
    username?: string;
  };
  /** Discord Rich Presence */
  discord: {
    enabled: boolean;
    showAlbumArt: boolean;
    showTimeRemaining: boolean;
  };
  /** Lidarr integration */
  lidarr: {
    autoImport: boolean;
    monitorNewReleases: boolean;
  };
}

// ============================================================================
// Activity Types
// ============================================================================

/**
 * User activity feed item
 */
export interface ActivityItem {
  /** Activity ID */
  id: string;
  /** User who performed the action */
  userId: string;
  /** User display info */
  user: {
    username: string;
    displayName?: string;
    avatarUrl?: string;
  };
  /** Type of activity */
  type: ActivityType;
  /** Activity-specific data */
  data: Record<string, unknown>;
  /** When activity occurred */
  timestamp: string;
}

/**
 * Types of user activities
 */
export type ActivityType =
  | 'listened'
  | 'liked_track'
  | 'liked_album'
  | 'liked_artist'
  | 'created_playlist'
  | 'followed_user'
  | 'followed_playlist'
  | 'added_to_playlist';

// ============================================================================
// Following Types
// ============================================================================

/**
 * Following relationship
 */
export interface Follow {
  /** User who is following */
  followerId: string;
  /** User being followed */
  followingId: string;
  /** When the follow was created */
  createdAt: string;
}
