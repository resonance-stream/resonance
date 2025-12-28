//! HTTP route handlers for the Resonance API
//!
//! This module contains all REST endpoint handlers including:
//! - Authentication endpoints
//! - Audio streaming endpoints
//! - Lidarr webhook handlers
//! - Health check and status endpoints

pub mod auth;
pub mod health;
pub mod streaming;

pub use auth::{auth_router, auth_router_with_rate_limiting, AuthState};
pub use health::{health_router, HealthState};
pub use streaming::{streaming_router, StreamingState};

// Future modules:
// pub mod webhooks;
