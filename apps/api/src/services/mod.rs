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
pub mod lastfm;
pub mod playlist;
pub mod search;
pub mod similarity;
pub mod transcoder;

pub use auth::AuthService;
pub use health::HealthService;
#[allow(unused_imports)] // Will be used once integrated into mutations
pub use playlist::PlaylistService;
pub use transcoder::{TranscodeFormat, TranscodeOptions, TranscoderService};

// AI/Search services - re-exported for schema builder and external use
// These are used via the schema builder pattern, not direct crate imports
#[allow(unused_imports)]
pub use lastfm::LastfmService;
#[allow(unused_imports)]
pub use search::SearchService;
#[allow(unused_imports)]
pub use similarity::SimilarityService;

// AuthConfig is available for custom configuration
#[allow(unused_imports)]
pub use auth::AuthConfig;

// Future modules:
// pub mod audio;
// pub mod library;
// pub mod recommendations;
// pub mod lidarr;
// pub mod ollama;
// pub mod meilisearch;
