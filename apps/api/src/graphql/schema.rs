//! GraphQL schema builder for Resonance
//!
//! This module provides the schema construction for the async-graphql API.

use async_graphql::{EmptySubscription, Schema};
use sqlx::PgPool;

use crate::services::auth::AuthService;

use super::mutation::Mutation;
use super::query::Query;

/// The Resonance GraphQL schema type
pub type ResonanceSchema = Schema<Query, Mutation, EmptySubscription>;

/// Builder for constructing the GraphQL schema with required services
pub struct SchemaBuilder {
    pool: Option<PgPool>,
    auth_service: Option<AuthService>,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            pool: None,
            auth_service: None,
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

    /// Build the schema with all configured services
    ///
    /// # Panics
    /// Panics if required services (pool, auth_service) are not configured
    pub fn build(self) -> ResonanceSchema {
        let pool = self.pool.expect("database pool is required");
        let auth_service = self.auth_service.expect("auth service is required");

        Schema::build(Query::default(), Mutation::default(), EmptySubscription)
            .data(pool)
            .data(auth_service)
            .finish()
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
/// with all required dependencies.
pub fn build_schema(pool: PgPool, auth_service: AuthService) -> ResonanceSchema {
    SchemaBuilder::new()
        .pool(pool)
        .auth_service(auth_service)
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
    }
}
