//! Ollama API client for Resonance AI features
//!
//! This crate provides a client for interacting with the Ollama API
//! for generating embeddings, text, and chat completions.
//!
//! # Requirements
//!
//! - Ollama must be running and accessible at the configured URL
//! - Required models must be pulled before use:
//!   ```bash
//!   ollama pull mistral
//!   ollama pull nomic-embed-text
//!   ```
//!
//! # Thread Safety
//!
//! `OllamaClient` is `Clone + Send + Sync` and can be safely shared
//! across threads. It uses a shared HTTP client connection pool.
//!
//! # Example
//!
//! ```no_run
//! use resonance_ollama_client::OllamaClient;
//! use resonance_shared_config::OllamaConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OllamaConfig::default();
//! let client = OllamaClient::new(&config)?;
//!
//! // Generate embeddings
//! let embedding = client.generate_embedding("Hello, world!").await?;
//! println!("Embedding dimensions: {}", embedding.len());
//!
//! // Generate text
//! let response = client.generate("What is the capital of France?").await?;
//! println!("Response: {}", response);
//!
//! // Chat
//! use resonance_ollama_client::ChatMessage;
//! let messages = vec![
//!     ChatMessage::system("You are a helpful assistant."),
//!     ChatMessage::user("Hello!"),
//! ];
//! let response = client.chat(messages).await?;
//! println!("Chat response: {}", response);
//!
//! // Batch embeddings with concurrency
//! let texts = vec!["text1".to_string(), "text2".to_string()];
//! let embeddings = client.generate_embeddings_batch(texts, 4).await?;
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
mod models;

pub use client::OllamaClient;
pub use error::{OllamaError, OllamaResult};
pub use models::{
    ChatMessage, ChatRequest, ChatResponse, ChatRole, EmbeddingRequest, EmbeddingResponse,
    EnergyLevel, GenerateOptions, GenerateRequest, GenerateResponse, ListModelsResponse, ModelInfo,
    MoodAnalysis, Valence,
};

/// Expected embedding dimension for nomic-embed-text
pub const EMBEDDING_DIMENSION: usize = 768;

/// Validate that an embedding has the expected dimension
pub fn validate_embedding_dimension(embedding: &[f32]) -> Result<(), OllamaError> {
    if embedding.len() != EMBEDDING_DIMENSION {
        return Err(OllamaError::DimensionMismatch {
            expected: EMBEDDING_DIMENSION,
            actual: embedding.len(),
        });
    }
    Ok(())
}
