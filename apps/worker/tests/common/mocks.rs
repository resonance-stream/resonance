//! Mock services for worker integration tests
//!
//! Re-exports mock implementations from the shared test-utils crate for
//! testing worker jobs without network dependencies.
//!
//! This module provides convenient access to:
//! - [`MockOllamaServer`] - Mock Ollama LLM server
//! - [`MockLidarrServer`] - Mock Lidarr server for library sync
//! - [`MockRedisStore`] - In-memory Redis mock for caching
//!
//! # Migration Note
//!
//! The mock implementations have been moved to the shared `resonance-test-utils`
//! crate to enable reuse across both worker and API test suites.

pub use resonance_test_utils::{
    LidarrAlbumFixture, LidarrAlbumStatisticsFixture, LidarrArtistFixture, LidarrImageFixture,
    MockLidarrServer, MockOllamaServer, MockRedisStore,
};
