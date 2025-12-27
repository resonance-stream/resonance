//! Ollama AI configuration types

use crate::{get_env_or_default, parse_env, ConfigResult};

/// Ollama AI service configuration
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Ollama server URL
    pub url: String,

    /// LLM model for chat/generation (e.g., mistral, llama2)
    pub model: String,

    /// Embedding model for vector search (e.g., nomic-embed-text)
    pub embedding_model: String,

    /// Request timeout in seconds
    pub timeout_secs: u64,

    /// Maximum tokens for generation
    pub max_tokens: u32,

    /// Temperature for generation (0.0 - 1.0)
    pub temperature: f32,
}

impl OllamaConfig {
    /// Load Ollama configuration from environment variables
    pub fn from_env() -> ConfigResult<Self> {
        Ok(Self {
            url: get_env_or_default("OLLAMA_URL", "http://localhost:11434"),
            model: get_env_or_default("OLLAMA_MODEL", "mistral"),
            embedding_model: get_env_or_default("EMBEDDING_MODEL", "nomic-embed-text"),
            timeout_secs: parse_env("OLLAMA_TIMEOUT", 60)?,
            max_tokens: parse_env("OLLAMA_MAX_TOKENS", 2048)?,
            temperature: parse_env("OLLAMA_TEMPERATURE", 0.7)?,
        })
    }

    /// Create a configuration with a custom URL (useful for testing)
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            model: "mistral".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            timeout_secs: 60,
            max_tokens: 2048,
            temperature: 0.7,
        }
    }

    /// Get the full URL for the generation endpoint
    pub fn generate_url(&self) -> String {
        format!("{}/api/generate", self.url.trim_end_matches('/'))
    }

    /// Get the full URL for the embeddings endpoint
    pub fn embeddings_url(&self) -> String {
        format!("{}/api/embeddings", self.url.trim_end_matches('/'))
    }

    /// Get the full URL for the chat endpoint
    pub fn chat_url(&self) -> String {
        format!("{}/api/chat", self.url.trim_end_matches('/'))
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:11434".to_string(),
            model: "mistral".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            timeout_secs: 60,
            max_tokens: 2048,
            temperature: 0.7,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OllamaConfig::default();
        assert_eq!(config.url, "http://localhost:11434");
        assert_eq!(config.model, "mistral");
        assert_eq!(config.embedding_model, "nomic-embed-text");
    }

    #[test]
    fn test_with_url() {
        let config = OllamaConfig::with_url("http://ollama:11434");
        assert_eq!(config.url, "http://ollama:11434");
    }

    #[test]
    fn test_endpoint_urls() {
        let config = OllamaConfig::default();
        assert_eq!(config.generate_url(), "http://localhost:11434/api/generate");
        assert_eq!(config.embeddings_url(), "http://localhost:11434/api/embeddings");
        assert_eq!(config.chat_url(), "http://localhost:11434/api/chat");
    }

    #[test]
    fn test_endpoint_urls_with_trailing_slash() {
        let config = OllamaConfig::with_url("http://localhost:11434/");
        assert_eq!(config.generate_url(), "http://localhost:11434/api/generate");
    }
}
