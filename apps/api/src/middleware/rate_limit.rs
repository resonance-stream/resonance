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
use uuid::Uuid;

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

// =============================================================================
// Trusted Proxy Configuration
// =============================================================================

/// Configuration for trusted proxy IPs
///
/// When running behind a reverse proxy (nginx, Cloudflare, etc.), the X-Forwarded-For
/// and X-Real-IP headers should only be trusted if the direct connection comes from
/// a known proxy IP. This prevents attackers from spoofing client IPs to bypass
/// rate limiting.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    /// List of trusted proxy IP addresses
    ips: Vec<IpAddr>,
    /// Trust all private/localhost IPs (useful for Docker deployments)
    trust_private: bool,
}

#[allow(dead_code)] // Infrastructure for configurable proxy trust
impl TrustedProxies {
    /// Create a new empty TrustedProxies config (trusts no forwarding headers)
    pub fn none() -> Self {
        Self {
            ips: Vec::new(),
            trust_private: false,
        }
    }

    /// Create a TrustedProxies config that trusts private/localhost IPs
    ///
    /// This is suitable for typical Docker Compose deployments where the
    /// reverse proxy runs on the same network.
    pub fn trust_private() -> Self {
        Self {
            ips: Vec::new(),
            trust_private: true,
        }
    }

    /// Create from a list of trusted IP addresses
    pub fn from_ips(ips: Vec<IpAddr>) -> Self {
        Self {
            ips,
            trust_private: false,
        }
    }

    /// Add an IP address to the trusted list
    pub fn add_ip(&mut self, ip: IpAddr) {
        self.ips.push(ip);
    }

    /// Parse trusted proxies from a comma-separated string (e.g., from env var)
    ///
    /// Format: "192.168.1.1,10.0.0.1" or "private" to trust all private IPs
    pub fn from_env(value: &str) -> Self {
        let value = value.trim();

        if value.eq_ignore_ascii_case("private") {
            return Self::trust_private();
        }

        if value.is_empty() || value.eq_ignore_ascii_case("none") {
            return Self::none();
        }

        let ips: Vec<IpAddr> = value
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        Self {
            ips,
            trust_private: false,
        }
    }

    /// Check if an IP is a trusted proxy
    pub fn is_trusted(&self, ip: &IpAddr) -> bool {
        // Check explicit list
        if self.ips.contains(ip) {
            return true;
        }

        // Check private/localhost if enabled
        if self.trust_private {
            return is_private_or_localhost(ip);
        }

        false
    }
}

/// Check if an IP is private (RFC 1918) or localhost
fn is_private_or_localhost(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_loopback()           // 127.0.0.0/8
                || ipv4.is_private()     // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                || ipv4.is_link_local() // 169.254.0.0/16
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback() // ::1
        }
    }
}

// =============================================================================
// Rate Limit Configuration
// =============================================================================

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

    /// Rate limit for token refresh: 10 attempts per 60 seconds per IP
    ///
    /// Token refresh is more lenient than login since it's automated by clients
    /// and requires a valid refresh token. However, we still limit it to prevent
    /// abuse of the token rotation mechanism.
    pub fn refresh_token() -> Self {
        Self::new("auth:refresh", 10, 60)
    }

    /// Rate limit for password change: 5 attempts per 15 minutes per IP
    ///
    /// Password changes are sensitive operations. While they require the current
    /// password, we still limit attempts to prevent brute-force attacks that
    /// could guess the current password.
    pub fn change_password() -> Self {
        Self::new("auth:change_password", 5, 900) // 15 minutes
    }
}

/// Entry for tracking request timestamps in the in-memory rate limiter
#[derive(Debug, Clone)]
struct RateLimitEntry {
    /// Timestamps of requests within the current window (relative to creation)
    timestamps: Vec<Instant>,
    /// When this entry expires (for deterministic cleanup)
    expires_at: Instant,
}

impl RateLimitEntry {
    fn new(window: Duration) -> Self {
        Self {
            timestamps: Vec::new(),
            expires_at: Instant::now() + window,
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
            // Extend expiry to cover this new request
            self.expires_at = now + window;
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

    /// Check if this entry has expired (can be safely cleaned up)
    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
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
        self.maybe_cleanup().await;

        let mut entries = self.entries.write().await;
        let entry = entries
            .entry(full_key.clone())
            .or_insert_with(|| RateLimitEntry::new(window));

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
    async fn maybe_cleanup(&self) {
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

        // Each entry tracks its own expires_at, so cleanup is deterministic
        entries.retain(|_, entry| !entry.is_expired());

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
    #[allow(dead_code)] // Available for tests that need custom fallback behavior
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

        // Get current time from Redis server to prevent clock skew
        // TIME command returns [seconds, microseconds] as strings
        let now: u64 = match redis::cmd("TIME")
            .query_async::<_, Vec<String>>(&mut conn)
            .await
        {
            Ok(time) if !time.is_empty() => time[0].parse().unwrap_or_else(|_| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }),
            _ => {
                // Fallback to system time if Redis TIME fails
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }
        };

        // Sliding window algorithm using Redis sorted sets
        // Each request is stored with its timestamp as the score
        let _window_start = now - config.window_secs;

        // Generate a UUID for unique entry identification
        let nonce = Uuid::new_v4().to_string();

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
            local nonce = ARGV[4]
            local window_start = now - window

            -- Remove old entries
            redis.call('ZREMRANGEBYSCORE', key, 0, window_start)

            -- Count current entries
            local current = redis.call('ZCARD', key)

            if current < max_requests then
                -- Add new request with current timestamp and UUID nonce for uniqueness
                redis.call('ZADD', key, now, now .. ':' .. nonce)
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
                    -- Clamp to minimum of 1 second to prevent zero-second retries
                    if retry_after < 1 then retry_after = 1 end
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
            .arg(&nonce)
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
            // Clamp retry_after to minimum of 1 second (defense in depth)
            let retry_after = ((-result) as u64).max(1);
            debug!(key = %full_key, retry_after = retry_after, "Rate limit exceeded");
            Err(retry_after)
        }
    }
}

/// Extract client IP from request headers or connection info
///
/// This function trusts X-Forwarded-For and X-Real-IP headers by default for
/// backwards compatibility. For production deployments behind a reverse proxy,
/// use `extract_client_ip_trusted` with explicit proxy configuration.
///
/// # Security Note
/// If this service is exposed directly to the internet (not behind a reverse
/// proxy), attackers can spoof the X-Forwarded-For header to bypass rate limiting.
/// Use `extract_client_ip_trusted` with `TrustedProxies::none()` to disable
/// header trust, or configure specific trusted proxy IPs.
pub fn extract_client_ip(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<std::net::SocketAddr>>,
) -> String {
    // Default: trust private IPs (typical Docker Compose setup)
    // Returns "unknown" if no IP can be determined (for backwards compatibility)
    extract_client_ip_option(headers, connect_info).unwrap_or_else(|| "unknown".to_string())
}

/// Extract client IP, returning None if it cannot be determined
///
/// Use this function when you need to distinguish between a known IP and
/// no IP being available (e.g., for conditional rate limiting).
pub fn extract_client_ip_option(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<std::net::SocketAddr>>,
) -> Option<String> {
    // Default: trust private IPs (typical Docker Compose setup)
    extract_client_ip_trusted_option(headers, connect_info, &TrustedProxies::trust_private())
}

/// Extract client IP with explicit trusted proxy configuration
///
/// Only trusts X-Forwarded-For and X-Real-IP headers when the direct connection
/// comes from a trusted proxy IP. This prevents IP spoofing attacks when the
/// service is exposed directly to the internet.
///
/// When `trust_private` is enabled, private IPs from forwarding headers are also
/// accepted, allowing proper tracking of internal service clients.
///
/// # Arguments
/// * `headers` - Request headers
/// * `connect_info` - Connection info containing the direct peer IP
/// * `trusted_proxies` - Configuration for which proxy IPs to trust
///
/// # Example
/// ```ignore
/// // Trust only specific proxy IPs
/// let proxies = TrustedProxies::from_ips(vec!["10.0.0.1".parse().unwrap()]);
/// let ip = extract_client_ip_trusted(&headers, connect_info.as_ref(), &proxies);
///
/// // Trust all private IPs (Docker networks)
/// let proxies = TrustedProxies::trust_private();
/// let ip = extract_client_ip_trusted(&headers, connect_info.as_ref(), &proxies);
///
/// // Trust no proxies (direct connections only)
/// let proxies = TrustedProxies::none();
/// let ip = extract_client_ip_trusted(&headers, connect_info.as_ref(), &proxies);
/// ```
#[allow(dead_code)] // Available for future use with trusted proxy configuration
pub fn extract_client_ip_trusted(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<std::net::SocketAddr>>,
    trusted_proxies: &TrustedProxies,
) -> String {
    // Get the direct peer IP first
    let direct_ip = connect_info.map(|ci| ci.0.ip());

    // Only trust forwarding headers if the direct connection is from a trusted proxy
    let should_trust_headers = direct_ip
        .map(|ip| trusted_proxies.is_trusted(&ip))
        .unwrap_or(false);

    if should_trust_headers {
        // When trust_private is enabled, also accept private IPs from forwarding headers
        // (allows tracking internal service clients behind trusted proxies)
        let accept_private = trusted_proxies.trust_private;

        // Try X-Forwarded-For first (for proxied requests)
        if let Some(forwarded) = headers.get("x-forwarded-for") {
            if let Ok(value) = forwarded.to_str() {
                // X-Forwarded-For can contain multiple IPs; pick the first valid one
                // When not trusting private, skip private addresses to prevent spoofing
                for ip in value.split(',').map(|s| s.trim()) {
                    if let Ok(parsed) = ip.parse::<IpAddr>() {
                        if accept_private || !is_private_or_localhost(&parsed) {
                            return ip.to_string();
                        }
                    }
                }
            }
        }

        // Try X-Real-IP (common with nginx)
        if let Some(real_ip) = headers.get("x-real-ip") {
            if let Ok(value) = real_ip.to_str() {
                let ip = value.trim();
                if let Ok(parsed) = ip.parse::<IpAddr>() {
                    if accept_private || !is_private_or_localhost(&parsed) {
                        return ip.to_string();
                    }
                }
            }
        }
    } else if headers.contains_key("x-forwarded-for") || headers.contains_key("x-real-ip") {
        // Log when we're ignoring forwarding headers from untrusted source
        debug!(
            direct_ip = ?direct_ip,
            "Ignoring X-Forwarded-For/X-Real-IP from untrusted proxy"
        );
    }

    // Fall back to connection info
    if let Some(ip) = direct_ip {
        return ip.to_string();
    }

    // Last resort - use a placeholder (shouldn't happen in production)
    warn!("Could not determine client IP for rate limiting");
    "unknown".to_string()
}

/// Extract client IP with explicit trusted proxy configuration, returning None if unknown
///
/// This variant returns None instead of "unknown" when the client IP cannot be
/// determined, allowing callers to decide how to handle this case (e.g., skip
/// rate limiting rather than rate limiting all unknown IPs together).
pub fn extract_client_ip_trusted_option(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<std::net::SocketAddr>>,
    trusted_proxies: &TrustedProxies,
) -> Option<String> {
    // Get the direct peer IP first
    let direct_ip = connect_info.map(|ci| ci.0.ip());

    // Only trust forwarding headers if the direct connection is from a trusted proxy
    let should_trust_headers = direct_ip
        .map(|ip| trusted_proxies.is_trusted(&ip))
        .unwrap_or(false);

    if should_trust_headers {
        // When trust_private is enabled, also accept private IPs from forwarding headers
        // (allows tracking internal service clients behind trusted proxies)
        let accept_private = trusted_proxies.trust_private;

        // Try X-Forwarded-For first (for proxied requests)
        if let Some(forwarded) = headers.get("x-forwarded-for") {
            if let Ok(value) = forwarded.to_str() {
                // X-Forwarded-For can contain multiple IPs; pick the first valid one
                // When not trusting private, skip private addresses to prevent spoofing
                for ip in value.split(',').map(|s| s.trim()) {
                    if let Ok(parsed) = ip.parse::<IpAddr>() {
                        if accept_private || !is_private_or_localhost(&parsed) {
                            return Some(ip.to_string());
                        }
                    }
                }
            }
        }

        // Try X-Real-IP (common with nginx)
        if let Some(real_ip) = headers.get("x-real-ip") {
            if let Ok(value) = real_ip.to_str() {
                let ip = value.trim();
                if let Ok(parsed) = ip.parse::<IpAddr>() {
                    if accept_private || !is_private_or_localhost(&parsed) {
                        return Some(ip.to_string());
                    }
                }
            }
        }
    } else if headers.contains_key("x-forwarded-for") || headers.contains_key("x-real-ip") {
        // Log when we're ignoring forwarding headers from untrusted source
        debug!(
            direct_ip = ?direct_ip,
            "Ignoring X-Forwarded-For/X-Real-IP from untrusted proxy"
        );
    }

    // Fall back to connection info
    direct_ip.map(|ip| ip.to_string())
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
    #[allow(dead_code)] // Available for custom rate limit configuration
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
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Get client IP - if we can't determine it, skip rate limiting rather than
    // putting all unknown clients in a shared bucket
    let client_ip = match extract_client_ip_option(&headers, Some(&ConnectInfo(addr))) {
        Some(ip) => ip,
        None => {
            warn!("Could not determine client IP for login rate limiting, skipping rate limit");
            return next.run(request).await;
        }
    };

    match state.limiter.check(&client_ip, &state.login_config).await {
        Ok(remaining) => {
            let mut response = next.run(request).await;
            // Add rate limit headers to response (handle potential invalid values gracefully)
            if let Ok(v) =
                axum::http::HeaderValue::from_str(&state.login_config.max_requests.to_string())
            {
                response.headers_mut().insert("X-RateLimit-Limit", v);
            }
            if let Ok(v) = axum::http::HeaderValue::from_str(&remaining.to_string()) {
                response.headers_mut().insert("X-RateLimit-Remaining", v);
            }
            // Use Unix timestamp for reset time (seconds since epoch)
            let reset_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_add(state.login_config.window_secs);
            if let Ok(v) = axum::http::HeaderValue::from_str(&reset_at.to_string()) {
                response.headers_mut().insert("X-RateLimit-Reset", v);
            }
            response
        }
        Err(retry_after) => {
            warn!(
                ip = %client_ip,
                retry_after = retry_after,
                "Login rate limit exceeded"
            );
            // Build rate-limited response with comprehensive headers
            let mut response = ApiError::RateLimited { retry_after }.into_response();
            // Add rate limit headers to help clients understand limits
            if let Ok(v) =
                axum::http::HeaderValue::from_str(&state.login_config.max_requests.to_string())
            {
                response.headers_mut().insert("X-RateLimit-Limit", v);
            }
            if let Ok(v) = axum::http::HeaderValue::from_str("0") {
                response.headers_mut().insert("X-RateLimit-Remaining", v);
            }
            // Add standard Retry-After header (RFC 6585)
            if let Ok(v) = axum::http::HeaderValue::from_str(&retry_after.to_string()) {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, v);
            }
            let reset_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_add(retry_after);
            if let Ok(v) = axum::http::HeaderValue::from_str(&reset_at.to_string()) {
                response.headers_mut().insert("X-RateLimit-Reset", v);
            }
            response
        }
    }
}

/// Middleware for rate limiting registration requests
pub async fn register_rate_limit(
    State(state): State<AuthRateLimitState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Get client IP - if we can't determine it, skip rate limiting rather than
    // putting all unknown clients in a shared bucket
    let client_ip = match extract_client_ip_option(&headers, Some(&ConnectInfo(addr))) {
        Some(ip) => ip,
        None => {
            warn!(
                "Could not determine client IP for registration rate limiting, skipping rate limit"
            );
            return next.run(request).await;
        }
    };

    match state
        .limiter
        .check(&client_ip, &state.register_config)
        .await
    {
        Ok(remaining) => {
            let mut response = next.run(request).await;
            // Add rate limit headers to response (handle potential invalid values gracefully)
            if let Ok(v) =
                axum::http::HeaderValue::from_str(&state.register_config.max_requests.to_string())
            {
                response.headers_mut().insert("X-RateLimit-Limit", v);
            }
            if let Ok(v) = axum::http::HeaderValue::from_str(&remaining.to_string()) {
                response.headers_mut().insert("X-RateLimit-Remaining", v);
            }
            // Use Unix timestamp for reset time (seconds since epoch)
            let reset_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_add(state.register_config.window_secs);
            if let Ok(v) = axum::http::HeaderValue::from_str(&reset_at.to_string()) {
                response.headers_mut().insert("X-RateLimit-Reset", v);
            }
            response
        }
        Err(retry_after) => {
            warn!(
                ip = %client_ip,
                retry_after = retry_after,
                "Registration rate limit exceeded"
            );
            // Build rate-limited response with comprehensive headers
            let mut response = ApiError::RateLimited { retry_after }.into_response();
            // Add rate limit headers to help clients understand limits
            if let Ok(v) =
                axum::http::HeaderValue::from_str(&state.register_config.max_requests.to_string())
            {
                response.headers_mut().insert("X-RateLimit-Limit", v);
            }
            if let Ok(v) = axum::http::HeaderValue::from_str("0") {
                response.headers_mut().insert("X-RateLimit-Remaining", v);
            }
            // Add standard Retry-After header (RFC 6585)
            if let Ok(v) = axum::http::HeaderValue::from_str(&retry_after.to_string()) {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, v);
            }
            let reset_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_add(retry_after);
            if let Ok(v) = axum::http::HeaderValue::from_str(&reset_at.to_string()) {
                response.headers_mut().insert("X-RateLimit-Reset", v);
            }
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::ConnectInfo;
    use axum::http::HeaderValue;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    /// Helper to create a ConnectInfo from localhost (trusted proxy)
    fn localhost_connect_info() -> ConnectInfo<SocketAddr> {
        ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            12345,
        ))
    }

    #[test]
    fn test_extract_client_ip_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.1, 10.0.0.1"),
        );

        // With trusted proxy (localhost), headers are trusted
        let connect_info = localhost_connect_info();
        let ip = extract_client_ip(&headers, Some(&connect_info));
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_extract_client_ip_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.42"));

        let connect_info = localhost_connect_info();
        let ip = extract_client_ip(&headers, Some(&connect_info));
        assert_eq!(ip, "198.51.100.42");
    }

    #[test]
    fn test_extract_client_ip_prefers_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.1"));
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.42"));

        let connect_info = localhost_connect_info();
        let ip = extract_client_ip(&headers, Some(&connect_info));
        assert_eq!(ip, "203.0.113.1");
    }

    #[test]
    fn test_extract_client_ip_invalid_falls_through() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("not-an-ip"));
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.42"));

        let connect_info = localhost_connect_info();
        let ip = extract_client_ip(&headers, Some(&connect_info));
        assert_eq!(ip, "198.51.100.42");
    }

    #[test]
    fn test_extract_client_ip_ignores_headers_from_untrusted() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.1"));

        // With untrusted proxy (public IP), headers are ignored
        let connect_info = ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            12345,
        ));
        let ip = extract_client_ip(&headers, Some(&connect_info));
        // Falls back to the direct connection IP
        assert_eq!(ip, "8.8.8.8");
    }

    #[test]
    fn test_extract_client_ip_no_connect_info() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.1"));

        // Without connect_info, headers are not trusted
        let ip = extract_client_ip(&headers, None);
        assert_eq!(ip, "unknown");
    }

    #[test]
    fn test_extract_client_ip_option_returns_none() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.1"));

        // Without connect_info, returns None instead of "unknown"
        let ip = extract_client_ip_option(&headers, None);
        assert_eq!(ip, None);
    }

    #[test]
    fn test_extract_client_ip_option_returns_some() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.1"));

        // With trusted proxy, returns Some(IP)
        let connect_info = localhost_connect_info();
        let ip = extract_client_ip_option(&headers, Some(&connect_info));
        assert_eq!(ip, Some("203.0.113.1".to_string()));
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

    #[test]
    fn test_rate_limit_config_change_password() {
        let config = RateLimitConfig::change_password();
        assert_eq!(config.max_requests, 5);
        assert_eq!(config.window_secs, 900); // 15 minutes
        assert_eq!(config.key_prefix, "auth:change_password");
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
        let window = Duration::from_secs(60);
        let mut entry = RateLimitEntry::new(window);

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
        let window = Duration::from_secs(60);

        // New entry is not expired (expires_at is in the future)
        let entry = RateLimitEntry::new(window);
        assert!(!entry.is_expired());

        // Entry with expires_at in the past is expired
        let mut old_entry = RateLimitEntry::new(window);
        old_entry.expires_at = Instant::now() - Duration::from_secs(1);
        assert!(old_entry.is_expired());
    }
}
