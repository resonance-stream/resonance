//! Last.fm service wrapper
//!
//! Provides similar artist discovery and genre enrichment via Last.fm API.
//! Integrates with the local library to indicate which similar artists
//! are already in the user's collection.
//!
//! Used by GraphQL SearchQuery for `similarArtists` and `artistTags` queries.

use sqlx::PgPool;
use tracing::instrument;

use crate::error::{ApiError, ApiResult};

pub use resonance_lastfm_client::{ApiKeyStatus, ArtistTag, LastfmClient, LastfmError};

/// Similar artist with library status
#[derive(Debug, Clone, serde::Serialize)]
pub struct SimilarArtistWithStatus {
    /// Artist name
    pub name: String,
    /// MusicBrainz ID (if available)
    pub mbid: Option<String>,
    /// Similarity score (0.0 - 1.0)
    pub match_score: f64,
    /// Whether this artist is in the local library
    pub in_library: bool,
    /// Local artist ID (if in library)
    pub local_artist_id: Option<uuid::Uuid>,
    /// Track count in library (if in library)
    pub track_count: Option<i64>,
}

/// Last.fm service with library integration
#[derive(Clone)]
pub struct LastfmService {
    client: LastfmClient,
    db: PgPool,
}

/// Map Last.fm errors to API errors with explicit handling of all variants
fn map_lastfm_error(e: LastfmError) -> ApiError {
    match e {
        LastfmError::MissingApiKey => {
            ApiError::Configuration("Last.fm API key not configured".into())
        }
        LastfmError::InvalidInput(msg) => ApiError::ValidationError(msg),
        LastfmError::ArtistNotFound(name) => ApiError::not_found("artist", name),
        LastfmError::RateLimited => ApiError::Lastfm("API rate limited".into()),
        LastfmError::Timeout => ApiError::Lastfm("request timed out".into()),
        LastfmError::Http(err) => ApiError::Lastfm(format!("HTTP error: {}", err)),
        LastfmError::Parse(err) => ApiError::Lastfm(format!("parse error: {}", err)),
        LastfmError::Api { code, message } => {
            ApiError::Lastfm(format!("API error {}: {}", code, message))
        }
    }
}

impl LastfmService {
    /// Create a new Last.fm service
    ///
    /// # Errors
    /// Returns an error if the Last.fm API key is missing or invalid
    #[allow(dead_code)] // Public API for direct instantiation/testing
    pub fn new(client: LastfmClient, db: PgPool) -> Self {
        Self { client, db }
    }

    /// Create from environment variable
    ///
    /// Reads `LASTFM_API_KEY` from the environment.
    pub fn from_env(db: PgPool) -> ApiResult<Self> {
        let client = LastfmClient::from_env().map_err(|e| match e {
            LastfmError::MissingApiKey => {
                ApiError::Configuration("LASTFM_API_KEY environment variable is required".into())
            }
            other => map_lastfm_error(other),
        })?;

        Ok(Self { client, db })
    }

    /// Get similar artists with library status
    ///
    /// Returns similar artists from Last.fm, enriched with information
    /// about whether each artist is present in the local library.
    #[instrument(skip(self))]
    pub async fn get_similar_artists(
        &self,
        artist_name: &str,
        limit: Option<u32>,
    ) -> ApiResult<Vec<SimilarArtistWithStatus>> {
        // Get similar artists from Last.fm
        let similar = self
            .client
            .get_similar_artists(artist_name, limit)
            .await
            .map_err(map_lastfm_error)?;

        // Check which artists are in our library
        let artist_names: Vec<&str> = similar.iter().map(|a| a.name.as_str()).collect();

        let library_artists = self.check_artists_in_library(&artist_names).await?;

        // Combine the results
        let results = similar
            .into_iter()
            .map(|artist| {
                let library_info = library_artists
                    .iter()
                    .find(|la| la.name.eq_ignore_ascii_case(&artist.name));

                SimilarArtistWithStatus {
                    name: artist.name,
                    mbid: artist.mbid,
                    match_score: artist.match_score,
                    in_library: library_info.is_some(),
                    local_artist_id: library_info.map(|la| la.id),
                    track_count: library_info.map(|la| la.track_count),
                }
            })
            .collect();

        Ok(results)
    }

    /// Get artist tags (genres/descriptors)
    #[instrument(skip(self))]
    pub async fn get_artist_tags(&self, artist_name: &str) -> ApiResult<Vec<ArtistTag>> {
        self.client
            .get_artist_tags(artist_name)
            .await
            .map_err(map_lastfm_error)
    }

    /// Check which artists from a list are in the local library
    async fn check_artists_in_library(
        &self,
        artist_names: &[&str],
    ) -> ApiResult<Vec<LibraryArtist>> {
        if artist_names.is_empty() {
            return Ok(Vec::new());
        }

        // Use case-insensitive matching
        let artists: Vec<LibraryArtist> = sqlx::query_as(
            r#"
            SELECT
                a.id,
                a.name,
                COUNT(t.id) as track_count
            FROM artists a
            LEFT JOIN tracks t ON t.artist_id = a.id
            WHERE LOWER(a.name) = ANY($1::text[])
            GROUP BY a.id, a.name
            "#,
        )
        .bind(
            artist_names
                .iter()
                .map(|n| n.to_lowercase())
                .collect::<Vec<_>>(),
        )
        .fetch_all(&self.db)
        .await?;

        Ok(artists)
    }

    /// Validate that the API key is working
    #[allow(dead_code)] // Public API for health checks
    pub async fn validate(&self) -> bool {
        matches!(self.client.validate_api_key().await, ApiKeyStatus::Valid)
    }
}

/// Artist info from local library
#[derive(Debug, sqlx::FromRow)]
struct LibraryArtist {
    id: uuid::Uuid,
    name: String,
    track_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similar_artist_with_status_serialization() {
        let artist = SimilarArtistWithStatus {
            name: "Test Artist".to_string(),
            mbid: Some("abc123".to_string()),
            match_score: 0.85,
            in_library: true,
            local_artist_id: Some(uuid::Uuid::new_v4()),
            track_count: Some(42),
        };

        let json = serde_json::to_string(&artist).unwrap();
        assert!(json.contains("Test Artist"));
        assert!(json.contains("0.85"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_similar_artist_not_in_library() {
        let artist = SimilarArtistWithStatus {
            name: "Unknown Artist".to_string(),
            mbid: None,
            match_score: 0.5,
            in_library: false,
            local_artist_id: None,
            track_count: None,
        };

        assert!(!artist.in_library);
        assert!(artist.local_artist_id.is_none());
        assert!(artist.track_count.is_none());
    }
}
