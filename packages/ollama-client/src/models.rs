//! Request and response types for Ollama API

use serde::{Deserialize, Serialize};

/// Request for generating embeddings
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    /// Model to use for embeddings
    pub model: String,
    /// Text to generate embeddings for
    pub prompt: String,
}

/// Response from embedding generation
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResponse {
    /// Generated embedding vector
    pub embedding: Vec<f32>,
}

/// Request for text generation
#[derive(Debug, Clone, Serialize)]
pub struct GenerateRequest {
    /// Model to use
    pub model: String,
    /// Prompt text
    pub prompt: String,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
    /// Generation options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerateOptions>,
}

/// Options for text generation
#[derive(Debug, Clone, Serialize, Default)]
pub struct GenerateOptions {
    /// Temperature (0.0 - 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<u32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
}

/// Response from text generation (non-streaming)
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    /// Generated text
    pub response: String,
    /// Whether generation is complete
    #[serde(default)]
    pub done: bool,
    /// Total duration in nanoseconds
    #[serde(default)]
    pub total_duration: Option<u64>,
    /// Tokens generated
    #[serde(default)]
    pub eval_count: Option<u32>,
}

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// A single chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message sender
    pub role: ChatRole,
    /// Content of the message
    pub content: String,
}

impl ChatMessage {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
        }
    }
}

/// Request for chat completion
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// Model to use
    pub model: String,
    /// Chat messages
    pub messages: Vec<ChatMessage>,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
    /// Generation options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerateOptions>,
}

/// Response from chat completion
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    /// The assistant's response message
    pub message: ChatMessage,
    /// Whether generation is complete
    #[serde(default)]
    pub done: bool,
    /// Total duration in nanoseconds
    #[serde(default)]
    pub total_duration: Option<u64>,
    /// Tokens generated
    #[serde(default)]
    pub eval_count: Option<u32>,
}

/// Response from listing models
#[derive(Debug, Clone, Deserialize)]
pub struct ListModelsResponse {
    /// Available models
    pub models: Vec<ModelInfo>,
}

/// Information about a model
#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    /// Model name
    pub name: String,
    /// Model size in bytes
    #[serde(default)]
    pub size: u64,
    /// Model digest
    #[serde(default)]
    pub digest: Option<String>,
}

/// Energy level for mood analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EnergyLevel {
    Low,
    #[default]
    Medium,
    High,
}

/// Emotional valence for mood analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Valence {
    Negative,
    #[default]
    Neutral,
    Positive,
}

/// Result of mood analysis from LLM
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MoodAnalysis {
    /// Mood tags (e.g., ["energetic", "happy", "uplifting"])
    pub moods: Vec<String>,
    /// Energy level
    #[serde(default)]
    pub energy: EnergyLevel,
    /// Emotional valence
    #[serde(default)]
    pub valence: Valence,
    /// Brief description
    #[serde(default)]
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_constructors() {
        let system = ChatMessage::system("You are a helpful assistant");
        assert_eq!(system.role, ChatRole::System);

        let user = ChatMessage::user("Hello!");
        assert_eq!(user.role, ChatRole::User);

        let assistant = ChatMessage::assistant("Hi there!");
        assert_eq!(assistant.role, ChatRole::Assistant);
    }

    #[test]
    fn test_embedding_request_serialization() {
        let request = EmbeddingRequest {
            model: "nomic-embed-text".to_string(),
            prompt: "test text".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("nomic-embed-text"));
        assert!(json.contains("test text"));
    }

    #[test]
    fn test_mood_analysis_deserialization() {
        let json = r#"{"moods": ["happy", "energetic"], "energy": "high", "valence": "positive", "description": "Upbeat track"}"#;
        let analysis: MoodAnalysis = serde_json::from_str(json).unwrap();
        assert_eq!(analysis.moods, vec!["happy", "energetic"]);
        assert_eq!(analysis.energy, EnergyLevel::High);
        assert_eq!(analysis.valence, Valence::Positive);
    }

    #[test]
    fn test_mood_analysis_defaults() {
        let json = r#"{"moods": ["calm"]}"#;
        let analysis: MoodAnalysis = serde_json::from_str(json).unwrap();
        assert_eq!(analysis.energy, EnergyLevel::Medium);
        assert_eq!(analysis.valence, Valence::Neutral);
    }
}
