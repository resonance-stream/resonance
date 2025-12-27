//! Resonance API library
//!
//! This module exposes the core API components for use in integration tests
//! and as a library.

pub mod config;
pub mod error;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod services;

// Re-export commonly used types
pub use error::{ApiError, ApiResult, ErrorResponse};
pub use services::{AuthConfig, AuthService};
