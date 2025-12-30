//! Last.fm API client for Resonance
//!
//! This crate provides a client for the Last.fm API, enabling:
//! - Similar artist discovery
//! - Artist tag retrieval
//!
//! # Example
//!
//! ```rust,no_run
//! use resonance_lastfm_client::LastfmClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = LastfmClient::new("your_api_key")?;
//!
//! // Get similar artists
//! let similar = client.get_similar_artists("Radiohead", Some(10)).await?;
//! for artist in similar {
//!     println!("{}: {:.2}", artist.name, artist.match_score);
//! }
//!
//! // Get artist tags
//! let tags = client.get_artist_tags("Radiohead").await?;
//! for tag in tags {
//!     println!("{}", tag.name);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Environment Variables
//!
//! - `LASTFM_API_KEY`: API key for Last.fm (required)

mod client;
mod error;
mod models;

pub use client::{ApiKeyStatus, LastfmClient};
pub use error::{LastfmError, LastfmResult};
pub use models::{ArtistTag, SimilarArtist};
