//! Playlist mutations for Resonance GraphQL API
//!
//! This module provides mutations for playlist management:
//! - createPlaylist: Create a new playlist (manual or smart)
//! - updatePlaylist: Update an existing playlist
//! - deletePlaylist: Delete a playlist
//! - refreshSmartPlaylist: Re-evaluate smart playlist rules
//! - addTracksToPlaylist: Add tracks to a playlist
//! - removeTracksFromPlaylist: Remove tracks from a playlist

use async_graphql::{Context, InputObject, Object, Result, ID};
use uuid::Uuid;

use crate::error::ApiError;
use crate::graphql::types::{Playlist, PlaylistType};
use crate::models::playlist::{
    PlaylistType as DbPlaylistType, SmartPlaylistRule, SmartPlaylistRules,
};
use crate::models::user::Claims;
use crate::repositories::PlaylistRepository;

// =============================================================================
// Input Validation Limits
// =============================================================================

/// Maximum number of tracks that can be added/removed in a single operation
const MAX_TRACKS_PER_OPERATION: usize = 500;

/// Maximum number of rules in a smart playlist
const MAX_RULES: usize = 50;

/// Maximum length of playlist name
const MAX_NAME_LENGTH: usize = 255;

/// Maximum length of playlist description
const MAX_DESCRIPTION_LENGTH: usize = 2000;

/// Maximum seed tracks for similarity rules
const MAX_SIMILAR_TO_SEEDS: usize = 10;

// =============================================================================
// Error Handling
// =============================================================================

/// Convert API errors to GraphQL errors with appropriate messages
fn to_graphql_error(error: ApiError) -> async_graphql::Error {
    match &error {
        ApiError::NotFound { resource_type, .. } => {
            async_graphql::Error::new(format!("{} not found", resource_type))
        }
        ApiError::Forbidden(msg) => async_graphql::Error::new(msg.clone()),
        ApiError::ValidationError(msg) => async_graphql::Error::new(msg.clone()),
        ApiError::Unauthorized => async_graphql::Error::new("Authentication required"),
        _ => {
            tracing::error!(error = %error, "Playlist mutation error");
            async_graphql::Error::new("An unexpected error occurred")
        }
    }
}

// =============================================================================
// Input Types
// =============================================================================

/// Input type for a single smart playlist rule
#[derive(Debug, Clone, InputObject)]
pub struct SmartPlaylistRuleInput {
    /// Field to match (genre, artist, mood, energy, similar_to, etc.)
    pub field: String,
    /// Operator (equals, contains, greater_than, similar_to, etc.)
    pub operator: String,
    /// Value to match against (can be string, number, array, or object)
    pub value: serde_json::Value,
}

/// Input type for smart playlist rules configuration
#[derive(Debug, Clone, InputObject)]
pub struct SmartPlaylistRulesInput {
    /// Match mode: "all" (AND) or "any" (OR)
    #[graphql(default_with = "String::from(\"all\")")]
    pub match_mode: String,
    /// List of rules to apply
    pub rules: Vec<SmartPlaylistRuleInput>,
    /// Maximum number of tracks (optional)
    pub limit: Option<i32>,
    /// Field to sort by (optional)
    pub sort_by: Option<String>,
    /// Sort direction: "asc" or "desc" (optional)
    pub sort_order: Option<String>,
}

/// Input for playlist type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, async_graphql::Enum)]
pub enum PlaylistTypeInput {
    /// Manually curated playlist
    Manual,
    /// Smart playlist with rules
    Smart,
}

impl From<PlaylistTypeInput> for DbPlaylistType {
    fn from(input: PlaylistTypeInput) -> Self {
        match input {
            PlaylistTypeInput::Manual => DbPlaylistType::Manual,
            PlaylistTypeInput::Smart => DbPlaylistType::Smart,
        }
    }
}

impl From<SmartPlaylistRuleInput> for SmartPlaylistRule {
    fn from(input: SmartPlaylistRuleInput) -> Self {
        Self {
            field: input.field,
            operator: input.operator,
            value: input.value,
        }
    }
}

impl From<SmartPlaylistRulesInput> for SmartPlaylistRules {
    fn from(input: SmartPlaylistRulesInput) -> Self {
        Self {
            // Normalize match_mode to lowercase for consistent comparison
            match_mode: input.match_mode.to_ascii_lowercase(),
            rules: input.rules.into_iter().map(Into::into).collect(),
            limit: input.limit,
            sort_by: input.sort_by,
            sort_order: input.sort_order,
        }
    }
}

/// Input for creating a new playlist
#[derive(Debug, InputObject)]
pub struct CreatePlaylistInput {
    /// Playlist name (required)
    pub name: String,
    /// Playlist description (optional)
    pub description: Option<String>,
    /// Whether the playlist is publicly visible
    #[graphql(default = false)]
    pub is_public: bool,
    /// Type of playlist (Manual or Smart)
    #[graphql(default_with = "PlaylistTypeInput::Manual")]
    pub playlist_type: PlaylistTypeInput,
    /// Smart playlist rules (required if playlist_type is Smart)
    pub smart_rules: Option<SmartPlaylistRulesInput>,
}

/// Input for updating an existing playlist
#[derive(Debug, InputObject)]
pub struct UpdatePlaylistInput {
    /// New playlist name (optional)
    pub name: Option<String>,
    /// New playlist description (optional)
    pub description: Option<String>,
    /// New cover image URL (optional)
    pub image_url: Option<String>,
    /// Whether the playlist is publicly visible (optional)
    pub is_public: Option<bool>,
    /// Updated smart playlist rules (optional, only for smart playlists)
    pub smart_rules: Option<SmartPlaylistRulesInput>,
}

/// Input for adding tracks to a playlist
#[derive(Debug, InputObject)]
pub struct AddTracksInput {
    /// Track IDs to add
    pub track_ids: Vec<ID>,
    /// Position to insert at (optional, defaults to end)
    pub position: Option<i32>,
}

/// Input for removing tracks from a playlist
#[derive(Debug, InputObject)]
pub struct RemoveTracksInput {
    /// Track IDs to remove
    pub track_ids: Vec<ID>,
}

// =============================================================================
// Mutations
// =============================================================================

/// Playlist mutations
#[derive(Default)]
pub struct PlaylistMutation;

#[Object]
impl PlaylistMutation {
    /// Create a new playlist
    ///
    /// Creates a new playlist for the authenticated user. If creating a smart
    /// playlist, you must provide the smart_rules field.
    ///
    /// # Arguments
    /// * `input` - The playlist creation input
    ///
    /// # Returns
    /// The newly created playlist
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if smart playlist created without rules
    /// - Returns error if validation fails
    async fn create_playlist(
        &self,
        ctx: &Context<'_>,
        input: CreatePlaylistInput,
    ) -> Result<Playlist> {
        // Get authenticated user
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        // Validate input
        let name = input.name.trim();
        if name.is_empty() {
            return Err(async_graphql::Error::new("Playlist name cannot be empty"));
        }
        if name.len() > MAX_NAME_LENGTH {
            return Err(async_graphql::Error::new(format!(
                "Playlist name cannot exceed {} characters",
                MAX_NAME_LENGTH
            )));
        }

        if let Some(ref desc) = input.description {
            if desc.len() > MAX_DESCRIPTION_LENGTH {
                return Err(async_graphql::Error::new(format!(
                    "Playlist description cannot exceed {} characters",
                    MAX_DESCRIPTION_LENGTH
                )));
            }
        }

        if input.playlist_type == PlaylistTypeInput::Smart && input.smart_rules.is_none() {
            return Err(async_graphql::Error::new(
                "Smart playlists require rules to be defined",
            ));
        }

        if let Some(ref rules) = input.smart_rules {
            if rules.rules.is_empty() {
                return Err(async_graphql::Error::new(
                    "Smart playlist must have at least one rule",
                ));
            }
            validate_smart_rules(rules)?;
        }

        let playlist_repo = ctx.data::<PlaylistRepository>()?;

        // Convert smart rules using From trait
        let smart_rules: Option<SmartPlaylistRules> = input.smart_rules.map(Into::into);

        let playlist = playlist_repo
            .create(
                claims.sub,
                name,
                input.description.as_deref(),
                input.is_public,
                input.playlist_type.into(),
                smart_rules,
            )
            .await
            .map_err(|e| to_graphql_error(e.into()))?;

        Ok(Playlist::from(playlist))
    }

    /// Update an existing playlist
    ///
    /// Updates the specified playlist. Only the playlist owner can update it.
    ///
    /// # Arguments
    /// * `id` - The playlist ID to update
    /// * `input` - The update input with optional fields
    ///
    /// # Returns
    /// The updated playlist
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if playlist not found
    /// - Returns error if user doesn't own the playlist
    async fn update_playlist(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdatePlaylistInput,
    ) -> Result<Playlist> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let playlist_id: Uuid = id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid playlist ID"))?;

        // Validate input lengths
        if let Some(ref name) = input.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(async_graphql::Error::new("Playlist name cannot be empty"));
            }
            if name.len() > MAX_NAME_LENGTH {
                return Err(async_graphql::Error::new(format!(
                    "Playlist name cannot exceed {} characters",
                    MAX_NAME_LENGTH
                )));
            }
        }

        if let Some(ref desc) = input.description {
            if desc.len() > MAX_DESCRIPTION_LENGTH {
                return Err(async_graphql::Error::new(format!(
                    "Playlist description cannot exceed {} characters",
                    MAX_DESCRIPTION_LENGTH
                )));
            }
        }

        let playlist_repo = ctx.data::<PlaylistRepository>()?;

        // Check if playlist exists and user owns it
        let existing = playlist_repo
            .find_by_id(playlist_id)
            .await
            .map_err(|e| to_graphql_error(e.into()))?
            .ok_or_else(|| async_graphql::Error::new("Playlist not found"))?;

        if existing.user_id != claims.sub {
            return Err(async_graphql::Error::new(
                "You don't have permission to update this playlist",
            ));
        }

        // Validate smart rules if provided
        if let Some(ref rules) = input.smart_rules {
            validate_smart_rules(rules)?;
        }

        // Convert smart rules using From trait
        let smart_rules: Option<SmartPlaylistRules> = input.smart_rules.map(Into::into);

        // Trim name if provided
        let name = input.name.as_ref().map(|n| n.trim());

        let updated = playlist_repo
            .update(
                playlist_id,
                name,
                input.description.as_deref(),
                input.image_url.as_deref(),
                input.is_public,
                smart_rules,
            )
            .await
            .map_err(|e| to_graphql_error(e.into()))?;

        Ok(Playlist::from(updated))
    }

    /// Delete a playlist
    ///
    /// Permanently deletes the specified playlist. Only the playlist owner can delete it.
    ///
    /// # Arguments
    /// * `id` - The playlist ID to delete
    ///
    /// # Returns
    /// True if the playlist was deleted successfully
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if playlist not found
    /// - Returns error if user doesn't own the playlist
    async fn delete_playlist(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let playlist_id: Uuid = id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid playlist ID"))?;

        let playlist_repo = ctx.data::<PlaylistRepository>()?;

        // Check ownership
        let existing = playlist_repo
            .find_by_id(playlist_id)
            .await
            .map_err(|e| to_graphql_error(e.into()))?
            .ok_or_else(|| async_graphql::Error::new("Playlist not found"))?;

        if existing.user_id != claims.sub {
            return Err(async_graphql::Error::new(
                "You don't have permission to delete this playlist",
            ));
        }

        let tracks_deleted = playlist_repo
            .delete(playlist_id)
            .await
            .map_err(|e| to_graphql_error(e.into()))?;

        tracing::info!(
            playlist_id = %playlist_id,
            user_id = %claims.sub,
            tracks_deleted = tracks_deleted,
            "Playlist deleted"
        );

        Ok(true)
    }

    /// Refresh a smart playlist
    ///
    /// Re-evaluates the smart playlist rules and updates the track list.
    /// Only works on smart playlists.
    ///
    /// # Arguments
    /// * `id` - The smart playlist ID to refresh
    ///
    /// # Returns
    /// The refreshed playlist with updated tracks
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if playlist not found
    /// - Returns error if playlist is not a smart playlist
    async fn refresh_smart_playlist(&self, ctx: &Context<'_>, id: ID) -> Result<Playlist> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let playlist_id: Uuid = id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid playlist ID"))?;

        let playlist_repo = ctx.data::<PlaylistRepository>()?;

        // Check if playlist exists and is owned by user
        let existing = playlist_repo
            .find_by_id(playlist_id)
            .await
            .map_err(|e| to_graphql_error(e.into()))?
            .ok_or_else(|| async_graphql::Error::new("Playlist not found"))?;

        if existing.user_id != claims.sub {
            return Err(async_graphql::Error::new(
                "You don't have permission to refresh this playlist",
            ));
        }

        if existing.playlist_type != DbPlaylistType::Smart {
            return Err(async_graphql::Error::new(
                "Only smart playlists can be refreshed",
            ));
        }

        // TODO: Implement smart rule evaluation in PlaylistService (Step 3)
        // For now, just return the existing playlist
        // let playlist_service = ctx.data::<PlaylistService>()?;
        // let refreshed = playlist_service.refresh_smart_playlist(playlist_id).await?;

        tracing::info!(
            playlist_id = %playlist_id,
            user_id = %claims.sub,
            "Smart playlist refresh requested (evaluation not yet implemented)"
        );

        Ok(Playlist::from(existing))
    }

    /// Add tracks to a playlist
    ///
    /// Adds one or more tracks to a playlist. Only works on manual playlists.
    ///
    /// # Arguments
    /// * `playlist_id` - The playlist ID
    /// * `input` - The tracks to add and optional position
    ///
    /// # Returns
    /// The updated playlist
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if playlist not found
    /// - Returns error if user can't edit the playlist
    /// - Returns error if playlist is a smart playlist
    async fn add_tracks_to_playlist(
        &self,
        ctx: &Context<'_>,
        playlist_id: ID,
        input: AddTracksInput,
    ) -> Result<Playlist> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let playlist_id: Uuid = playlist_id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid playlist ID"))?;

        let playlist_repo = ctx.data::<PlaylistRepository>()?;

        // Check permissions
        let existing = playlist_repo
            .find_by_id(playlist_id)
            .await
            .map_err(|e| to_graphql_error(e.into()))?
            .ok_or_else(|| async_graphql::Error::new("Playlist not found"))?;

        // Check if user can edit (owner or collaborator)
        let can_edit = playlist_repo
            .can_edit(playlist_id, claims.sub)
            .await
            .map_err(|e| to_graphql_error(e.into()))?;

        if !can_edit {
            return Err(async_graphql::Error::new(
                "You don't have permission to edit this playlist",
            ));
        }

        if existing.playlist_type == DbPlaylistType::Smart {
            return Err(async_graphql::Error::new(
                "Cannot manually add tracks to a smart playlist",
            ));
        }

        // Validate track count limit
        if input.track_ids.len() > MAX_TRACKS_PER_OPERATION {
            return Err(async_graphql::Error::new(format!(
                "Cannot add more than {} tracks at once",
                MAX_TRACKS_PER_OPERATION
            )));
        }

        // Parse track IDs
        let track_ids: Result<Vec<Uuid>, _> = input
            .track_ids
            .iter()
            .map(|id| id.parse::<Uuid>())
            .collect();
        let track_ids = track_ids.map_err(|_| async_graphql::Error::new("Invalid track ID"))?;

        if track_ids.is_empty() {
            return Err(async_graphql::Error::new("No tracks provided"));
        }

        playlist_repo
            .add_tracks(playlist_id, &track_ids, claims.sub, input.position)
            .await
            .map_err(|e| to_graphql_error(e.into()))?;

        // Re-fetch the playlist to get updated stats
        let updated = playlist_repo
            .find_by_id(playlist_id)
            .await
            .map_err(|e| to_graphql_error(e.into()))?
            .ok_or_else(|| async_graphql::Error::new("Playlist not found"))?;

        Ok(Playlist::from(updated))
    }

    /// Remove tracks from a playlist
    ///
    /// Removes one or more tracks from a playlist. Only works on manual playlists.
    ///
    /// # Arguments
    /// * `playlist_id` - The playlist ID
    /// * `input` - The tracks to remove
    ///
    /// # Returns
    /// The updated playlist
    ///
    /// # Errors
    /// - Returns error if not authenticated
    /// - Returns error if playlist not found
    /// - Returns error if user can't edit the playlist
    /// - Returns error if playlist is a smart playlist
    async fn remove_tracks_from_playlist(
        &self,
        ctx: &Context<'_>,
        playlist_id: ID,
        input: RemoveTracksInput,
    ) -> Result<Playlist> {
        let claims = ctx
            .data_opt::<Claims>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

        let playlist_id: Uuid = playlist_id
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid playlist ID"))?;

        let playlist_repo = ctx.data::<PlaylistRepository>()?;

        // Check permissions
        let existing = playlist_repo
            .find_by_id(playlist_id)
            .await
            .map_err(|e| to_graphql_error(e.into()))?
            .ok_or_else(|| async_graphql::Error::new("Playlist not found"))?;

        let can_edit = playlist_repo
            .can_edit(playlist_id, claims.sub)
            .await
            .map_err(|e| to_graphql_error(e.into()))?;

        if !can_edit {
            return Err(async_graphql::Error::new(
                "You don't have permission to edit this playlist",
            ));
        }

        if existing.playlist_type == DbPlaylistType::Smart {
            return Err(async_graphql::Error::new(
                "Cannot manually remove tracks from a smart playlist",
            ));
        }

        // Validate track count limit
        if input.track_ids.len() > MAX_TRACKS_PER_OPERATION {
            return Err(async_graphql::Error::new(format!(
                "Cannot remove more than {} tracks at once",
                MAX_TRACKS_PER_OPERATION
            )));
        }

        // Parse track IDs
        let track_ids: Result<Vec<Uuid>, _> = input
            .track_ids
            .iter()
            .map(|id| id.parse::<Uuid>())
            .collect();
        let track_ids = track_ids.map_err(|_| async_graphql::Error::new("Invalid track ID"))?;

        if track_ids.is_empty() {
            return Err(async_graphql::Error::new("No tracks provided"));
        }

        playlist_repo
            .remove_tracks(playlist_id, &track_ids)
            .await
            .map_err(|e| to_graphql_error(e.into()))?;

        // Re-fetch the playlist to get updated stats
        let updated = playlist_repo
            .find_by_id(playlist_id)
            .await
            .map_err(|e| to_graphql_error(e.into()))?
            .ok_or_else(|| async_graphql::Error::new("Playlist not found"))?;

        Ok(Playlist::from(updated))
    }
}

// =============================================================================
// Validation Helpers
// =============================================================================

/// Valid fields for smart playlist rules
const VALID_FIELDS: &[&str] = &[
    // String fields
    "title",
    "artist",
    "album",
    "genre",
    "genres",
    "ai_mood",
    "ai_tags",
    // Numeric fields
    "duration_ms",
    "play_count",
    "skip_count",
    "bpm",
    "energy",
    "danceability",
    "valence",
    "acousticness",
    "instrumentalness",
    "speechiness",
    "loudness",
    // Date fields
    "created_at",
    "last_played_at",
    // Special similarity field
    "similar_to",
];

/// Valid operators for smart playlist rules
const VALID_OPERATORS: &[&str] = &[
    // String operators
    "equals",
    "not_equals",
    "contains",
    "not_contains",
    "starts_with",
    "ends_with",
    "in",
    "not_in",
    "is_empty",
    // Numeric operators
    "greater_than",
    "less_than",
    "greater_than_or_equal",
    "less_than_or_equal",
    "between",
    // Similarity operators
    "combined",
    "semantic",
    "acoustic",
    "categorical",
];

/// Validate smart playlist rules
fn validate_smart_rules(rules: &SmartPlaylistRulesInput) -> Result<()> {
    // Validate rule count limit
    if rules.rules.len() > MAX_RULES {
        return Err(async_graphql::Error::new(format!(
            "Smart playlist cannot have more than {} rules",
            MAX_RULES
        )));
    }

    // Validate match_mode
    if rules.match_mode != "all" && rules.match_mode != "any" {
        return Err(async_graphql::Error::new(
            "match_mode must be 'all' or 'any'",
        ));
    }

    // Validate sort_by if provided
    if let Some(ref sort_by) = rules.sort_by {
        if !VALID_FIELDS.contains(&sort_by.as_str()) {
            return Err(async_graphql::Error::new(format!(
                "Invalid sort_by '{}'. Valid fields: {}",
                sort_by,
                VALID_FIELDS.join(", ")
            )));
        }
    }

    // Validate sort_order if provided
    if let Some(ref order) = rules.sort_order {
        if order != "asc" && order != "desc" {
            return Err(async_graphql::Error::new(
                "sort_order must be 'asc' or 'desc'",
            ));
        }
    }

    // Validate limit if provided
    if let Some(limit) = rules.limit {
        if limit <= 0 {
            return Err(async_graphql::Error::new("Limit must be a positive number"));
        }
        if limit > 10000 {
            return Err(async_graphql::Error::new(
                "Limit cannot exceed 10,000 tracks",
            ));
        }
    }

    // Validate each rule
    for rule in &rules.rules {
        // Validate field
        if !VALID_FIELDS.contains(&rule.field.as_str()) {
            return Err(async_graphql::Error::new(format!(
                "Invalid field '{}'. Valid fields: {}",
                rule.field,
                VALID_FIELDS.join(", ")
            )));
        }

        // Validate operator
        if !VALID_OPERATORS.contains(&rule.operator.as_str()) {
            return Err(async_graphql::Error::new(format!(
                "Invalid operator '{}'. Valid operators: {}",
                rule.operator,
                VALID_OPERATORS.join(", ")
            )));
        }

        // Define similarity operators for bidirectional validation
        const SIMILARITY_OPERATORS: &[&str] = &["combined", "semantic", "acoustic", "categorical"];

        // Validate similar_to rules have proper structure
        if rule.field == "similar_to" {
            if !SIMILARITY_OPERATORS.contains(&rule.operator.as_str()) {
                return Err(async_graphql::Error::new(
                    "similar_to field requires operator: combined, semantic, acoustic, or categorical",
                ));
            }
        } else if SIMILARITY_OPERATORS.contains(&rule.operator.as_str()) {
            // Similarity operators can ONLY be used with similar_to field
            return Err(async_graphql::Error::new(format!(
                "Operator '{}' can only be used with the 'similar_to' field",
                rule.operator
            )));
        }

        // Continue validation for similar_to rules
        if rule.field == "similar_to" {
            // Check for track_ids in value
            let track_ids = rule.value.get("track_ids").ok_or_else(|| {
                async_graphql::Error::new("similar_to rule requires 'track_ids' in value")
            })?;

            // Validate track_ids array
            if let Some(arr) = track_ids.as_array() {
                if arr.is_empty() {
                    return Err(async_graphql::Error::new(
                        "similar_to rule requires at least one seed track",
                    ));
                }
                if arr.len() > MAX_SIMILAR_TO_SEEDS {
                    return Err(async_graphql::Error::new(format!(
                        "similar_to rule cannot have more than {} seed tracks",
                        MAX_SIMILAR_TO_SEEDS
                    )));
                }
                // Validate each track_id is a valid UUID string
                for (i, id) in arr.iter().enumerate() {
                    if let Some(id_str) = id.as_str() {
                        if Uuid::parse_str(id_str).is_err() {
                            return Err(async_graphql::Error::new(format!(
                                "Invalid track ID at index {}: {}",
                                i, id_str
                            )));
                        }
                    } else {
                        return Err(async_graphql::Error::new(format!(
                            "track_ids[{}] must be a string UUID",
                            i
                        )));
                    }
                }
            } else {
                return Err(async_graphql::Error::new(
                    "track_ids must be an array of UUIDs",
                ));
            }

            // Validate min_score if provided
            if let Some(min_score) = rule.value.get("min_score") {
                if let Some(score) = min_score.as_f64() {
                    if !(0.0..=1.0).contains(&score) {
                        return Err(async_graphql::Error::new(
                            "min_score must be between 0.0 and 1.0",
                        ));
                    }
                } else {
                    return Err(async_graphql::Error::new("min_score must be a number"));
                }
            }
        }
    }

    Ok(())
}
