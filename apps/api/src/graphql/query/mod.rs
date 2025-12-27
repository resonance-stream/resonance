//! GraphQL queries for Resonance
//!
//! This module contains all query resolvers, organized by domain.

mod user;

pub use user::UserQuery;

use async_graphql::MergedObject;

/// Root query type combining all query domains
#[derive(MergedObject, Default)]
pub struct Query(UserQuery);
