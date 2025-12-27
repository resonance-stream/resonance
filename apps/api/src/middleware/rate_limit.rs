//! Rate limiting middleware for Resonance API
//!
//! Provides Redis-based rate limiting to prevent brute-force attacks on
//! authentication endpoints. Uses a sliding window algorithm for accurate
//! rate limiting across distributed instances.
//!
//! When Redis is unavailable, falls back to an in-memory rate limiter that
//! provides local rate limiting per instance. This ensures rate limiting
//! remains active even during Redis outages, though limits are per-instance
//! rather than global.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::error::ApiError;

/// Rate limit configuration for a specific endpoint
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed in the window
    pub max_requests: u32,
    /// Window size in seconds
    pub window_secs: u64,
    /// Key prefix for Redis (e.g., "login", "register")
    pub key_prefix: String,
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(key_prefix: impl Into<String>, max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            key_prefix: key_prefix.into(),
        }
    }

    /// Rate limit for login: 5 attempts per 60 seconds per IP
    pub fn login() -> Self {
        Self::new("auth:login", 5, 60)
    }

    /// Rate limit for registration: 3 attempts per 3600 seconds (1 hour) per IP
    pub fn register() -> Self {
        Self::new("auth:register", 3, 3600)
    }
}

/// Entry for tracking request timestamps in the in-memory rate limiter
#[derive(Debug, Clone)]
struct RateLimitEntry {
    /// Timestamps of requests within the current window (relative to creation)
    timestamps: Vec<Instant>,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            timestamps: Vec::new(),
        }
    }

    /// Remove expired timestamps and add a new one if under the limit
    /// Returns Ok(remaining) if allowed, Err(retry_after_secs) if rate limited
    fn check_and_record(&mut self, max_requests: u32, window: Duration) -> Result<u32, u64> {
        let now = Instant::now();
        let window_start = now.checked_sub(window).unwrap_or(now);

        // Remove expired entries
        self.timestamps.retain(|&ts| ts > window_start);

        let current_count = self.timestamps.len() as u32;

        if current_count < max_requests {
            // Add new request
            self.timestamps.push(now);
            Ok(max_requests - current_count - 1)
        } else {
            // Rate limited - calculate retry after based on oldest entry
            if let Some(&oldest) = self.timestamps.first() {
                let elapsed = now.duration_since(oldest);
                let retry_after = window.saturating_sub(elapsed);
                Err(retry_after.as_secs().max(1))
            } else {
                Err(window.as_secs())
            }
        }
    }

    /// Check if this entry is empty (all timestamps expired)
    fn is_expired(&self, window: Duration) -> bool {
        let now = Instant::now();
        let window_start = now.checked_sub(window).unwrap_or(now);
        self.timestamps.iter().all(|&ts| ts <= window_start)
    }
}

/// In-memory rate limiter using a sliding window algorithm
///
/// This is used as a fallback when Redis is unavailable. It provides
/// per-instance rate limiting, which is less effective than distributed
/// rate limiting but still provides protection against brute-force attacks.
#[derive(Debug)]
pub struct InMemoryRateLimiter {
    /// Map of (key_prefix, client_id) -> rate limit entries
    entries: RwLock<HashMap<String, RateLimitEntry>>,
    /// Maximum window duration for cleanup purposes
    max_window: Duration,
    /// Last cleanup time
    last_cleanup: RwLock<Instant>,
}

impl Default for InMemoryRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRateLimiter {
    /// Create a new in-memory rate limiter
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_window: Duration::from_secs(3600), // Default to 1 hour for cleanup
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Check if a request should be rate limited
    ///
    /// Returns Ok(remaining) if allowed, Err(retry_after) if rate limited
    pub async fn check(&self, key: &str, config: &RateLimitConfig) -> Result<u32, u64> {
        let full_key = format!("{}:{}", config.key_prefix, key);
        let window = Duration::from_secs(config.window_secs);

        // Periodically cleanup expired entries (every 60 seconds)
        self.maybe_cleanup(window).await;

        let mut entries = self.entries.write().await;
        let entry = entries
            .entry(full_key.clone())
            .or_insert_with(RateLimitEntry::new);

        let result = entry.check_and_record(config.max_requests, window);

        match &result {
            Ok(remaining) => {
                debug!(key = %full_key, remaining = remaining, "In-memory rate limit check passed");
            }
            Err(retry_after) => {
                debug!(key = %full_key, retry_after = retry_after, "In-memory rate limit exceeded");
            }
        }

        result
    }

    /// Cleanup expired entries to prevent unbounded memory growth
    async fn maybe_cleanup(&self, window: Duration) {
        let cleanup_interval = Duration::from_secs(60);

        {
            let last_cleanup = self.last_cleanup.read().await;
            if last_cleanup.elapsed() < cleanup_interval {
                return;
            }
        }

        // Try to acquire write lock for cleanup
        let mut last_cleanup = self.last_cleanup.write().await;

        // Double-check after acquiring write lock
        if last_cleanup.elapsed() < cleanup_interval {
            return;
        }

        *last_cleanup = Instant::now();
        drop(last_cleanup);

        let mut entries = self.entries.write().await;
        let initial_count = entries.len();

        // Use the larger of the provided window or max_window for cleanup
        let cleanup_window = window.max(self.max_window);
        entries.retain(|_, entry| !entry.is_expired(cleanup_window));

        let removed = initial_count - entries.len();
        if removed > 0 {
            debug!(
                removed = removed,
                remaining = entries.len(),
                "Cleaned up expired rate limit entries"
            );
        }
    }

    /// Get the current number of tracked entries (for monitoring/testing)
    #[cfg(test)]
    pub async fn entry_count(&self) -> usize {
        self.entries.read().await.len()
    }
}

/// State for rate limiting middleware
#[derive(Clone)]
pub struct RateLimiter {
    redis: Arc<redis::Client>,
    fallback: Arc<InMemoryRateLimiter>,
}

impl RateLimiter {
    /// Create a new rate limiter with a Redis client
    pub fn new(redis: redis::Client) -> Self {
        Self {
            redis: Arc::new(redis),
            fallback: Arc::new(InMemoryRateLimiter::new()),
        }
    }

    /// Create a new rate limiter with a custom fallback (for testing)
    #[cfg(test)]
    pub fn with_fallback(redis: redis::Client, fallback: InMemoryRateLimiter) -> Self {
        Self {
            redis: Arc::new(redis),
            fallback: Arc::new(fallback),
        }
    }

    /// Check if a request should be rate limited
    ///
    /// Uses Redis for distributed rate limiting when available.
    /// Falls back to in-memory rate limiting when Redis is unavailable,
    /// providing per-instance protection against brute-force attacks.
    ///
    /// Returns Ok(remaining) if allowed, Err(retry_after) if rate limited
    pub async fn check(&self, key: &str, config: &RateLimitConfig) -> Result<u32, u64> {
        let full_key = format!("ratelimit:{}:{}", config.key_prefix, key);

        let mut conn = match self.redis.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                // If Redis is unavailable, use in-memory fallback rate limiter
                // This provides per-instance rate limiting instead of global,
                // which is still better than allowing unlimited requests
                warn!(
                    error = %e,
                    "Redis unavailable for rate limiting, using in-memory fallback"
                );
                return self.fallback.check(key, config).await;
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Sliding window algorithm using Redis sorted sets
        // Each request is stored with its timestamp as the score
        let _window_start = now - config.window_secs;

        // Lua script for atomic rate limiting operation:
        // 1. Remove expired entries (older than window)
        // 2. Count current entries
        // 3. If under limit, add new entry and return remaining
        // 4. If over limit, return -1 and the TTL
        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local window = tonumber(ARGV[2])
            local max_requests = tonumber(ARGV[3])
            local window_start = now - window

            -- Remove old entries
            redis.call('ZREMRANGEBYSCORE', key, 0, window_start)

            -- Count current entries
            local current = redis.call('ZCARD', key)

            if current < max_requests then
                -- Add new request with current timestamp
                redis.call('ZADD', key, now, now .. ':' .. math.random())
                -- Set expiry on the key
                redis.call('EXPIRE', key, window)
                -- Return remaining requests
                return max_requests - current - 1
            else
                -- Get oldest entry to calculate retry-after
                local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
                if #oldest >= 2 then
                    local oldest_time = tonumber(oldest[2])
                    local retry_after = oldest_time + window - now
                    return -retry_after
                end
                return -window
            end
            "#,
        );

        let result: i64 = match script
            .key(&full_key)
            .arg(now)
            .arg(config.window_secs)
            .arg(config.max_requests)
            .invoke_async(&mut conn)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                // Redis script execution failed, use in-memory fallback
                warn!(
                    error = %e,
                    key = %full_key,
                    "Rate limit check failed, using in-memory fallback"
                );
                return self.fallback.check(key, config).await;
            }
        };

        if result >= 0 {
            debug!(key = %full_key, remaining = result, "Rate limit check passed");
            Ok(result as u32)
        } else {
            let retry_after = (-result) as u64;
            debug!(key = %full_key, retry_after = retry_after, "Rate limit exceeded");
            Err(retry_after)
        }
    }
}

/// Extract client IP from request headers or connection info
pub fn extract_client_ip(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<std::net::SocketAddr>>,
) -> String {
    // Try X-Forwarded-For first (for proxied requests)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(value) = forwarded.to_str() {
            // X-Forwarded-For can contain multiple IPs, take the first (client IP)
            if let Some(ip) = value.split(',').next() {
                let ip = ip.trim();
                // Validate it's a proper IP
                if ip.parse::<IpAddr>().is_ok() {
                    return ip.to_string();
                }
            }
        }
    }

    // Try X-Real-IP (common with nginx)
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            let ip = value.trim();
            if ip.parse::<IpAddr>().is_ok() {
                return ip.to_string();
            }
        }
    }

    // Fall back to connection info
    if let Some(connect_info) = connect_info {
        return connect_info.0.ip().to_string();
    }

    // Last resort - use a placeholder (shouldn't happen in production)
    warn!("Could not determine client IP for rate limiting");
    "unknown".to_string()
}

/// Rate limiting state for auth endpoints
#[derive(Clone)]
pub struct AuthRateLimitState {
    pub limiter: RateLimiter,
    pub login_config: RateLimitConfig,
    pub register_config: RateLimitConfig,
}

impl AuthRateLimitState {
    /// Create new auth rate limit state with default configurations
    pub fn new(redis_client: redis::Client) -> Self {
        Self {
            limiter: RateLimiter::new(redis_client),
            login_config: RateLimitConfig::login(),
            register_config: RateLimitConfig::register(),
        }
    }

    /// Create with custom configurations
    pub fn with_config(
        redis_client: redis::Client,
        login_config: RateLimitConfig,
        register_config: RateLimitConfig,
    ) -> Self {
        Self {
            limiter: RateLimiter::new(redis_client),
            login_config,
            register_config,
        }
    }
}

/// Middleware for rate limiting login requests
pub async fn login_rate_limit(
    State(state): State<AuthRateLimitState>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    let client_ip = extract_client_ip(&headers, None);

    match state.limiter.check(&client_ip, &state.login_config).await {
        Ok(remaining) => {
            let mut response = next.run(request).await;
            // Add rate limit headers to response
            response.headers_mut().insert(
                "X-RateLimit-Limit",
                state.login_config.max_requests.to_string().parse().unwrap(),
            );
            response.headers_mut().insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap(),
            );
            response.headers_mut().insert(
                "X-RateLimit-Reset",
                state.login_config.window_secs.to_string().parse().unwrap(),
            );
            response
        }
        Err(retry_after) => {
            warn!(
                ip = %client_ip,
                retry_after = retry_after,
                "Login rate limit exceeded"
            );
            ApiError::RateLimited { retry_after }.into_response()
        }
    }
}

/// Middleware for rate limiting registration requests
pub async fn register_rate_limit(
    State(state): State<AuthRateLimitState>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    let client_ip = extract_client_ip(&headers, None);

    match state
        .limiter
        .check(&client_ip, &state.register_config)
        .await
    {
        Ok(remaining) => {
            let mut response = next.run(request).await;
            // Add rate limit headers to response
            response.headers_mut().insert(
                "X-RateLimit-Limit",
                state
                    .register_config
                    .max_requests
                    .to_string()
                    .parse()
                    .unwrap(),
            );
            response.headers_mut().insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap(),
            );
            response.headers_mut().insert(
                "X-RateLimit-Reset",
                state
                    .register_config
                    .window_secs
                    .to_string()
                    .parse()
                    .unwrap(),
            );
            response
        }
        Err(retry_after) => {
            warn!(
                ip = %client_ip,
                retry_after = retry_after,
                "Registration rate limit exceeded"
            );
            ApiError::RateLimited { retry_after }.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_client_ip_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.1, 10.0.0.1"),
        );

        let ip = extract_client_ip(&headers, None);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_extract_client_ip_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.42"));

        let ip = extract_client_ip(&headers, None);
        assert_eq!(ip, "198.51.100.42");
    }

    #[test]
    fn test_extract_client_ip_prefers_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.1"));
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.42"));

        let ip = extract_client_ip(&headers, None);
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_extract_client_ip_invalid_falls_through() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("not-an-ip"));
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.42"));

        let ip = extract_client_ip(&headers, None);
        assert_eq!(ip, "198.51.100.42");
    }

    #[test]
    fn test_rate_limit_config_login() {
        let config = RateLimitConfig::login();
        assert_eq!(config.max_requests, 5);
        assert_eq!(config.window_secs, 60);
        assert_eq!(config.key_prefix, "auth:login");
    }

    #[test]
    fn test_rate_limit_config_register() {
        let config = RateLimitConfig::register();
        assert_eq!(config.max_requests, 3);
        assert_eq!(config.window_secs, 3600);
        assert_eq!(config.key_prefix, "auth:register");
    }

    // In-memory rate limiter tests

    #[tokio::test]
    async fn test_in_memory_rate_limiter_allows_within_limit() {
        let limiter = InMemoryRateLimiter::new();
        let config = RateLimitConfig::new("test", 3, 60);

        // First 3 requests should be allowed
        let result = limiter.check("client1", &config).await;
        assert_eq!(result, Ok(2)); // 3 - 1 - 1 = 1? No wait, it returns remaining after adding

        let result = limiter.check("client1", &config).await;
        assert_eq!(result, Ok(1));

        let result = limiter.check("client1", &config).await;
        assert_eq!(result, Ok(0));
    }

    #[tokio::test]
    async fn test_in_memory_rate_limiter_blocks_over_limit() {
        let limiter = InMemoryRateLimiter::new();
        let config = RateLimitConfig::new("test", 2, 60);

        // Use up the limit
        let _ = limiter.check("client1", &config).await;
        let _ = limiter.check("client1", &config).await;

        // Third request should be blocked
        let result = limiter.check("client1", &config).await;
        assert!(result.is_err());

        // Should return a retry_after value
        if let Err(retry_after) = result {
            assert!(retry_after > 0);
            assert!(retry_after <= 60);
        }
    }

    #[tokio::test]
    async fn test_in_memory_rate_limiter_different_clients() {
        let limiter = InMemoryRateLimiter::new();
        let config = RateLimitConfig::new("test", 1, 60);

        // First client uses their limit
        let result = limiter.check("client1", &config).await;
        assert_eq!(result, Ok(0));

        // Second client should still be allowed
        let result = limiter.check("client2", &config).await;
        assert_eq!(result, Ok(0));

        // First client is still blocked
        let result = limiter.check("client1", &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_rate_limiter_different_prefixes() {
        let limiter = InMemoryRateLimiter::new();
        let login_config = RateLimitConfig::new("login", 1, 60);
        let register_config = RateLimitConfig::new("register", 1, 60);

        // Use up login limit
        let result = limiter.check("client1", &login_config).await;
        assert_eq!(result, Ok(0));

        // Register limit should still be available
        let result = limiter.check("client1", &register_config).await;
        assert_eq!(result, Ok(0));
    }

    #[tokio::test]
    async fn test_in_memory_rate_limiter_entry_count() {
        let limiter = InMemoryRateLimiter::new();
        let config = RateLimitConfig::new("test", 5, 60);

        assert_eq!(limiter.entry_count().await, 0);

        limiter.check("client1", &config).await.unwrap();
        assert_eq!(limiter.entry_count().await, 1);

        limiter.check("client2", &config).await.unwrap();
        assert_eq!(limiter.entry_count().await, 2);

        // Same client doesn't add new entry
        limiter.check("client1", &config).await.unwrap();
        assert_eq!(limiter.entry_count().await, 2);
    }

    #[test]
    fn test_rate_limit_entry_check_and_record() {
        let mut entry = RateLimitEntry::new();
        let window = Duration::from_secs(60);

        // First request should succeed
        let result = entry.check_and_record(3, window);
        assert_eq!(result, Ok(2));

        // Second request should succeed
        let result = entry.check_and_record(3, window);
        assert_eq!(result, Ok(1));

        // Third request should succeed with 0 remaining
        let result = entry.check_and_record(3, window);
        assert_eq!(result, Ok(0));

        // Fourth request should fail
        let result = entry.check_and_record(3, window);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_entry_is_expired() {
        let mut entry = RateLimitEntry::new();
        let window = Duration::from_secs(60);

        // Empty entry is expired
        assert!(entry.is_expired(window));

        // Add a timestamp
        entry.timestamps.push(Instant::now());
        assert!(!entry.is_expired(window));

        // Entry with very old timestamp is expired
        let mut old_entry = RateLimitEntry::new();
        old_entry
            .timestamps
            .push(Instant::now() - Duration::from_secs(120));
        assert!(old_entry.is_expired(window));
    }
}
