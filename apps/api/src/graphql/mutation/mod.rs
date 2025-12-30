//! GraphQL mutations for Resonance
//!
//! This module contains all mutation resolvers, organized by domain.

mod auth;
mod chat;
mod integrations;
mod playlist;

pub use auth::AuthMutation;
pub use chat::ChatMutation;
pub use integrations::IntegrationsMutation;
pub use playlist::PlaylistMutation;

use async_graphql::MergedObject;

/// Root mutation type combining all mutation domains
#[derive(MergedObject, Default)]
pub struct Mutation(
    AuthMutation,
    PlaylistMutation,
    IntegrationsMutation,
    ChatMutation,
);
