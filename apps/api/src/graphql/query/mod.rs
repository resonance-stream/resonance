//! GraphQL queries for Resonance
//!
//! This module contains all query resolvers, organized by domain.

mod chat;
mod library;
mod playlist;
mod search;
mod user;

pub use chat::ChatQuery;
pub use library::LibraryQuery;
pub use playlist::PlaylistQuery;
pub use search::SearchQuery;
pub use user::UserQuery;

use async_graphql::MergedObject;

/// Root query type combining all query domains
#[derive(MergedObject, Default)]
pub struct Query(UserQuery, LibraryQuery, PlaylistQuery, SearchQuery, ChatQuery);
