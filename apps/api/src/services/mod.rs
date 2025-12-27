//! Business logic services for Resonance
//!
//! This module contains the core business logic including:
//! - Authentication and session management
//! - Audio processing and transcoding
//! - Library management
//! - Recommendation engine
//! - External service integrations
//! - Health checks

pub mod auth;
pub mod health;

pub use auth::{AuthConfig, AuthService};
pub use health::HealthService;

// Future modules:
// pub mod audio;
// pub mod library;
// pub mod recommendations;
// pub mod lidarr;
// pub mod ollama;
// pub mod meilisearch;
