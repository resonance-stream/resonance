//! Playlist queries for Resonance GraphQL API
//!
//! This module provides queries for user playlists:
//! - playlist: Get a specific playlist by ID
//! - myPlaylists: Get playlists owned by the authenticated user
//! - publicPlaylists: Browse public playlists

use async_graphql::{Context, Object, Result, ID};
use uuid::Uuid;

use crate::graphql::pagination::{clamp_limit, clamp_offset, MAX_LIMIT};
use crate::graphql::types::Playlist;
use crate::models::user::Claims;
use crate::repositories::PlaylistRepository;

/// Playlist-related queries
#[derive(Default)]
pub struct PlaylistQuery;

#[Object]
impl PlaylistQuery {
    /// Get a playlist by ID
    ///
    /// Returns the playlist if the user has access (owner, collaborator, or public).
    async fn playlist(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Playlist>> {
        let repo = ctx.data::<PlaylistRepository>()?;
        let uuid = Uuid::parse_str(&id)?;

        // Get user ID if authenticated (for access check)
        let user_id = ctx.data_opt::<Claims>().map(|c| c.sub);

        // Check if playlist exists and user has access
        if let Some(playlist) = repo.find_by_id(uuid).await? {
            // Simplified access check using match for clarity
            let has_access = match user_id {
                Some(uid) => {
                    // Authenticated user: check if owner or has access
                    playlist.user_id == uid || repo.can_access(uuid, uid).await?
                }
                None => {
                    // Anonymous user: only public playlists
                    playlist.is_public
                }
            };

            if has_access {
                return Ok(Some(Playlist::from(playlist)));
            }
        }

        Ok(None)
    }

    /// Get playlists owned by the authenticated user
    ///
    /// Requires authentication. Returns all playlists where the user is the owner.
    async fn my_playlists(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Playlist>> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("authentication required"))?;

        let repo = ctx.data::<PlaylistRepository>()?;
        let playlists = repo
            .find_by_user(
                claims.sub,
                clamp_limit(limit, MAX_LIMIT),
                clamp_offset(offset),
            )
            .await?;

        Ok(playlists.into_iter().map(Playlist::from).collect())
    }

    /// Browse public playlists
    ///
    /// Returns publicly visible playlists from all users.
    async fn public_playlists(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = 0)] offset: i32,
    ) -> Result<Vec<Playlist>> {
        let repo = ctx.data::<PlaylistRepository>()?;
        let playlists = repo
            .find_public(clamp_limit(limit, MAX_LIMIT), clamp_offset(offset))
            .await?;
        Ok(playlists.into_iter().map(Playlist::from).collect())
    }
}
