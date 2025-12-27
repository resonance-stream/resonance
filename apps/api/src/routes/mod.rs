//! HTTP route handlers for the Resonance API
//!
//! This module contains all REST endpoint handlers including:
//! - Audio streaming endpoints
//! - Lidarr webhook handlers
//! - Health check and status endpoints

pub mod health;

pub use health::{health_router, HealthState};

// Future modules:
// pub mod streaming;
// pub mod webhooks;
// pub mod auth;
