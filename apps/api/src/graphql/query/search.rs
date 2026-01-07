//! Search queries for Resonance GraphQL API
//!
//! This module provides queries for AI-powered search and discovery:
//! - Full-text search using Meilisearch
//! - Semantic search using embeddings
//! - Similar tracks using audio features (bliss)
//! - Mood-based track discovery
//! - Similar artists via Last.fm

use async_graphql::{Context, Object, Result, ID};
use tracing::{debug, instrument, warn};
use uuid::Uuid;

use crate::graphql::pagination::{clamp_limit, MAX_SEARCH_LIMIT};
use crate::graphql::types::{
    ArtistTag, FullTextAlbumHit, FullTextArtistHit, FullTextSearchResult, FullTextTrackHit,
    MoodTag, ScoredTrack, SemanticSearchResult, SimilarArtist, SimilarTrack, SimilarityMethod,
};
use crate::services::lastfm::LastfmService;
use crate::services::meilisearch::filter::{
    self, FilterValidationError, ALBUM_ATTRIBUTES, ARTIST_ATTRIBUTES, TRACK_ATTRIBUTES,
};
use crate::services::meilisearch::MeilisearchService;
use crate::services::search::SearchService;
use crate::services::similarity::SimilarityService;

/// Validate a filter string against allowed attributes, converting to GraphQL error if invalid
fn validate_filter<'a>(
    filter: Option<&'a str>,
    allowed_attributes: &[&str],
) -> Result<Option<&'a str>> {
    match filter {
        None => Ok(None),
        Some(f) => match filter::validate(f, allowed_attributes) {
            Ok(validated) => Ok(Some(validated)),
            Err(FilterValidationError::Empty) => Ok(None), // Treat empty as no filter
            Err(e) => {
                warn!(error = %e, filter = f, "Invalid search filter");
                Err(async_graphql::Error::new(format!("Invalid filter: {}", e)))
            }
        },
    }
}

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
        let uuid = Uuid::parse_str(&track_id)
            .map_err(|_| async_graphql::Error::new("Invalid track ID"))?;
        let limit = clamp_limit(limit, MAX_SEARCH_LIMIT) as i32;

        let similarity_service = ctx.data::<SimilarityService>()?;
        let similar = similarity_service
            .find_similar_combined(uuid, limit)
            .await?;

        Ok(similar.into_iter().map(ScoredTrack::from).collect())
    }

    /// Find tracks similar to a given track using a specific similarity method.
    ///
    /// Available methods:
    /// - `COMBINED`: Weighted blend (50% semantic, 30% acoustic, 20% categorical)
    /// - `SEMANTIC`: AI embeddings similarity (requires tracks to have embeddings)
    /// - `ACOUSTIC`: Audio features (BPM, energy, loudness, valence, danceability)
    /// - `CATEGORICAL`: Genre and mood tag matching
    #[instrument(skip(self, ctx), fields(track_id = ?track_id, method = ?method))]
    async fn similar_tracks_by_method(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "ID of the track to find similar tracks for")] track_id: ID,
        #[graphql(desc = "Similarity algorithm to use")] method: SimilarityMethod,
        #[graphql(
            default = 10,
            desc = "Maximum number of results (default: 10, max: 50)"
        )]
        limit: i32,
    ) -> Result<Vec<SimilarTrack>> {
        let uuid = Uuid::parse_str(&track_id)
            .map_err(|_| async_graphql::Error::new("Invalid track ID"))?;
        let limit = clamp_limit(limit, MAX_SEARCH_LIMIT) as i32;

        let similarity_service = ctx.data::<SimilarityService>()?;

        let similar = match method {
            SimilarityMethod::Combined => {
                similarity_service
                    .find_similar_combined(uuid, limit)
                    .await?
            }
            SimilarityMethod::Semantic => {
                similarity_service
                    .find_similar_by_embedding(uuid, limit)
                    .await?
            }
            SimilarityMethod::Acoustic => {
                similarity_service
                    .find_similar_by_features(uuid, limit)
                    .await?
            }
            SimilarityMethod::Categorical => {
                similarity_service.find_similar_by_tags(uuid, limit).await?
            }
        };

        Ok(similar.into_iter().map(SimilarTrack::from).collect())
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

    // ==================== Full-Text Search (Meilisearch) ====================

    /// Full-text search across tracks, albums, and artists.
    /// Uses Meilisearch for fast, typo-tolerant keyword search.
    /// Requires Meilisearch to be configured and running.
    #[instrument(skip(self, ctx))]
    async fn search(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Search query (e.g., 'Beatles Abbey Road')")] query: String,
        #[graphql(default = 10, desc = "Maximum results per type (default: 10, max: 50)")]
        limit: i32,
    ) -> Result<FullTextSearchResult> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(FullTextSearchResult {
                tracks: Vec::new(),
                albums: Vec::new(),
                artists: Vec::new(),
                total_hits: 0,
                processing_time_ms: 0,
            });
        }

        let limit = clamp_limit(limit, MAX_SEARCH_LIMIT);

        let meilisearch = ctx.data::<MeilisearchService>().map_err(|_| {
            async_graphql::Error::new("Search is not available: Meilisearch not configured")
        })?;

        let results = meilisearch
            .search_all(trimmed, Some(limit as usize))
            .await?;

        Ok(FullTextSearchResult::from(results))
    }

    /// Search tracks by keyword using full-text search.
    /// Searches across title, artist name, album name, genres, and moods.
    ///
    /// The filter parameter supports Meilisearch filter syntax with allowed attributes:
    /// `artist_id`, `album_id`, `genres`, `moods`, `explicit`, `duration_ms`
    #[instrument(skip(self, ctx))]
    async fn search_tracks(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Search query")] query: String,
        #[graphql(default = 20, desc = "Maximum results (default: 20, max: 50)")] limit: i32,
        #[graphql(
            desc = "Optional Meilisearch filter (e.g., \"genres = 'Rock'\"). Allowed attributes: artist_id, album_id, genres, moods, explicit, duration_ms"
        )]
        filter: Option<String>,
    ) -> Result<Vec<FullTextTrackHit>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let limit = clamp_limit(limit, MAX_SEARCH_LIMIT);

        // Validate filter before passing to Meilisearch
        let validated_filter = validate_filter(filter.as_deref(), TRACK_ATTRIBUTES)?;

        let meilisearch = ctx.data::<MeilisearchService>().map_err(|_| {
            async_graphql::Error::new("Search is not available: Meilisearch not configured")
        })?;

        let results = meilisearch
            .search_tracks(trimmed, Some(limit as usize), validated_filter)
            .await?;

        Ok(results.into_iter().map(FullTextTrackHit::from).collect())
    }

    /// Search albums by keyword using full-text search.
    /// Searches across title, artist name, and genres.
    ///
    /// The filter parameter supports Meilisearch filter syntax with allowed attributes:
    /// `artist_id`, `genres`, `album_type`, `release_year`
    #[instrument(skip(self, ctx))]
    async fn search_albums(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Search query")] query: String,
        #[graphql(default = 20, desc = "Maximum results (default: 20, max: 50)")] limit: i32,
        #[graphql(
            desc = "Optional Meilisearch filter (e.g., 'release_year > 2020'). Allowed attributes: artist_id, genres, album_type, release_year"
        )]
        filter: Option<String>,
    ) -> Result<Vec<FullTextAlbumHit>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let limit = clamp_limit(limit, MAX_SEARCH_LIMIT);

        // Validate filter before passing to Meilisearch
        let validated_filter = validate_filter(filter.as_deref(), ALBUM_ATTRIBUTES)?;

        let meilisearch = ctx.data::<MeilisearchService>().map_err(|_| {
            async_graphql::Error::new("Search is not available: Meilisearch not configured")
        })?;

        let results = meilisearch
            .search_albums(trimmed, Some(limit as usize), validated_filter)
            .await?;

        Ok(results.into_iter().map(FullTextAlbumHit::from).collect())
    }

    /// Search artists by keyword using full-text search.
    /// Searches across name, sort name, genres, and biography.
    ///
    /// The filter parameter supports Meilisearch filter syntax with allowed attributes:
    /// `genres`
    #[instrument(skip(self, ctx))]
    async fn search_artists(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Search query")] query: String,
        #[graphql(default = 20, desc = "Maximum results (default: 20, max: 50)")] limit: i32,
        #[graphql(
            desc = "Optional Meilisearch filter (e.g., \"genres = 'Jazz'\"). Allowed attributes: genres"
        )]
        filter: Option<String>,
    ) -> Result<Vec<FullTextArtistHit>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let limit = clamp_limit(limit, MAX_SEARCH_LIMIT);

        // Validate filter before passing to Meilisearch
        let validated_filter = validate_filter(filter.as_deref(), ARTIST_ATTRIBUTES)?;

        let meilisearch = ctx.data::<MeilisearchService>().map_err(|_| {
            async_graphql::Error::new("Search is not available: Meilisearch not configured")
        })?;

        let results = meilisearch
            .search_artists(trimmed, Some(limit as usize), validated_filter)
            .await?;

        Ok(results.into_iter().map(FullTextArtistHit::from).collect())
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
