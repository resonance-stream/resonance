//! GraphQL mutations for Resonance
//!
//! This module contains all mutation resolvers, organized by domain.

mod auth;

pub use auth::AuthMutation;

use async_graphql::MergedObject;

/// Root mutation type combining all mutation domains
#[derive(MergedObject, Default)]
pub struct Mutation(AuthMutation);
