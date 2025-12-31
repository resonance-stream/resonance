/**
 * Admin dashboard types for Resonance
 */

import type { UserRole } from './user.js';

// ============================================================================
// System Statistics
// ============================================================================

/**
 * System-wide statistics for the admin dashboard
 */
export interface SystemStats {
  /** Total number of registered users */
  userCount: number;
  /** Total number of tracks in the library */
  trackCount: number;
  /** Total number of albums in the library */
  albumCount: number;
  /** Total number of artists in the library */
  artistCount: number;
  /** Total duration of all tracks in milliseconds */
  totalDurationMs: number;
  /** Total file size of all tracks in bytes */
  totalFileSizeBytes: number;
  /** Number of currently active sessions */
  activeSessionCount: number;
  /** Formatted total duration (e.g., "1,234 hours") */
  totalDurationFormatted: string;
  /** Formatted total file size (e.g., "45.6 GB") */
  totalFileSizeFormatted: string;
}

// ============================================================================
// User Management Types
// ============================================================================

/**
 * User item in the admin user list
 */
export interface AdminUserListItem {
  /** Unique user identifier */
  id: string;
  /** User's email address */
  email: string;
  /** Display name shown in the UI */
  displayName: string;
  /** URL to user's avatar image */
  avatarUrl?: string;
  /** User's role */
  role: UserRole;
  /** Whether email has been verified */
  emailVerified: boolean;
  /** Last time user was seen online */
  lastSeenAt?: string;
  /** Account creation timestamp */
  createdAt: string;
  /** Number of active sessions for this user */
  sessionCount: number;
}

/**
 * Paginated list of users for admin management
 */
export interface AdminUserList {
  /** List of users on this page */
  users: AdminUserListItem[];
  /** Total number of users (for pagination) */
  totalCount: number;
  /** Whether there are more users after this page */
  hasNextPage: boolean;
}

/**
 * Session information for admin user detail view
 */
export interface AdminSession {
  /** Session identifier */
  id: string;
  /** Device type (desktop, mobile, tablet, web, tv) */
  deviceType?: string;
  /** Human-readable device name */
  deviceName?: string;
  /** Client IP address */
  ipAddress?: string;
  /** Client user agent string */
  userAgent?: string;
  /** Whether session is currently active */
  isActive: boolean;
  /** Last activity timestamp */
  lastActiveAt: string;
  /** Session creation timestamp */
  createdAt: string;
}

/**
 * Detailed user info with sessions for admin detail view
 */
export interface AdminUserDetail {
  /** User information */
  user: AdminUserListItem;
  /** User's active and recent sessions */
  sessions: AdminSession[];
}

// ============================================================================
// Admin Operation Types
// ============================================================================

/**
 * Result of an admin operation
 */
export interface AdminOperationResult {
  /** Whether the operation was successful */
  success: boolean;
  /** Optional message describing the result */
  message?: string;
}

/**
 * Result of a session invalidation operation
 */
export interface InvalidateSessionsResult {
  /** Whether the operation was successful */
  success: boolean;
  /** Number of sessions that were invalidated */
  sessionsInvalidated: number;
}

/**
 * Input for updating a user's role
 */
export interface UpdateUserRoleInput {
  /** The ID of the user to update */
  userId: string;
  /** The new role to assign */
  role: UserRole;
}
