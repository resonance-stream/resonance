//! Shared pagination utilities for GraphQL resolvers
//!
//! This module provides constants and helper functions for consistent
//! pagination across all query resolvers.

/// Maximum items per page for top-level list queries
pub const MAX_LIMIT: i32 = 100;

/// Maximum items for search results
pub const MAX_SEARCH_LIMIT: i32 = 50;

/// Maximum items for nested relationship resolvers
pub const MAX_NESTED_LIMIT: i32 = 50;

/// Maximum tracks per playlist request
pub const MAX_PLAYLIST_TRACKS: i32 = 500;

/// Clamp pagination limit to valid range
#[inline]
pub fn clamp_limit(limit: i32, max: i32) -> i64 {
    limit.clamp(1, max) as i64
}

/// Clamp offset to non-negative
#[inline]
pub fn clamp_offset(offset: i32) -> i64 {
    offset.max(0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_limit_valid() {
        assert_eq!(clamp_limit(50, 100), 50);
    }

    #[test]
    fn test_clamp_limit_too_high() {
        assert_eq!(clamp_limit(200, 100), 100);
    }

    #[test]
    fn test_clamp_limit_too_low() {
        assert_eq!(clamp_limit(0, 100), 1);
        assert_eq!(clamp_limit(-5, 100), 1);
    }

    #[test]
    fn test_clamp_offset_valid() {
        assert_eq!(clamp_offset(10), 10);
    }

    #[test]
    fn test_clamp_offset_negative() {
        assert_eq!(clamp_offset(-5), 0);
    }
}
