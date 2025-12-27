//! Middleware components for Resonance API
//!
//! This module provides authentication extractors for Axum handlers:
//! - `AuthUser`: Requires valid authentication, returns 401 if missing/invalid
//! - `MaybeAuthUser`: Optional authentication, returns None if not authenticated
//! - `AdminUser`: Requires admin role, returns 403 if not admin

pub mod auth;

pub use auth::{AdminUser, AuthUser, MaybeAuthUser};
