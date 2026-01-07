//! Business logic services for Resonance
//!
//! This module contains the core business logic including:
//! - Authentication and session management
//! - Audio processing and transcoding
//! - Library management
//! - Recommendation engine
//! - External service integrations
//! - Health checks
//! - AI chat assistant
//! - Encryption for sensitive data
//! - Configuration loading with DB -> Env -> Defaults priority
//! - Meilisearch full-text search

pub mod auth;
pub mod chat;
pub mod config;
pub mod encryption;
pub mod health;
pub mod lastfm;
pub mod listenbrainz;
pub mod meilisearch;
pub mod playlist;
pub mod search;
pub mod similarity;
pub mod transcoder;

pub use auth::AuthService;
#[allow(unused_imports)] // Used for runtime config loading
pub use config::{ConfigError, ConfigResult, ConfigService, LastFmConfig, MusicLibraryConfig};
pub use encryption::{EncryptionError, EncryptionService};
pub use health::HealthService;
#[allow(unused_imports)] // Will be used once integrated into mutations
pub use playlist::PlaylistService;
pub use transcoder::{TranscodeFormat, TranscodeOptions, TranscoderService};

// AI/Search services - re-exported for schema builder and external use
// These are used via the schema builder pattern, not direct crate imports
#[allow(unused_imports)]
pub use chat::{ChatAction, ChatError, ChatService, UserContext};
#[allow(unused_imports)]
pub use lastfm::LastfmService;
#[allow(unused_imports)]
pub use listenbrainz::{ListenBrainzService, ScrobbleTrack};
#[allow(unused_imports)]
pub use search::SearchService;
#[allow(unused_imports)]
pub use similarity::{
    CachedSimilarityService, SimilarityCacheConfig, SimilarityConfig, SimilarityConfigError,
    SimilarityService,
};

// AuthConfig is available for custom configuration
#[allow(unused_imports)]
pub use auth::AuthConfig;

// Meilisearch full-text search service
#[allow(unused_imports)]
pub use meilisearch::{
    AlbumDocument, AlbumSearchHit, ArtistDocument, ArtistSearchHit, IndexStats,
    MeilisearchHealthStatus, MeilisearchService, TrackDocument, TrackSearchHit,
    UnifiedSearchResults,
};

// Future modules:
// pub mod audio;
// pub mod library;
// pub mod recommendations;
// pub mod lidarr;
// pub mod ollama;
