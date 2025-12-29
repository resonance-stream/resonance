//! Last.fm API response models

use serde::{Deserialize, Serialize};

/// A similar artist from Last.fm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarArtist {
    /// Artist name
    pub name: String,
    /// MusicBrainz ID (if available)
    pub mbid: Option<String>,
    /// Similarity score (0.0 - 1.0)
    pub match_score: f64,
    /// URL to Last.fm artist page
    pub url: Option<String>,
}

/// Artist tag (genre/descriptor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistTag {
    /// Tag name (e.g., "rock", "electronic")
    pub name: String,
    /// Tag count/weight
    pub count: Option<i32>,
    /// URL to Last.fm tag page
    pub url: Option<String>,
}

// Internal response types for deserialization

#[derive(Debug, Deserialize)]
pub(crate) struct SimilarArtistsResponse {
    pub similarartists: SimilarArtistsWrapper,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SimilarArtistsWrapper {
    pub artist: Vec<RawSimilarArtist>,
    #[serde(rename = "@attr")]
    #[allow(dead_code)] // Required for serde deserialization, not used in code
    pub attr: Option<SimilarArtistsAttr>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Required for serde deserialization
pub(crate) struct SimilarArtistsAttr {
    pub artist: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawSimilarArtist {
    pub name: String,
    #[serde(default)]
    pub mbid: Option<String>,
    #[serde(rename = "match")]
    pub match_score: String,
    pub url: Option<String>,
}

impl From<RawSimilarArtist> for SimilarArtist {
    fn from(raw: RawSimilarArtist) -> Self {
        let parsed: f64 = raw.match_score.parse().unwrap_or_else(|e| {
            tracing::warn!(
                artist = %raw.name,
                raw_score = %raw.match_score,
                error = %e,
                "Failed to parse match_score, defaulting to 0.0"
            );
            0.0
        });

        // Validate and clamp the score to [0.0, 1.0] range
        let match_score = if parsed.is_finite() {
            parsed.clamp(0.0, 1.0)
        } else {
            0.0
        };

        Self {
            name: raw.name,
            mbid: raw.mbid.filter(|s| !s.is_empty()),
            match_score,
            url: raw.url,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct TopTagsResponse {
    pub toptags: TopTagsWrapper,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TopTagsWrapper {
    pub tag: Vec<RawArtistTag>,
    #[serde(rename = "@attr")]
    #[allow(dead_code)] // Required for serde deserialization, not used in code
    pub attr: Option<TopTagsAttr>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Required for serde deserialization
pub(crate) struct TopTagsAttr {
    pub artist: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawArtistTag {
    pub name: String,
    pub count: Option<i32>,
    pub url: Option<String>,
}

impl From<RawArtistTag> for ArtistTag {
    fn from(raw: RawArtistTag) -> Self {
        Self {
            name: raw.name,
            count: raw.count,
            url: raw.url,
        }
    }
}

/// Last.fm API error response
#[derive(Debug, Deserialize)]
pub(crate) struct ErrorResponse {
    pub error: i32,
    pub message: String,
}
