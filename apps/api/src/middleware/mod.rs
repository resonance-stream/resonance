//! Middleware components for Resonance API
//!
//! This module provides authentication extractors for Axum handlers:
//! - `AuthUser`: Requires valid authentication, returns 401 if missing/invalid
//! - `MaybeAuthUser`: Optional authentication, returns None if not authenticated
//! - `AdminUser`: Requires admin role, returns 403 if not admin
//!
//! And rate limiting middleware:
//! - `login_rate_limit`: Limits login attempts (5 per minute per IP)
//! - `register_rate_limit`: Limits registration attempts (3 per hour per IP)

pub mod auth;
pub mod rate_limit;

pub use auth::{AdminUser, AuthUser, MaybeAuthUser};
pub use rate_limit::{
    extract_client_ip, extract_client_ip_trusted, login_rate_limit, register_rate_limit,
    AuthRateLimitState, RateLimitConfig, RateLimiter, TrustedProxies,
};
