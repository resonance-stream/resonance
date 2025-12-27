//! Rate limiting guard for GraphQL mutations
//!
//! This module provides rate limiting for GraphQL authentication mutations
//! (register, login, refreshToken) to prevent brute-force attacks.
//!
//! The guard uses the same Redis-based sliding window algorithm as the REST
//! endpoints for consistency. When Redis is unavailable, it falls back to
//! in-memory rate limiting.

use async_graphql::{Context, ErrorExtensions, Guard};
use tracing::{debug, warn};

use crate::middleware::rate_limit::{RateLimitConfig, RateLimiter};
use crate::models::user::RequestMetadata;

/// Rate limiter wrapper for GraphQL context
///
/// This wraps the `RateLimiter` from the middleware module and provides
/// convenient methods for checking rate limits from within GraphQL resolvers.
#[derive(Clone)]
pub struct GraphQLRateLimiter {
    limiter: RateLimiter,
    login_config: RateLimitConfig,
    register_config: RateLimitConfig,
    refresh_config: RateLimitConfig,
}

impl GraphQLRateLimiter {
    /// Create a new GraphQL rate limiter with a Redis client
    pub fn new(redis_client: redis::Client) -> Self {
        Self {
            limiter: RateLimiter::new(redis_client),
            login_config: RateLimitConfig::login(),
            register_config: RateLimitConfig::register(),
            refresh_config: RateLimitConfig::refresh_token(),
        }
    }

    /// Create a rate limiter with in-memory fallback only (for testing or when Redis unavailable)
    #[cfg(test)]
    #[allow(dead_code)] // Available for tests that need in-memory rate limiting
    pub fn in_memory_only() -> Self {
        // Create a dummy Redis client that will fail to connect
        // This will cause the RateLimiter to use its in-memory fallback
        let dummy_client = redis::Client::open("redis://localhost:0").unwrap();
        Self {
            limiter: RateLimiter::new(dummy_client),
            login_config: RateLimitConfig::login(),
            register_config: RateLimitConfig::register(),
            refresh_config: RateLimitConfig::refresh_token(),
        }
    }

    /// Check login rate limit for a client
    pub async fn check_login(&self, client_ip: &str) -> Result<u32, u64> {
        self.limiter.check(client_ip, &self.login_config).await
    }

    /// Check registration rate limit for a client
    pub async fn check_register(&self, client_ip: &str) -> Result<u32, u64> {
        self.limiter.check(client_ip, &self.register_config).await
    }

    /// Check refresh token rate limit for a client
    pub async fn check_refresh(&self, client_ip: &str) -> Result<u32, u64> {
        self.limiter.check(client_ip, &self.refresh_config).await
    }
}

/// Type of rate limit to apply
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitType {
    /// Login rate limit: 5 attempts per 60 seconds
    Login,
    /// Registration rate limit: 3 attempts per hour
    Register,
    /// Token refresh rate limit: 10 attempts per 60 seconds
    RefreshToken,
}

/// Rate limiting guard for GraphQL mutations
///
/// This guard checks if the client has exceeded their rate limit before
/// allowing the mutation to proceed. The client IP is extracted from the
/// `RequestMetadata` in the GraphQL context.
///
/// # Example
///
/// ```ignore
/// #[Object]
/// impl AuthMutation {
///     #[graphql(guard = "RateLimitGuard::new(RateLimitType::Login)")]
///     async fn login(&self, ctx: &Context<'_>, input: LoginInput) -> Result<AuthPayload> {
///         // ... login implementation
///     }
/// }
/// ```
pub struct RateLimitGuard {
    limit_type: RateLimitType,
}

impl RateLimitGuard {
    /// Create a new rate limit guard
    pub fn new(limit_type: RateLimitType) -> Self {
        Self { limit_type }
    }

    /// Create a login rate limit guard
    #[allow(dead_code)]
    pub fn login() -> Self {
        Self::new(RateLimitType::Login)
    }

    /// Create a registration rate limit guard
    #[allow(dead_code)]
    pub fn register() -> Self {
        Self::new(RateLimitType::Register)
    }

    /// Create a token refresh rate limit guard
    #[allow(dead_code)]
    pub fn refresh_token() -> Self {
        Self::new(RateLimitType::RefreshToken)
    }
}

impl Guard for RateLimitGuard {
    async fn check(&self, ctx: &Context<'_>) -> async_graphql::Result<()> {
        // Get the rate limiter from context
        let rate_limiter = match ctx.data_opt::<GraphQLRateLimiter>() {
            Some(limiter) => limiter,
            None => {
                // Rate limiting is disabled if the limiter is not in context
                debug!("GraphQL rate limiter not configured, skipping rate limit check");
                return Ok(());
            }
        };

        // Get the client IP from request metadata
        let client_ip = ctx
            .data_opt::<RequestMetadata>()
            .and_then(|m| m.ip_address.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Check the rate limit based on the type
        let result = match self.limit_type {
            RateLimitType::Login => rate_limiter.check_login(&client_ip).await,
            RateLimitType::Register => rate_limiter.check_register(&client_ip).await,
            RateLimitType::RefreshToken => rate_limiter.check_refresh(&client_ip).await,
        };

        match result {
            Ok(remaining) => {
                debug!(
                    ip = %client_ip,
                    limit_type = ?self.limit_type,
                    remaining = remaining,
                    "GraphQL rate limit check passed"
                );
                Ok(())
            }
            Err(retry_after) => {
                warn!(
                    ip = %client_ip,
                    limit_type = ?self.limit_type,
                    retry_after = retry_after,
                    "GraphQL rate limit exceeded"
                );

                // Return a GraphQL error with rate limit information
                Err(async_graphql::Error::new(format!(
                    "Rate limit exceeded. Please try again in {} seconds.",
                    retry_after
                ))
                .extend_with(|_, e| {
                    e.set("code", "RATE_LIMITED");
                    e.set("retry_after", retry_after);
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_type_debug() {
        assert_eq!(format!("{:?}", RateLimitType::Login), "Login");
        assert_eq!(format!("{:?}", RateLimitType::Register), "Register");
        assert_eq!(format!("{:?}", RateLimitType::RefreshToken), "RefreshToken");
    }

    #[test]
    fn test_rate_limit_guard_constructors() {
        let login_guard = RateLimitGuard::login();
        assert_eq!(login_guard.limit_type, RateLimitType::Login);

        let register_guard = RateLimitGuard::register();
        assert_eq!(register_guard.limit_type, RateLimitType::Register);

        let refresh_guard = RateLimitGuard::refresh_token();
        assert_eq!(refresh_guard.limit_type, RateLimitType::RefreshToken);
    }
}
