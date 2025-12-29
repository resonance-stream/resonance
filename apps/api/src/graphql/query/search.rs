//! Search queries for Resonance GraphQL API
//!
//! This module provides queries for AI-powered search and discovery:
//! - Semantic search using embeddings
//! - Similar tracks using audio features (bliss)
//! - Mood-based track discovery
//! - Similar artists via Last.fm

use async_graphql::{Context, Object, Result, ID};
use tracing::{debug, instrument, warn};
use uuid::Uuid;

use crate::graphql::pagination::{clamp_limit, MAX_SEARCH_LIMIT};
use crate::graphql::types::{ArtistTag, MoodTag, ScoredTrack, SemanticSearchResult, SimilarArtist};
use crate::services::lastfm::LastfmService;
use crate::services::search::SearchService;
use crate::services::similarity::SimilarityService;

/// Search and discovery queries using AI and external services
#[derive(Default)]
pub struct SearchQuery;

#[Object]
impl SearchQuery {
    // ==================== Semantic Search ====================

    /// Semantic search using natural language query.
    /// Uses AI to understand the query and find matching tracks based on
    /// their descriptions and metadata embeddings.
    /// Requires Ollama to be configured and running.
    #[instrument(skip(self, ctx))]
    async fn semantic_search(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Natural language search query (e.g., 'upbeat songs for working out')")]
        query: String,
        #[graphql(
            default = 20,
            desc = "Maximum number of results (default: 20, max: 50)"
        )]
        limit: i32,
    ) -> Result<SemanticSearchResult> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(SemanticSearchResult {
                tracks: Vec::new(),
                interpretation: None,
            });
        }

        let limit = clamp_limit(limit, MAX_SEARCH_LIMIT) as i32;

        // Get Ollama client for embedding generation
        let ollama = ctx
            .data::<resonance_ollama_client::OllamaClient>()
            .map_err(|_| {
                async_graphql::Error::new("Semantic search is not available: Ollama not configured")
            })?;

        let search_service = ctx.data::<SearchService>()?;

        // Check if we have any embeddings to search
        if !search_service.has_embeddings().await? {
            return Ok(SemanticSearchResult {
                tracks: Vec::new(),
                interpretation: Some(
                    "No tracks have been analyzed yet. Please run embedding generation."
                        .to_string(),
                ),
            });
        }

        // Generate embedding for the query
        debug!(query = %trimmed, "Generating embedding for semantic search");
        let query_embedding = ollama.generate_embedding(trimmed).await.map_err(|e| {
            warn!(error = %e, "Failed to generate query embedding");
            async_graphql::Error::new(format!("Failed to process query: {}", e))
        })?;

        // Search by embedding
        let scored_tracks = search_service
            .search_by_embedding(&query_embedding, limit)
            .await?;

        let tracks: Vec<ScoredTrack> = scored_tracks.into_iter().map(ScoredTrack::from).collect();

        Ok(SemanticSearchResult {
            tracks,
            interpretation: Some(format!("Finding tracks similar to: \"{}\"", trimmed)),
        })
    }

    // ==================== Track Similarity ====================

    /// Find tracks similar to a given track.
    /// Uses combined similarity (semantic, acoustic, and categorical) to find
    /// the most similar tracks in your library.
    #[instrument(skip(self, ctx))]
    async fn similar_tracks(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "ID of the track to find similar tracks for")] track_id: ID,
        #[graphql(
            default = 10,
            desc = "Maximum number of results (default: 10, max: 50)"
        )]
        limit: i32,
    ) -> Result<Vec<ScoredTrack>> {
        let uuid = Uuid::parse_str(&track_id)?;
        let limit = clamp_limit(limit, MAX_SEARCH_LIMIT) as i32;

        let similarity_service = ctx.data::<SimilarityService>()?;
        let similar = similarity_service
            .find_similar_combined(uuid, limit)
            .await?;

        Ok(similar.into_iter().map(ScoredTrack::from).collect())
    }

    // ==================== Mood-Based Discovery ====================

    /// Search tracks by mood tags.
    /// Finds tracks that match any of the specified mood tags.
    /// Results are scored by the number of matching moods.
    #[instrument(skip(self, ctx))]
    async fn search_by_mood(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "List of mood tags to search for (e.g., ['happy', 'energetic'])")]
        moods: Vec<String>,
        #[graphql(
            default = 20,
            desc = "Maximum number of results (default: 20, max: 50)"
        )]
        limit: i32,
    ) -> Result<Vec<ScoredTrack>> {
        if moods.is_empty() {
            return Err(async_graphql::Error::new(
                "At least one mood must be specified",
            ));
        }

        let limit = clamp_limit(limit, MAX_SEARCH_LIMIT) as i32;

        let search_service = ctx.data::<SearchService>()?;
        let tracks = search_service.search_by_mood(&moods, limit).await?;

        Ok(tracks.into_iter().map(ScoredTrack::from).collect())
    }

    /// Get all available mood tags in the library.
    /// Returns a list of mood tags that have been detected in tracks,
    /// along with how many tracks have each mood.
    #[instrument(skip(self, ctx))]
    async fn available_moods(&self, ctx: &Context<'_>) -> Result<Vec<MoodTag>> {
        let search_service = ctx.data::<SearchService>()?;
        let moods = search_service.get_available_moods().await?;
        Ok(moods.into_iter().map(MoodTag::from).collect())
    }

    // ==================== Similar Artists (Last.fm) ====================

    /// Find similar artists using Last.fm.
    /// Returns artists that are musically similar, with information about
    /// whether they're already in your library.
    /// Requires LASTFM_API_KEY to be configured.
    #[instrument(skip(self, ctx))]
    async fn similar_artists(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Name of the artist to find similar artists for")] artist_name: String,
        #[graphql(
            default = 10,
            desc = "Maximum number of results (default: 10, max: 50)"
        )]
        limit: i32,
    ) -> Result<Vec<SimilarArtist>> {
        let trimmed = artist_name.trim();
        if trimmed.is_empty() {
            return Err(async_graphql::Error::new("Artist name cannot be empty"));
        }

        let limit = limit.clamp(1, 50) as u32;

        let lastfm_service = ctx.data::<LastfmService>().map_err(|_| {
            async_graphql::Error::new("Similar artists not available: Last.fm not configured")
        })?;

        let similar = lastfm_service
            .get_similar_artists(trimmed, Some(limit))
            .await?;

        Ok(similar.into_iter().map(SimilarArtist::from).collect())
    }

    /// Get tags/genres for an artist from Last.fm.
    /// Returns genre and style tags associated with the artist.
    /// Requires LASTFM_API_KEY to be configured.
    #[instrument(skip(self, ctx))]
    async fn artist_tags(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Name of the artist to get tags for")] artist_name: String,
    ) -> Result<Vec<ArtistTag>> {
        let trimmed = artist_name.trim();
        if trimmed.is_empty() {
            return Err(async_graphql::Error::new("Artist name cannot be empty"));
        }

        let lastfm_service = ctx.data::<LastfmService>().map_err(|_| {
            async_graphql::Error::new("Artist tags not available: Last.fm not configured")
        })?;

        let tags = lastfm_service.get_artist_tags(trimmed).await?;

        Ok(tags.into_iter().map(ArtistTag::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_constructable() {
        let _query = SearchQuery;
        // SearchQuery should be constructable
    }
}
