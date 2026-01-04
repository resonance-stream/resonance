//! GraphQL mutations for Resonance
//!
//! This module contains all mutation resolvers, organized by domain.

mod admin;
mod auth;
mod chat;
mod integrations;
mod playlist;
mod preferences;

pub use admin::AdminMutation;
pub use auth::AuthMutation;
pub use chat::ChatMutation;
pub use integrations::IntegrationsMutation;
pub use playlist::PlaylistMutation;
pub use preferences::PreferencesMutation;

use async_graphql::MergedObject;

/// Root mutation type combining all mutation domains
#[derive(MergedObject, Default)]
pub struct Mutation(
    AuthMutation,
    PlaylistMutation,
    IntegrationsMutation,
    ChatMutation,
    AdminMutation,
    PreferencesMutation,
);
