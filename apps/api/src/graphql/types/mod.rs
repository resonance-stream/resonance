//! GraphQL type definitions for Resonance
//!
//! This module contains the GraphQL object types that are exposed
//! through the API, including user and authentication types.

mod user;

pub use user::{AuthPayload, RefreshPayload, User, UserRole};
