//! Rate limiting middleware for Resonance API
//!
//! Provides Redis-based rate limiting to prevent brute-force attacks on
//! authentication endpoints. Uses a sliding window algorithm for accurate
//! rate limiting across distributed instances.

use std::net::IpAddr;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
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

/// State for rate limiting middleware
#[derive(Clone)]
pub struct RateLimiter {
    redis: Arc<redis::Client>,
}

impl RateLimiter {
    /// Create a new rate limiter with a Redis client
    pub fn new(redis: redis::Client) -> Self {
        Self {
            redis: Arc::new(redis),
        }
    }

    /// Check if a request should be rate limited
    ///
    /// Returns Ok(remaining) if allowed, Err(retry_after) if rate limited
    pub async fn check(&self, key: &str, config: &RateLimitConfig) -> Result<u32, u64> {
        let full_key = format!("ratelimit:{}:{}", config.key_prefix, key);

        let mut conn = match self.redis.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                // If Redis is unavailable, log a warning but allow the request
                // This prevents Redis outages from blocking all auth attempts
                warn!(error = %e, "Redis unavailable for rate limiting, allowing request");
                return Ok(config.max_requests);
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
                warn!(error = %e, key = %full_key, "Rate limit check failed, allowing request");
                return Ok(config.max_requests);
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
pub fn extract_client_ip(headers: &HeaderMap, connect_info: Option<&ConnectInfo<std::net::SocketAddr>>) -> String {
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

    match state.limiter.check(&client_ip, &state.register_config).await {
        Ok(remaining) => {
            let mut response = next.run(request).await;
            // Add rate limit headers to response
            response.headers_mut().insert(
                "X-RateLimit-Limit",
                state.register_config.max_requests.to_string().parse().unwrap(),
            );
            response.headers_mut().insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap(),
            );
            response.headers_mut().insert(
                "X-RateLimit-Reset",
                state.register_config.window_secs.to_string().parse().unwrap(),
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
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.1, 10.0.0.1"));

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
}
