//! Shared test utilities for Resonance workspace
//!
//! This crate provides mock implementations of external services for testing
//! without network dependencies. These mocks can be used across the worker
//! and API test suites.
//!
//! # Mock Services
//!
//! - [`MockOllamaServer`] - Mock Ollama LLM server for embedding and generation tests
//! - [`MockLidarrServer`] - Mock Lidarr server for library sync tests
//! - [`MockRedisStore`] - In-memory Redis mock for caching tests
//!
//! # Example
//!
//! ```rust,ignore
//! use resonance_test_utils::{MockOllamaServer, MockLidarrServer, MockRedisStore};
//!
//! #[tokio::test]
//! async fn test_with_mocks() {
//!     let ollama = MockOllamaServer::start().await;
//!     ollama.mock_embeddings_success().await;
//!
//!     // Use ollama.url() to configure your client
//! }
//! ```

mod ollama;
mod lidarr;
mod redis;

pub use ollama::MockOllamaServer;
pub use lidarr::{
    MockLidarrServer,
    LidarrArtistFixture,
    LidarrAlbumFixture,
    LidarrAlbumStatisticsFixture,
    LidarrImageFixture,
};
pub use redis::MockRedisStore;
