//! GraphQL schema builder for Resonance
//!
//! This module provides the schema construction for the async-graphql API.

use async_graphql::dataloader::DataLoader;
use async_graphql::{EmptySubscription, Schema};
use sqlx::PgPool;

use crate::repositories::{AlbumRepository, ArtistRepository, PlaylistRepository, TrackRepository};
use crate::services::auth::AuthService;
use crate::services::lastfm::LastfmService;
use crate::services::search::SearchService;
use crate::services::similarity::SimilarityService;

use super::guards::GraphQLRateLimiter;
use super::loaders::{
    AlbumLoader, AlbumsByArtistLoader, ArtistLoader, TrackLoader, TracksByAlbumLoader,
    TracksByArtistLoader,
};
use super::mutation::Mutation;
use super::query::Query;

/// The Resonance GraphQL schema type
pub type ResonanceSchema = Schema<Query, Mutation, EmptySubscription>;

/// Builder for constructing the GraphQL schema with required services
pub struct SchemaBuilder {
    pool: Option<PgPool>,
    auth_service: Option<AuthService>,
    rate_limiter: Option<GraphQLRateLimiter>,
    artist_repository: Option<ArtistRepository>,
    album_repository: Option<AlbumRepository>,
    track_repository: Option<TrackRepository>,
    playlist_repository: Option<PlaylistRepository>,
    // AI/Search services (optional - gracefully degrade if not configured)
    search_service: Option<SearchService>,
    similarity_service: Option<SimilarityService>,
    lastfm_service: Option<LastfmService>,
    ollama_client: Option<resonance_ollama_client::OllamaClient>,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            pool: None,
            auth_service: None,
            rate_limiter: None,
            artist_repository: None,
            album_repository: None,
            track_repository: None,
            playlist_repository: None,
            search_service: None,
            similarity_service: None,
            lastfm_service: None,
            ollama_client: None,
        }
    }

    /// Set the database pool
    pub fn pool(mut self, pool: PgPool) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Set the auth service
    pub fn auth_service(mut self, auth_service: AuthService) -> Self {
        self.auth_service = Some(auth_service);
        self
    }

    /// Set the rate limiter for GraphQL mutations
    ///
    /// If not set, rate limiting guards will be skipped (permissive).
    pub fn rate_limiter(mut self, rate_limiter: GraphQLRateLimiter) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    /// Set the artist repository
    #[allow(dead_code)]
    pub fn artist_repository(mut self, repo: ArtistRepository) -> Self {
        self.artist_repository = Some(repo);
        self
    }

    /// Set the album repository
    #[allow(dead_code)]
    pub fn album_repository(mut self, repo: AlbumRepository) -> Self {
        self.album_repository = Some(repo);
        self
    }

    /// Set the track repository
    #[allow(dead_code)]
    pub fn track_repository(mut self, repo: TrackRepository) -> Self {
        self.track_repository = Some(repo);
        self
    }

    /// Set the playlist repository
    #[allow(dead_code)]
    pub fn playlist_repository(mut self, repo: PlaylistRepository) -> Self {
        self.playlist_repository = Some(repo);
        self
    }

    /// Set the search service for semantic search
    #[allow(dead_code)] // Public API for external callers
    pub fn search_service(mut self, service: SearchService) -> Self {
        self.search_service = Some(service);
        self
    }

    /// Set the similarity service for audio-based track similarity
    #[allow(dead_code)] // Public API for external callers
    pub fn similarity_service(mut self, service: SimilarityService) -> Self {
        self.similarity_service = Some(service);
        self
    }

    /// Set the Last.fm service for similar artists
    #[allow(dead_code)] // Public API for external callers
    pub fn lastfm_service(mut self, service: LastfmService) -> Self {
        self.lastfm_service = Some(service);
        self
    }

    /// Set the Ollama client for embedding generation
    #[allow(dead_code)] // Public API for external callers
    pub fn ollama_client(mut self, client: resonance_ollama_client::OllamaClient) -> Self {
        self.ollama_client = Some(client);
        self
    }

    /// Build the schema with all configured services
    ///
    /// # Panics
    /// Panics if required services (pool, auth_service) are not configured
    pub fn build(self) -> ResonanceSchema {
        let pool = self.pool.expect("database pool is required");
        let auth_service = self.auth_service.expect("auth service is required");

        // Create repositories from pool if not explicitly provided
        let artist_repo = self
            .artist_repository
            .unwrap_or_else(|| ArtistRepository::new(pool.clone()));
        let album_repo = self
            .album_repository
            .unwrap_or_else(|| AlbumRepository::new(pool.clone()));
        let track_repo = self
            .track_repository
            .unwrap_or_else(|| TrackRepository::new(pool.clone()));
        let playlist_repo = self
            .playlist_repository
            .unwrap_or_else(|| PlaylistRepository::new(pool.clone()));

        // Create DataLoaders for batched fetching
        // Note: DataLoaders already batch requests within a single query execution.
        // Per-request caching is handled by async-graphql's request context.
        // Single-entity loaders (return one entity by ID)
        let artist_loader = DataLoader::new(ArtistLoader::new(pool.clone()), tokio::spawn);
        let album_loader = DataLoader::new(AlbumLoader::new(pool.clone()), tokio::spawn);
        let track_loader = DataLoader::new(TrackLoader::new(pool.clone()), tokio::spawn);

        // Collection loaders (return Vec of related entities by parent ID)
        let albums_by_artist_loader =
            DataLoader::new(AlbumsByArtistLoader::new(pool.clone()), tokio::spawn);
        let tracks_by_album_loader =
            DataLoader::new(TracksByAlbumLoader::new(pool.clone()), tokio::spawn);
        let tracks_by_artist_loader =
            DataLoader::new(TracksByArtistLoader::new(pool.clone()), tokio::spawn);

        let mut builder = Schema::build(Query::default(), Mutation::default(), EmptySubscription)
            // Query complexity and depth limits to prevent DoS attacks
            .limit_complexity(500) // Max query complexity score
            .limit_depth(10) // Max nesting depth
            .data(pool)
            .data(auth_service)
            .data(artist_repo)
            .data(album_repo)
            .data(track_repo)
            .data(playlist_repo)
            .data(artist_loader)
            .data(album_loader)
            .data(track_loader)
            .data(albums_by_artist_loader)
            .data(tracks_by_album_loader)
            .data(tracks_by_artist_loader);

        // Add rate limiter if configured
        if let Some(rate_limiter) = self.rate_limiter {
            builder = builder.data(rate_limiter);
        }

        // Add AI/Search services if configured (optional - queries gracefully degrade)
        if let Some(search_service) = self.search_service {
            builder = builder.data(search_service);
        }
        if let Some(similarity_service) = self.similarity_service {
            builder = builder.data(similarity_service);
        }
        if let Some(lastfm_service) = self.lastfm_service {
            builder = builder.data(lastfm_service);
        }
        if let Some(ollama_client) = self.ollama_client {
            builder = builder.data(ollama_client);
        }

        builder.finish()
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new GraphQL schema with the provided services
///
/// This is a convenience function for quickly creating a schema
/// with all required dependencies. Rate limiting is not enabled.
/// Use `build_schema_with_rate_limiting` for rate-limited schemas.
#[allow(dead_code)] // Public API for simple use cases and tests
pub fn build_schema(pool: PgPool, auth_service: AuthService) -> ResonanceSchema {
    SchemaBuilder::new()
        .pool(pool)
        .auth_service(auth_service)
        .build()
}

/// Create a new GraphQL schema with rate limiting enabled
///
/// This adds the GraphQL rate limiter to the schema context,
/// enabling rate limit guards on authentication mutations.
#[allow(dead_code)] // Public API for simple use cases and tests
pub fn build_schema_with_rate_limiting(
    pool: PgPool,
    auth_service: AuthService,
    rate_limiter: GraphQLRateLimiter,
) -> ResonanceSchema {
    SchemaBuilder::new()
        .pool(pool)
        .auth_service(auth_service)
        .rate_limiter(rate_limiter)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests for the schema would require a database connection
    // and are better placed in the integration test suite.

    #[test]
    fn test_schema_builder_default() {
        let builder = SchemaBuilder::default();
        assert!(builder.pool.is_none());
        assert!(builder.auth_service.is_none());
        assert!(builder.rate_limiter.is_none());
        assert!(builder.artist_repository.is_none());
        assert!(builder.album_repository.is_none());
        assert!(builder.track_repository.is_none());
        assert!(builder.playlist_repository.is_none());
        assert!(builder.search_service.is_none());
        assert!(builder.similarity_service.is_none());
        assert!(builder.lastfm_service.is_none());
        assert!(builder.ollama_client.is_none());
    }
}
