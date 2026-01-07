//! Search-related GraphQL types
//!
//! This module defines types for semantic search, mood-based discovery,
//! similar artist queries, and full-text search via Meilisearch.

use async_graphql::{ComplexObject, Context, Enum, Result, SimpleObject};
use uuid::Uuid;

use crate::services::meilisearch::{
    AlbumSearchHit as ServiceAlbumSearchHit, ArtistSearchHit as ServiceArtistSearchHit,
    TrackSearchHit as ServiceTrackSearchHit, UnifiedSearchResults as ServiceUnifiedSearchResults,
};
use crate::services::search::{MoodTag as ServiceMoodTag, ScoredTrack as ServiceScoredTrack};
use crate::services::similarity::{
    SimilarTrack as ServiceSimilarTrack, SimilarityType as ServiceSimilarityType,
};

use super::{Album, Artist, Track};

/// A track with its similarity/relevance score
#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct ScoredTrack {
    /// The track ID
    pub track_id: Uuid,
    /// Track title
    pub title: String,
    /// Artist name (if available)
    pub artist_name: Option<String>,
    /// Album title (if available)
    pub album_title: Option<String>,
    /// Relevance/similarity score (0.0 - 1.0)
    pub score: f64,
}

#[ComplexObject]
impl ScoredTrack {
    /// Full track details (requires additional query)
    async fn track(&self, ctx: &Context<'_>) -> Result<Option<Track>> {
        use crate::repositories::TrackRepository;
        let repo = ctx.data::<TrackRepository>()?;
        let track = repo.find_by_id(self.track_id).await?;
        Ok(track.map(Track::from))
    }
}

impl From<ServiceScoredTrack> for ScoredTrack {
    fn from(st: ServiceScoredTrack) -> Self {
        let score = if st.score.is_finite() {
            st.score.clamp(0.0, 1.0)
        } else {
            0.0
        };

        Self {
            track_id: st.track_id,
            title: st.title,
            artist_name: st.artist_name,
            album_title: st.album_title,
            score,
        }
    }
}

impl From<ServiceSimilarTrack> for ScoredTrack {
    fn from(st: ServiceSimilarTrack) -> Self {
        let score = if st.score.is_finite() {
            st.score.clamp(0.0, 1.0)
        } else {
            0.0
        };

        Self {
            track_id: st.track_id,
            title: st.title,
            artist_name: st.artist_name,
            album_title: st.album_title,
            score,
        }
    }
}

/// Result of a semantic search query
#[derive(Debug, Clone, SimpleObject)]
pub struct SemanticSearchResult {
    /// Matching tracks with relevance scores
    pub tracks: Vec<ScoredTrack>,
    /// How the AI interpreted the query (for display to user)
    pub interpretation: Option<String>,
}

/// A mood tag with usage statistics
#[derive(Debug, Clone, SimpleObject)]
pub struct MoodTag {
    /// The mood name (e.g., "happy", "energetic", "melancholic")
    pub name: String,
    /// Number of tracks with this mood
    pub track_count: i64,
}

impl From<ServiceMoodTag> for MoodTag {
    fn from(mt: ServiceMoodTag) -> Self {
        Self {
            name: mt.name,
            track_count: mt.track_count,
        }
    }
}

/// Similar artist from Last.fm with local library status
#[derive(Debug, Clone, SimpleObject)]
pub struct SimilarArtist {
    /// Artist name
    pub name: String,
    /// MusicBrainz ID (if available)
    pub mbid: Option<String>,
    /// Similarity score (0.0 - 1.0)
    pub match_score: f64,
    /// Whether this artist is in the local library
    pub in_library: bool,
    /// Local artist ID (if in library)
    pub local_artist_id: Option<Uuid>,
    /// Number of tracks in library (if in library)
    pub track_count: Option<i64>,
}

impl From<crate::services::lastfm::SimilarArtistWithStatus> for SimilarArtist {
    fn from(sa: crate::services::lastfm::SimilarArtistWithStatus) -> Self {
        Self {
            name: sa.name,
            mbid: sa.mbid,
            match_score: sa.match_score,
            in_library: sa.in_library,
            local_artist_id: sa.local_artist_id,
            track_count: sa.track_count,
        }
    }
}

/// Artist tag/genre from Last.fm
#[derive(Debug, Clone, SimpleObject)]
pub struct ArtistTag {
    /// Tag name (e.g., "rock", "alternative")
    pub name: String,
    /// Usage count on Last.fm (higher means more popular)
    pub count: i32,
}

impl From<resonance_lastfm_client::ArtistTag> for ArtistTag {
    fn from(tag: resonance_lastfm_client::ArtistTag) -> Self {
        Self {
            name: tag.name,
            count: tag.count.unwrap_or(0),
        }
    }
}

// ==================== Similarity Types ====================

/// Similarity method to use when finding similar tracks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum SimilarityMethod {
    /// Weighted blend: 50% semantic, 30% acoustic, 20% categorical
    Combined,
    /// AI embeddings similarity (pgvector cosine distance)
    Semantic,
    /// Audio features: BPM, energy, loudness, valence, danceability
    Acoustic,
    /// Genre and mood tag matching (weighted Jaccard)
    Categorical,
}

/// The similarity algorithm that produced a match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum SimilarityType {
    /// Based on AI-generated description embeddings
    Semantic,
    /// Based on audio features (loudness, energy, BPM, etc.)
    Acoustic,
    /// Based on genre and mood tags
    Categorical,
    /// Combined similarity using multiple factors
    Combined,
}

impl From<ServiceSimilarityType> for SimilarityType {
    fn from(st: ServiceSimilarityType) -> Self {
        match st {
            ServiceSimilarityType::Semantic => Self::Semantic,
            ServiceSimilarityType::Acoustic => Self::Acoustic,
            ServiceSimilarityType::Categorical => Self::Categorical,
            ServiceSimilarityType::Combined => Self::Combined,
        }
    }
}

/// A track with its similarity score and the method used to find it
#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct SimilarTrack {
    /// The track ID
    pub track_id: Uuid,
    /// Track title
    pub title: String,
    /// Artist name (if available)
    pub artist_name: Option<String>,
    /// Album title (if available)
    pub album_title: Option<String>,
    /// Similarity score (0.0 - 1.0)
    pub score: f64,
    /// The type of similarity used for this match
    pub similarity_type: SimilarityType,
}

#[ComplexObject]
impl SimilarTrack {
    /// Full track details (requires additional query)
    async fn track(&self, ctx: &Context<'_>) -> Result<Option<Track>> {
        use crate::repositories::TrackRepository;
        let repo = ctx.data::<TrackRepository>()?;
        let track = repo.find_by_id(self.track_id).await?;
        Ok(track.map(Track::from))
    }
}

impl From<ServiceSimilarTrack> for SimilarTrack {
    fn from(st: ServiceSimilarTrack) -> Self {
        let score = if st.score.is_finite() {
            st.score.clamp(0.0, 1.0)
        } else {
            0.0
        };

        Self {
            track_id: st.track_id,
            title: st.title,
            artist_name: st.artist_name,
            album_title: st.album_title,
            score,
            similarity_type: st.similarity_type.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scored_track_from_service() {
        let service_track = ServiceScoredTrack {
            track_id: Uuid::new_v4(),
            title: "Test Track".to_string(),
            artist_id: Uuid::new_v4(),
            artist_name: Some("Test Artist".to_string()),
            album_id: None,
            album_title: None,
            score: 0.85,
        };

        let graphql_track: ScoredTrack = service_track.into();
        assert_eq!(graphql_track.title, "Test Track");
        assert_eq!(graphql_track.artist_name, Some("Test Artist".to_string()));
        assert!((graphql_track.score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similar_track_to_scored_track() {
        use crate::services::similarity::SimilarityType;

        let similar_track = ServiceSimilarTrack {
            track_id: Uuid::new_v4(),
            title: "Similar Track".to_string(),
            artist_name: Some("Similar Artist".to_string()),
            album_title: Some("Album".to_string()),
            score: 0.72,
            similarity_type: SimilarityType::Combined,
        };

        let scored: ScoredTrack = similar_track.into();
        assert_eq!(scored.title, "Similar Track");
        assert_eq!(scored.album_title, Some("Album".to_string()));
        assert!((scored.score - 0.72).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mood_tag_from_service() {
        let service_tag = ServiceMoodTag {
            name: "happy".to_string(),
            track_count: 42,
        };

        let graphql_tag: MoodTag = service_tag.into();
        assert_eq!(graphql_tag.name, "happy");
        assert_eq!(graphql_tag.track_count, 42);
    }

    #[test]
    fn test_similarity_type_from_service() {
        assert_eq!(
            SimilarityType::from(ServiceSimilarityType::Semantic),
            SimilarityType::Semantic
        );
        assert_eq!(
            SimilarityType::from(ServiceSimilarityType::Acoustic),
            SimilarityType::Acoustic
        );
        assert_eq!(
            SimilarityType::from(ServiceSimilarityType::Categorical),
            SimilarityType::Categorical
        );
        assert_eq!(
            SimilarityType::from(ServiceSimilarityType::Combined),
            SimilarityType::Combined
        );
    }

    #[test]
    fn test_similar_track_from_service() {
        let service_track = ServiceSimilarTrack {
            track_id: Uuid::new_v4(),
            title: "Test Track".to_string(),
            artist_name: Some("Test Artist".to_string()),
            album_title: Some("Test Album".to_string()),
            score: 0.85,
            similarity_type: ServiceSimilarityType::Acoustic,
        };

        let graphql_track: SimilarTrack = service_track.into();
        assert_eq!(graphql_track.title, "Test Track");
        assert_eq!(graphql_track.artist_name, Some("Test Artist".to_string()));
        assert_eq!(graphql_track.album_title, Some("Test Album".to_string()));
        assert!((graphql_track.score - 0.85).abs() < f64::EPSILON);
        assert_eq!(graphql_track.similarity_type, SimilarityType::Acoustic);
    }

    #[test]
    fn test_similarity_method_variants() {
        // Ensure all enum variants exist and are accessible
        let _combined = SimilarityMethod::Combined;
        let _semantic = SimilarityMethod::Semantic;
        let _acoustic = SimilarityMethod::Acoustic;
        let _categorical = SimilarityMethod::Categorical;
    }

    #[test]
    fn test_similar_track_with_none_values() {
        let service_track = ServiceSimilarTrack {
            track_id: Uuid::new_v4(),
            title: "Track Without Metadata".to_string(),
            artist_name: None,
            album_title: None,
            score: 0.0,
            similarity_type: ServiceSimilarityType::Semantic,
        };

        let graphql_track: SimilarTrack = service_track.into();
        assert!(graphql_track.artist_name.is_none());
        assert!(graphql_track.album_title.is_none());
        assert!((graphql_track.score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_score_sanitization_nan() {
        let service_track = ServiceSimilarTrack {
            track_id: Uuid::new_v4(),
            title: "Test".to_string(),
            artist_name: None,
            album_title: None,
            score: f64::NAN,
            similarity_type: ServiceSimilarityType::Semantic,
        };

        let graphql_track: SimilarTrack = service_track.into();
        assert!((graphql_track.score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_score_sanitization_infinity() {
        let service_track = ServiceSimilarTrack {
            track_id: Uuid::new_v4(),
            title: "Test".to_string(),
            artist_name: None,
            album_title: None,
            score: f64::INFINITY,
            similarity_type: ServiceSimilarityType::Semantic,
        };

        let graphql_track: SimilarTrack = service_track.into();
        assert!((graphql_track.score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_score_sanitization_clamps_to_range() {
        // Test negative score gets clamped to 0
        let service_track = ServiceSimilarTrack {
            track_id: Uuid::new_v4(),
            title: "Test".to_string(),
            artist_name: None,
            album_title: None,
            score: -0.5,
            similarity_type: ServiceSimilarityType::Semantic,
        };
        let graphql_track: SimilarTrack = service_track.into();
        assert!((graphql_track.score - 0.0).abs() < f64::EPSILON);

        // Test score > 1.0 gets clamped to 1.0
        let service_track = ServiceSimilarTrack {
            track_id: Uuid::new_v4(),
            title: "Test".to_string(),
            artist_name: None,
            album_title: None,
            score: 1.5,
            similarity_type: ServiceSimilarityType::Semantic,
        };
        let graphql_track: SimilarTrack = service_track.into();
        assert!((graphql_track.score - 1.0).abs() < f64::EPSILON);
    }
}

// ==================== Full-Text Search Types (Meilisearch) ====================

/// A track result from full-text search
#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct FullTextTrackHit {
    /// The track ID
    pub track_id: Uuid,
    /// Track title
    pub title: String,
    /// Artist name
    pub artist_name: String,
    /// Artist ID
    pub artist_id: Uuid,
    /// Album title (if available)
    pub album_title: Option<String>,
    /// Album ID (if available)
    pub album_id: Option<Uuid>,
    /// Genres
    pub genres: Vec<String>,
    /// AI-detected moods
    pub moods: Vec<String>,
    /// Duration in milliseconds
    pub duration_ms: i32,
}

#[ComplexObject]
impl FullTextTrackHit {
    /// Full track details (requires additional query)
    async fn track(&self, ctx: &Context<'_>) -> Result<Option<Track>> {
        use crate::repositories::TrackRepository;
        let repo = ctx.data::<TrackRepository>()?;
        let track = repo.find_by_id(self.track_id).await?;
        Ok(track.map(Track::from))
    }
}

impl From<ServiceTrackSearchHit> for FullTextTrackHit {
    fn from(hit: ServiceTrackSearchHit) -> Self {
        Self {
            track_id: hit.track_id,
            title: hit.title,
            artist_name: hit.artist_name,
            artist_id: hit.artist_id,
            album_title: hit.album_title,
            album_id: hit.album_id,
            genres: hit.genres,
            moods: hit.moods,
            duration_ms: hit.duration_ms,
        }
    }
}

/// An album result from full-text search
#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct FullTextAlbumHit {
    /// The album ID
    pub album_id: Uuid,
    /// Album title
    pub title: String,
    /// Artist name
    pub artist_name: String,
    /// Artist ID
    pub artist_id: Uuid,
    /// Genres
    pub genres: Vec<String>,
    /// Album type (album, single, EP, etc.)
    pub album_type: String,
    /// Release year
    pub release_year: Option<i32>,
}

#[ComplexObject]
impl FullTextAlbumHit {
    /// Full album details (requires additional query)
    async fn album(&self, ctx: &Context<'_>) -> Result<Option<Album>> {
        use crate::repositories::AlbumRepository;
        let repo = ctx.data::<AlbumRepository>()?;
        let album = repo.find_by_id(self.album_id).await?;
        Ok(album.map(Album::from))
    }
}

impl From<ServiceAlbumSearchHit> for FullTextAlbumHit {
    fn from(hit: ServiceAlbumSearchHit) -> Self {
        Self {
            album_id: hit.album_id,
            title: hit.title,
            artist_name: hit.artist_name,
            artist_id: hit.artist_id,
            genres: hit.genres,
            album_type: hit.album_type,
            release_year: hit.release_year,
        }
    }
}

/// An artist result from full-text search
#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct FullTextArtistHit {
    /// The artist ID
    pub artist_id: Uuid,
    /// Artist name
    pub name: String,
    /// Genres
    pub genres: Vec<String>,
}

#[ComplexObject]
impl FullTextArtistHit {
    /// Full artist details (requires additional query)
    async fn artist(&self, ctx: &Context<'_>) -> Result<Option<Artist>> {
        use crate::repositories::ArtistRepository;
        let repo = ctx.data::<ArtistRepository>()?;
        let artist = repo.find_by_id(self.artist_id).await?;
        Ok(artist.map(Artist::from))
    }
}

impl From<ServiceArtistSearchHit> for FullTextArtistHit {
    fn from(hit: ServiceArtistSearchHit) -> Self {
        Self {
            artist_id: hit.artist_id,
            name: hit.name,
            genres: hit.genres,
        }
    }
}

/// Unified full-text search results across all entity types
#[derive(Debug, Clone, SimpleObject)]
pub struct FullTextSearchResult {
    /// Matching tracks
    pub tracks: Vec<FullTextTrackHit>,
    /// Matching albums
    pub albums: Vec<FullTextAlbumHit>,
    /// Matching artists
    pub artists: Vec<FullTextArtistHit>,
    /// Total number of hits across all types
    pub total_hits: i32,
    /// Query processing time in milliseconds
    pub processing_time_ms: i32,
}

impl From<ServiceUnifiedSearchResults> for FullTextSearchResult {
    fn from(results: ServiceUnifiedSearchResults) -> Self {
        Self {
            tracks: results
                .tracks
                .into_iter()
                .map(FullTextTrackHit::from)
                .collect(),
            albums: results
                .albums
                .into_iter()
                .map(FullTextAlbumHit::from)
                .collect(),
            artists: results
                .artists
                .into_iter()
                .map(FullTextArtistHit::from)
                .collect(),
            total_hits: results.total_hits as i32,
            processing_time_ms: results.processing_time_ms as i32,
        }
    }
}
