//! Search-related GraphQL types
//!
//! This module defines types for semantic search, mood-based discovery,
//! and similar artist queries.

use async_graphql::{ComplexObject, Context, Result, SimpleObject};
use uuid::Uuid;

use crate::services::search::{MoodTag as ServiceMoodTag, ScoredTrack as ServiceScoredTrack};
use crate::services::similarity::SimilarTrack as ServiceSimilarTrack;

use super::Track;

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
        Self {
            track_id: st.track_id,
            title: st.title,
            artist_name: st.artist_name,
            album_title: st.album_title,
            score: st.score,
        }
    }
}

impl From<ServiceSimilarTrack> for ScoredTrack {
    fn from(st: ServiceSimilarTrack) -> Self {
        Self {
            track_id: st.track_id,
            title: st.title,
            artist_name: st.artist_name,
            album_title: st.album_title,
            score: st.score,
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
}
