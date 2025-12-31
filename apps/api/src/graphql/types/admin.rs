//! Admin GraphQL types for dashboard and user management
//!
//! This module defines the GraphQL types for admin-only operations including
//! system statistics, user management, and session information.

use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::user::UserRole;
use crate::repositories::{AdminSessionRow, AdminUserRow, SystemStats as DbSystemStats};

/// System-wide statistics for the admin dashboard
#[derive(Debug, Clone, SimpleObject)]
pub struct SystemStats {
    /// Total number of registered users
    pub user_count: i64,
    /// Total number of tracks in the library
    pub track_count: i64,
    /// Total number of albums in the library
    pub album_count: i64,
    /// Total number of artists in the library
    pub artist_count: i64,
    /// Total duration of all tracks in milliseconds
    pub total_duration_ms: i64,
    /// Total file size of all tracks in bytes
    pub total_file_size_bytes: i64,
    /// Number of currently active sessions
    pub active_session_count: i64,
    /// Formatted total duration (e.g., "1,234 hours")
    pub total_duration_formatted: String,
    /// Formatted total file size (e.g., "45.6 GB")
    pub total_file_size_formatted: String,
}

impl From<DbSystemStats> for SystemStats {
    fn from(stats: DbSystemStats) -> Self {
        let hours = stats.total_duration_ms / 3600000;
        let gb = stats.total_file_size_bytes as f64 / 1_073_741_824.0;

        Self {
            user_count: stats.user_count,
            track_count: stats.track_count,
            album_count: stats.album_count,
            artist_count: stats.artist_count,
            total_duration_ms: stats.total_duration_ms,
            total_file_size_bytes: stats.total_file_size_bytes,
            active_session_count: stats.active_session_count,
            total_duration_formatted: format!("{} hours", hours),
            total_file_size_formatted: format!("{:.1} GB", gb),
        }
    }
}

/// Paginated list of users for admin management
#[derive(Debug, Clone, SimpleObject)]
pub struct AdminUserList {
    /// List of users on this page
    pub users: Vec<AdminUserListItem>,
    /// Total number of users (for pagination)
    pub total_count: i64,
    /// Whether there are more users after this page
    pub has_next_page: bool,
}

/// User item in the admin user list (simplified for list view)
#[derive(Debug, Clone, SimpleObject)]
pub struct AdminUserListItem {
    /// Unique user identifier
    pub id: Uuid,
    /// User's email address
    pub email: String,
    /// Display name shown in the UI
    pub display_name: String,
    /// URL to user's avatar image
    pub avatar_url: Option<String>,
    /// User's role
    pub role: UserRole,
    /// Whether email has been verified
    pub email_verified: bool,
    /// Last time user was seen online
    pub last_seen_at: Option<DateTime<Utc>>,
    /// Account creation timestamp
    pub created_at: DateTime<Utc>,
    /// Number of active sessions for this user
    pub session_count: i64,
}

impl From<AdminUserRow> for AdminUserListItem {
    fn from(row: AdminUserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            avatar_url: row.avatar_url,
            role: row.role.into(),
            email_verified: row.email_verified,
            last_seen_at: row.last_seen_at,
            created_at: row.created_at,
            session_count: row.session_count,
        }
    }
}

/// Session information for admin user detail view
#[derive(Debug, Clone, SimpleObject)]
pub struct AdminSession {
    /// Session identifier
    pub id: Uuid,
    /// Device type (desktop, mobile, tablet, web, tv)
    pub device_type: Option<String>,
    /// Human-readable device name
    pub device_name: Option<String>,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Client user agent string
    pub user_agent: Option<String>,
    /// Whether session is currently active
    pub is_active: bool,
    /// Last activity timestamp
    pub last_active_at: DateTime<Utc>,
    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
}

impl From<AdminSessionRow> for AdminSession {
    fn from(row: AdminSessionRow) -> Self {
        Self {
            id: row.id,
            device_type: row.device_type,
            device_name: row.device_name,
            ip_address: row.ip_address,
            user_agent: row.user_agent,
            is_active: row.is_active,
            last_active_at: row.last_active_at,
            created_at: row.created_at,
        }
    }
}

/// Detailed user info with sessions for admin detail view
#[derive(Debug, Clone, SimpleObject)]
pub struct AdminUserDetail {
    /// User information
    pub user: AdminUserListItem,
    /// User's active and recent sessions
    pub sessions: Vec<AdminSession>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::SystemStats as DbStats;

    #[test]
    fn test_system_stats_formatting() {
        let db_stats = DbStats {
            user_count: 10,
            track_count: 1000,
            album_count: 100,
            artist_count: 50,
            total_duration_ms: 3600000 * 100,       // 100 hours
            total_file_size_bytes: 1073741824 * 50, // 50 GB
            active_session_count: 5,
        };

        let stats: SystemStats = db_stats.into();
        assert_eq!(stats.user_count, 10);
        assert_eq!(stats.total_duration_formatted, "100 hours");
        assert_eq!(stats.total_file_size_formatted, "50.0 GB");
    }
}
