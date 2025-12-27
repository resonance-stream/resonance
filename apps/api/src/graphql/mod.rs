//! GraphQL schema and resolvers for Resonance
//!
//! This module contains the async-graphql schema including:
//! - Query resolvers for user data, library, search, recommendations
//! - Mutation resolvers for authentication, playlists, settings
//! - Type definitions for all GraphQL objects
//! - Guards for rate limiting and authorization

pub mod guards;
pub mod mutation;
pub mod query;
pub mod schema;
pub mod types;

pub use guards::GraphQLRateLimiter;
pub use schema::{build_schema, build_schema_with_rate_limiting, ResonanceSchema};
