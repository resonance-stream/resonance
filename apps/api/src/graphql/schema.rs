//! GraphQL schema builder for Resonance
//!
//! This module provides the schema construction for the async-graphql API.

use async_graphql::{EmptySubscription, Schema};
use sqlx::PgPool;

use crate::services::auth::AuthService;

use super::guards::GraphQLRateLimiter;
use super::mutation::Mutation;
use super::query::Query;

/// The Resonance GraphQL schema type
pub type ResonanceSchema = Schema<Query, Mutation, EmptySubscription>;

/// Builder for constructing the GraphQL schema with required services
pub struct SchemaBuilder {
    pool: Option<PgPool>,
    auth_service: Option<AuthService>,
    rate_limiter: Option<GraphQLRateLimiter>,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            pool: None,
            auth_service: None,
            rate_limiter: None,
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

    /// Build the schema with all configured services
    ///
    /// # Panics
    /// Panics if required services (pool, auth_service) are not configured
    pub fn build(self) -> ResonanceSchema {
        let pool = self.pool.expect("database pool is required");
        let auth_service = self.auth_service.expect("auth service is required");

        let mut builder = Schema::build(Query::default(), Mutation::default(), EmptySubscription)
            .data(pool)
            .data(auth_service);

        // Add rate limiter if configured
        if let Some(rate_limiter) = self.rate_limiter {
            builder = builder.data(rate_limiter);
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
    }
}
