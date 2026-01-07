//! Chat service for AI assistant functionality
//!
//! This module provides the core AI chat functionality using Ollama with the Ministral model,
//! supporting native function calling for music library operations.

// Allow dead_code - WebSocket integration (Phase 4) will consume this service
#![allow(dead_code)]

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::models::chat::{
    ChatConversation, ChatMessage, ChatRole, ContextSnapshot, CreateChatMessage,
    CreateConversation, ToolCall, ToolCallFunction,
};
use crate::repositories::ChatRepository;
use crate::services::search::SearchService;
use crate::services::similarity::SimilarityService;
use resonance_ollama_client::OllamaClient;
use resonance_shared_config::OllamaConfig;

/// Chat service errors
#[derive(Debug, Error)]
pub enum ChatError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("conversation not found: {0}")]
    ConversationNotFound(Uuid),

    #[error("ollama request failed: {0}")]
    OllamaRequest(#[from] reqwest::Error),

    #[error("ollama response error: {0}")]
    OllamaResponse(String),

    #[error("json serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("tool execution failed: {tool_name}: {message}")]
    ToolExecution { tool_name: String, message: String },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("operation timeout")]
    Timeout,
}

// ==================== ApiError Integration ====================

impl From<ChatError> for crate::error::ApiError {
    fn from(err: ChatError) -> Self {
        match err {
            ChatError::Database(e) => crate::error::ApiError::Database(e),
            ChatError::ConversationNotFound(id) => crate::error::ApiError::NotFound {
                resource_type: "conversation",
                id: id.to_string(),
            },
            ChatError::OllamaRequest(e) => crate::error::ApiError::AiService(e.to_string()),
            ChatError::OllamaResponse(msg) => crate::error::ApiError::AiService(msg),
            ChatError::Serialization(e) => crate::error::ApiError::Serialization(e),
            ChatError::ToolExecution { tool_name, message } => crate::error::ApiError::AiService(
                format!("Tool '{}' failed: {}", tool_name, message),
            ),
            ChatError::InvalidInput(msg) => crate::error::ApiError::ValidationError(msg),
            ChatError::Timeout => {
                crate::error::ApiError::AiService("Operation timed out".to_string())
            }
        }
    }
}

/// Result type for chat operations
pub type ChatResult<T> = Result<T, ChatError>;

// ==================== Ollama API Types ====================

/// Request body for Ollama chat API
#[derive(Debug, Clone, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
    stream: bool,
    options: OllamaOptions,
}

/// Options for Ollama generation
#[derive(Debug, Clone, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
}

/// Message format for Ollama API
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

/// Tool definition for Ollama
#[derive(Debug, Clone, Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaToolFunction,
}

/// Function definition for tool calling
#[derive(Debug, Clone, Serialize)]
struct OllamaToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// Tool call from Ollama response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OllamaToolCallFunction,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaToolCallFunction {
    name: String,
    arguments: String,
}

/// Response from Ollama chat API
#[derive(Debug, Clone, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    done: bool,
    #[serde(default)]
    done_reason: Option<String>,
}

// ==================== User Context ====================

/// User context for AI assistant
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: Uuid,
    pub track_count: i64,
    pub artist_count: i64,
    pub album_count: i64,
    pub playlist_count: i64,
    pub top_genres: Vec<String>,
    pub current_track_id: Option<Uuid>,
    pub current_track_title: Option<String>,
}

impl From<&UserContext> for ContextSnapshot {
    fn from(ctx: &UserContext) -> Self {
        ContextSnapshot {
            track_count: ctx.track_count,
            artist_count: ctx.artist_count,
            album_count: ctx.album_count,
            playlist_count: ctx.playlist_count,
            top_genres: ctx.top_genres.clone(),
            current_track_id: ctx.current_track_id,
            current_track_title: ctx.current_track_title.clone(),
        }
    }
}

// ==================== Tool Execution Result ====================

/// Result of executing a tool
#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    /// The tool call ID this result corresponds to
    pub tool_call_id: String,
    /// The result content (JSON string)
    pub content: String,
    /// Whether this was an error
    pub is_error: bool,
}

/// Action for the frontend to execute
#[derive(Debug, Clone, Serialize)]
pub struct ChatAction {
    /// Action type (play_track, add_to_queue, create_playlist, etc.)
    pub action_type: String,
    /// Action data
    pub data: serde_json::Value,
}

// ==================== Streaming Events ====================

/// Events emitted during streaming chat responses
///
/// These events are sent through an mpsc channel to allow real-time
/// token-by-token streaming to WebSocket clients.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A token (partial content) from the AI response
    Token(String),

    /// A tool call is starting
    ToolCallStart {
        /// Name of the tool being called
        tool_name: String,
        /// Unique ID for this tool call
        call_id: String,
    },

    /// A tool call has completed
    ToolCallComplete {
        /// The tool call ID
        call_id: String,
        /// Result of the tool execution (JSON string)
        result: String,
    },

    /// Streaming is complete
    Complete {
        /// ID of the saved message
        message_id: Uuid,
        /// Full response text
        full_response: String,
        /// Actions to execute on the frontend
        actions: Vec<ChatAction>,
    },

    /// An error occurred during streaming
    Error {
        /// Error message
        message: String,
        /// Error code for categorization
        code: StreamErrorCode,
    },
}

/// Error codes for streaming errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamErrorCode {
    /// Database operation failed
    Database,
    /// Conversation not found
    ConversationNotFound,
    /// Ollama request failed
    OllamaRequest,
    /// Ollama response error
    OllamaResponse,
    /// JSON serialization error
    Serialization,
    /// Tool execution failed
    ToolExecution,
    /// Invalid input
    InvalidInput,
    /// Operation timeout
    Timeout,
}

impl StreamEvent {
    /// Create an error event from a ChatError
    pub fn from_error(err: &ChatError) -> Self {
        let (message, code) = match err {
            ChatError::Database(e) => (e.to_string(), StreamErrorCode::Database),
            ChatError::ConversationNotFound(id) => (
                format!("Conversation not found: {}", id),
                StreamErrorCode::ConversationNotFound,
            ),
            ChatError::OllamaRequest(e) => (e.to_string(), StreamErrorCode::OllamaRequest),
            ChatError::OllamaResponse(msg) => (msg.clone(), StreamErrorCode::OllamaResponse),
            ChatError::Serialization(e) => (e.to_string(), StreamErrorCode::Serialization),
            ChatError::ToolExecution { tool_name, message } => (
                format!("Tool '{}' failed: {}", tool_name, message),
                StreamErrorCode::ToolExecution,
            ),
            ChatError::InvalidInput(msg) => (msg.clone(), StreamErrorCode::InvalidInput),
            ChatError::Timeout => ("Operation timed out".to_string(), StreamErrorCode::Timeout),
        };
        StreamEvent::Error { message, code }
    }
}

// ==================== Constants ====================

/// Maximum user message length in characters
const MAX_MESSAGE_LENGTH: usize = 10_000;

/// Maximum messages to include in AI context
const MAX_CONTEXT_MESSAGES: i64 = 20;

/// Maximum tool calling iterations to prevent infinite loops
const MAX_TOOL_ITERATIONS: usize = 5;

/// Total operation timeout multiplier (timeout_secs * this value)
const TOTAL_TIMEOUT_MULTIPLIER: u64 = 2;

/// Channel capacity for streaming events
const STREAM_CHANNEL_CAPACITY: usize = 100;

// ==================== Chat Service ====================

/// Service for AI chat functionality
///
/// This service is Clone-able (cheap due to Arc-backed internals) to support
/// spawning background tasks for streaming responses.
#[derive(Clone)]
pub struct ChatService {
    repository: ChatRepository,
    http_client: Client,
    config: OllamaConfig,
    /// Search service for semantic and mood-based search
    search_service: SearchService,
    /// Similarity service for track recommendations
    similarity_service: SimilarityService,
    /// Ollama client for generating embeddings
    ollama_client: Option<OllamaClient>,
}

impl ChatService {
    /// Create a new ChatService with configured HTTP client and AI services
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `config` - Ollama configuration
    /// * `search_service` - Service for semantic and mood-based search
    /// * `similarity_service` - Service for finding similar tracks
    /// * `ollama_client` - Optional Ollama client for generating embeddings
    pub fn new(
        pool: PgPool,
        config: OllamaConfig,
        search_service: SearchService,
        similarity_service: SimilarityService,
        ollama_client: Option<OllamaClient>,
    ) -> Self {
        let http_client = Client::builder()
            .pool_max_idle_per_host(5)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            repository: ChatRepository::new(pool),
            http_client,
            config,
            search_service,
            similarity_service,
            ollama_client,
        }
    }

    /// Create a new conversation
    #[instrument(skip(self))]
    pub async fn create_conversation(
        &self,
        user_id: Uuid,
        title: Option<String>,
    ) -> ChatResult<ChatConversation> {
        let input = CreateConversation { user_id, title };
        Ok(self.repository.create_conversation(input).await?)
    }

    /// Get a conversation by ID
    #[instrument(skip(self))]
    pub async fn get_conversation(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> ChatResult<ChatConversation> {
        self.repository
            .find_conversation_by_id(conversation_id, user_id)
            .await?
            .ok_or(ChatError::ConversationNotFound(conversation_id))
    }

    /// List conversations for a user
    #[instrument(skip(self))]
    pub async fn list_conversations(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> ChatResult<Vec<ChatConversation>> {
        Ok(self
            .repository
            .find_conversations_by_user(user_id, limit, offset)
            .await?)
    }

    /// Delete a conversation (soft delete)
    #[instrument(skip(self))]
    pub async fn delete_conversation(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> ChatResult<bool> {
        Ok(self
            .repository
            .delete_conversation(conversation_id, user_id)
            .await?)
    }

    /// Get messages for a conversation
    #[instrument(skip(self))]
    pub async fn get_messages(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        limit: i64,
    ) -> ChatResult<Vec<ChatMessage>> {
        Ok(self
            .repository
            .get_messages(conversation_id, user_id, limit)
            .await?)
    }

    /// Send a user message and get AI response
    ///
    /// This method:
    /// 1. Validates the message input
    /// 2. Creates/validates the conversation
    /// 3. Saves the user message
    /// 4. Builds context and sends to Ollama
    /// 5. Handles tool calling loop if needed
    /// 6. Saves and returns the assistant response
    #[instrument(skip(self, context))]
    pub async fn send_message(
        &self,
        conversation_id: Option<Uuid>,
        user_id: Uuid,
        message: String,
        context: &UserContext,
    ) -> ChatResult<(ChatConversation, ChatMessage, Vec<ChatAction>)> {
        // Validate message input
        if message.len() > MAX_MESSAGE_LENGTH {
            return Err(ChatError::InvalidInput(format!(
                "Message too long: {} characters (max {})",
                message.len(),
                MAX_MESSAGE_LENGTH
            )));
        }

        let message = message.trim().to_string();
        if message.is_empty() {
            return Err(ChatError::InvalidInput(
                "Message cannot be empty".to_string(),
            ));
        }

        // Create or get conversation
        let conversation = match conversation_id {
            Some(id) => self.get_conversation(id, user_id).await?,
            None => {
                // Generate title from first few words of message
                let title = message
                    .split_whitespace()
                    .take(5)
                    .collect::<Vec<_>>()
                    .join(" ");
                self.create_conversation(user_id, Some(title)).await?
            }
        };

        // Save user message
        let user_message = self
            .repository
            .add_message(CreateChatMessage {
                conversation_id: conversation.id,
                user_id,
                role: ChatRole::User,
                content: Some(message.clone()),
                tool_calls: None,
                tool_call_id: None,
                context_snapshot: Some(context.into()),
                model_used: None,
                token_count: None,
            })
            .await?;

        info!(
            conversation_id = %conversation.id,
            message_id = %user_message.id,
            "User message saved"
        );

        // Get conversation history for context
        let history = self
            .repository
            .get_recent_messages(conversation.id, user_id, MAX_CONTEXT_MESSAGES)
            .await?;

        // Send to Ollama and handle tool calling loop
        let (response_content, tool_calls_made, actions) =
            self.chat_with_ollama(&history, context).await?;

        // Save assistant response
        let assistant_message = self
            .repository
            .add_message(CreateChatMessage {
                conversation_id: conversation.id,
                user_id,
                role: ChatRole::Assistant,
                content: Some(response_content),
                tool_calls: if tool_calls_made.is_empty() {
                    None
                } else {
                    Some(tool_calls_made)
                },
                tool_call_id: None,
                context_snapshot: None,
                model_used: Some(self.config.model.clone()),
                token_count: None,
            })
            .await?;

        info!(
            conversation_id = %conversation.id,
            message_id = %assistant_message.id,
            actions_count = actions.len(),
            "Assistant response saved"
        );

        Ok((conversation, assistant_message, actions))
    }

    /// Send a user message and stream the AI response token by token
    ///
    /// This method returns immediately with a conversation ID and a receiver channel.
    /// The actual streaming happens in a spawned background task.
    ///
    /// # Returns
    /// - `conversation_id` - The ID of the conversation (new or existing)
    /// - `receiver` - Channel receiver for streaming events
    ///
    /// # Events
    /// The receiver will emit:
    /// - `StreamEvent::Token(String)` - Each token as it's generated
    /// - `StreamEvent::ToolCallStart` - When a tool call begins
    /// - `StreamEvent::ToolCallComplete` - When a tool call finishes
    /// - `StreamEvent::Complete` - Final event with message_id and full response
    /// - `StreamEvent::Error` - If an error occurs
    #[instrument(skip(self, context))]
    pub async fn send_message_streaming(
        &self,
        conversation_id: Option<Uuid>,
        user_id: Uuid,
        message: String,
        context: UserContext,
    ) -> ChatResult<(Uuid, mpsc::Receiver<StreamEvent>)> {
        // Validate message input
        if message.len() > MAX_MESSAGE_LENGTH {
            return Err(ChatError::InvalidInput(format!(
                "Message too long: {} characters (max {})",
                message.len(),
                MAX_MESSAGE_LENGTH
            )));
        }

        let message = message.trim().to_string();
        if message.is_empty() {
            return Err(ChatError::InvalidInput(
                "Message cannot be empty".to_string(),
            ));
        }

        // Create or get conversation
        let conversation = match conversation_id {
            Some(id) => self.get_conversation(id, user_id).await?,
            None => {
                // Generate title from first few words of message
                let title = message
                    .split_whitespace()
                    .take(5)
                    .collect::<Vec<_>>()
                    .join(" ");
                self.create_conversation(user_id, Some(title)).await?
            }
        };

        // Save user message
        let user_message = self
            .repository
            .add_message(CreateChatMessage {
                conversation_id: conversation.id,
                user_id,
                role: ChatRole::User,
                content: Some(message.clone()),
                tool_calls: None,
                tool_call_id: None,
                context_snapshot: Some((&context).into()),
                model_used: None,
                token_count: None,
            })
            .await?;

        info!(
            conversation_id = %conversation.id,
            message_id = %user_message.id,
            "User message saved, starting streaming response"
        );

        // Create channel for streaming events
        let (tx, rx) = mpsc::channel(STREAM_CHANNEL_CAPACITY);

        // Clone self and context for the spawned task
        let service = self.clone();
        let conv_id = conversation.id;

        // Spawn background task to handle streaming
        tokio::spawn(async move {
            service
                .stream_with_tool_calls(conv_id, user_id, context, tx)
                .await;
        });

        Ok((conversation.id, rx))
    }

    /// Internal method that performs streaming with tool call handling
    ///
    /// This runs in a spawned task and sends events through the channel.
    async fn stream_with_tool_calls(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        context: UserContext,
        tx: mpsc::Sender<StreamEvent>,
    ) {
        let total_timeout = std::time::Duration::from_secs(
            self.config
                .timeout_secs
                .saturating_mul(TOTAL_TIMEOUT_MULTIPLIER),
        );

        let result = tokio::time::timeout(
            total_timeout,
            self.stream_with_tool_calls_inner(conversation_id, user_id, &context, &tx),
        )
        .await;

        match result {
            Ok(Ok(())) => {
                // Successfully completed, Complete event already sent
            }
            Ok(Err(e)) => {
                // Error during streaming
                let _ = tx.send(StreamEvent::from_error(&e)).await;
            }
            Err(_) => {
                // Timeout
                let _ = tx
                    .send(StreamEvent::Error {
                        message: "Operation timed out".to_string(),
                        code: StreamErrorCode::Timeout,
                    })
                    .await;
            }
        }
    }

    /// Inner implementation of streaming with tool calls
    async fn stream_with_tool_calls_inner(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        context: &UserContext,
        tx: &mpsc::Sender<StreamEvent>,
    ) -> ChatResult<()> {
        // Get conversation history for context
        let history = self
            .repository
            .get_recent_messages(conversation_id, user_id, MAX_CONTEXT_MESSAGES)
            .await?;

        let system_prompt = self.build_system_prompt(context);

        // Convert history to Ollama format (without tool definitions for streaming)
        let mut messages: Vec<resonance_ollama_client::ChatMessage> =
            vec![resonance_ollama_client::ChatMessage::system(system_prompt)];

        for msg in &history {
            let role = match msg.role {
                ChatRole::User => resonance_ollama_client::ChatRole::User,
                ChatRole::Assistant => resonance_ollama_client::ChatRole::Assistant,
                ChatRole::System => resonance_ollama_client::ChatRole::System,
                ChatRole::Tool => continue, // Skip tool messages for now
            };
            messages.push(resonance_ollama_client::ChatMessage {
                role,
                content: msg.content.clone().unwrap_or_default(),
            });
        }

        // Get Ollama client for streaming
        let Some(ref ollama) = self.ollama_client else {
            return Err(ChatError::OllamaResponse(
                "Ollama client not configured for streaming".to_string(),
            ));
        };

        // Start streaming
        let mut stream = ollama
            .chat_stream(messages, None)
            .await
            .map_err(|e| ChatError::OllamaResponse(format!("Failed to start stream: {}", e)))?;

        let mut full_response = String::new();

        // Process stream chunks
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let content = &chunk.message.content;

                    // Send token event if there's content
                    if !content.is_empty() {
                        full_response.push_str(content);

                        if tx.send(StreamEvent::Token(content.clone())).await.is_err() {
                            // Receiver dropped, client disconnected
                            debug!("Stream receiver dropped, stopping");
                            return Ok(());
                        }
                    }

                    // Check if streaming is done
                    if chunk.done {
                        debug!(
                            content_len = full_response.len(),
                            done_reason = ?chunk.done_reason,
                            "Stream complete"
                        );
                        break;
                    }
                }
                Err(e) => {
                    return Err(ChatError::OllamaResponse(format!("Stream error: {}", e)));
                }
            }
        }

        // NOTE: Tool calls during streaming are not yet supported.
        // The Ollama streaming chat API doesn't include tool_calls in the same
        // format as the non-streaming API. When Ollama adds support for streaming
        // tool calls, this method should be updated to:
        // 1. Detect tool_calls in the final chunk
        // 2. Send StreamEvent::ToolCallStart for each tool
        // 3. Execute the tool
        // 4. Send StreamEvent::ToolCallComplete
        // 5. Resume streaming with the tool result
        let all_tool_calls: Vec<ToolCall> = Vec::new();
        let all_actions: Vec<ChatAction> = Vec::new();

        // Save the assistant message
        let assistant_message = self
            .repository
            .add_message(CreateChatMessage {
                conversation_id,
                user_id,
                role: ChatRole::Assistant,
                content: Some(full_response.clone()),
                tool_calls: if all_tool_calls.is_empty() {
                    None
                } else {
                    Some(all_tool_calls)
                },
                tool_call_id: None,
                context_snapshot: None,
                model_used: Some(self.config.model.clone()),
                token_count: None,
            })
            .await?;

        info!(
            conversation_id = %conversation_id,
            message_id = %assistant_message.id,
            response_len = full_response.len(),
            actions_count = all_actions.len(),
            "Streaming response saved"
        );

        // Send complete event
        let _ = tx
            .send(StreamEvent::Complete {
                message_id: assistant_message.id,
                full_response,
                actions: all_actions,
            })
            .await;

        Ok(())
    }

    /// Chat with Ollama, handling tool calling loop with total operation timeout
    #[instrument(skip(self, history, context))]
    async fn chat_with_ollama(
        &self,
        history: &[ChatMessage],
        context: &UserContext,
    ) -> ChatResult<(String, Vec<ToolCall>, Vec<ChatAction>)> {
        let total_timeout = std::time::Duration::from_secs(
            self.config
                .timeout_secs
                .saturating_mul(TOTAL_TIMEOUT_MULTIPLIER),
        );

        tokio::time::timeout(total_timeout, self.chat_with_ollama_inner(history, context))
            .await
            .map_err(|_| ChatError::Timeout)?
    }

    /// Inner implementation of chat_with_ollama without timeout wrapper
    async fn chat_with_ollama_inner(
        &self,
        history: &[ChatMessage],
        context: &UserContext,
    ) -> ChatResult<(String, Vec<ToolCall>, Vec<ChatAction>)> {
        let system_prompt = self.build_system_prompt(context);
        let tools = self.get_tools();

        // Convert history to Ollama format
        let mut messages: Vec<OllamaMessage> = vec![OllamaMessage {
            role: "system".to_string(),
            content: system_prompt,
            tool_calls: None,
            tool_call_id: None,
        }];

        for msg in history {
            messages.push(OllamaMessage {
                role: msg.role.as_str().to_string(),
                content: msg.content.clone().unwrap_or_default(),
                tool_calls: msg.tool_calls.as_ref().map(|tcs| {
                    tcs.iter()
                        .map(|tc| OllamaToolCall {
                            id: tc.id.clone(),
                            call_type: tc.call_type.clone(),
                            function: OllamaToolCallFunction {
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            },
                        })
                        .collect()
                }),
                tool_call_id: msg.tool_call_id.clone(),
            });
        }

        let mut all_tool_calls = Vec::new();
        let mut all_actions = Vec::new();
        let mut iteration = 0;

        // Tool calling loop
        loop {
            iteration += 1;
            if iteration > MAX_TOOL_ITERATIONS {
                warn!("Max tool calling iterations reached");
                break;
            }

            let request = OllamaChatRequest {
                model: self.config.model.clone(),
                messages: messages.clone(),
                tools: Some(tools.clone()),
                stream: false,
                options: OllamaOptions {
                    temperature: self.config.temperature,
                    num_predict: self.config.max_tokens as i32,
                },
            };

            debug!("Sending request to Ollama");

            let response = self
                .http_client
                .post(self.config.chat_url())
                .json(&request)
                .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let body = match response.text().await {
                    Ok(text) => text,
                    Err(e) => format!("Failed to read error body: {}", e),
                };
                // Log truncated body to avoid flooding logs
                // Use char boundary to avoid panic on multi-byte UTF-8 characters
                const MAX_LOG_BODY: usize = 4096;
                let truncated_body = if body.len() > MAX_LOG_BODY {
                    let mut cut = MAX_LOG_BODY;
                    while cut > 0 && !body.is_char_boundary(cut) {
                        cut -= 1;
                    }
                    format!("{}...[truncated {} bytes]", &body[..cut], body.len() - cut)
                } else {
                    body
                };
                error!(status = %status, body = %truncated_body, "Ollama request failed");
                // Return sanitized error to caller
                return Err(ChatError::OllamaResponse(format!(
                    "AI service unavailable (status {})",
                    status.as_u16()
                )));
            }

            let chat_response: OllamaChatResponse = response.json().await?;

            // Verify response is complete for non-streaming requests
            if !chat_response.done {
                return Err(ChatError::OllamaResponse(
                    chat_response
                        .done_reason
                        .unwrap_or_else(|| "AI response not complete".to_string()),
                ));
            }

            // Check for tool calls
            if let Some(ref tool_calls) = chat_response.message.tool_calls {
                if !tool_calls.is_empty() {
                    debug!(count = tool_calls.len(), "Processing tool calls");

                    // Add assistant's tool call message
                    messages.push(OllamaMessage {
                        role: "assistant".to_string(),
                        content: chat_response.message.content.clone(),
                        tool_calls: Some(tool_calls.clone()),
                        tool_call_id: None,
                    });

                    // Execute each tool and collect results
                    for tool_call in tool_calls {
                        let (result, action) = self.execute_tool(tool_call).await;

                        // Add tool result message
                        messages.push(OllamaMessage {
                            role: "tool".to_string(),
                            content: result.content,
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                        });

                        // Track for storage
                        all_tool_calls.push(ToolCall {
                            id: tool_call.id.clone(),
                            call_type: tool_call.call_type.clone(),
                            function: ToolCallFunction {
                                name: tool_call.function.name.clone(),
                                arguments: tool_call.function.arguments.clone(),
                            },
                        });

                        if let Some(a) = action {
                            all_actions.push(a);
                        }
                    }

                    continue;
                }
            }

            // No tool calls - this is the final response
            return Ok((chat_response.message.content, all_tool_calls, all_actions));
        }

        // If we hit max iterations, return whatever we have
        Ok((
            "I apologize, but I'm having trouble completing this request. Please try again."
                .to_string(),
            all_tool_calls,
            all_actions,
        ))
    }

    /// Build the system prompt with user context
    fn build_system_prompt(&self, context: &UserContext) -> String {
        let current_track = context
            .current_track_title
            .as_ref()
            .map(|t| format!("Currently playing: {}", t))
            .unwrap_or_else(|| "Nothing currently playing".to_string());

        let top_genres = if context.top_genres.is_empty() {
            "No listening history yet".to_string()
        } else {
            context.top_genres.join(", ")
        };

        format!(
            r#"You are Resonance AI, a friendly and knowledgeable music assistant for a personal music streaming library.

## User's Library Stats
- Tracks: {}
- Artists: {}
- Albums: {}
- Playlists: {}

## User's Top Genres
{}

## Current Status
{}

## Your Capabilities
You can help users with their music library by:
1. Searching for tracks, albums, and artists
2. Playing music and adding to the queue
3. Creating playlists based on preferences
4. Getting personalized recommendations
5. Answering questions about their music collection

## Guidelines
- Be conversational and helpful
- When asked to play something, use the search function first if you don't have an exact ID
- Suggest relevant music based on the user's tastes
- If you're unsure what the user wants, ask clarifying questions
- Keep responses concise but informative"#,
            context.track_count,
            context.artist_count,
            context.album_count,
            context.playlist_count,
            top_genres,
            current_track
        )
    }

    /// Get tool definitions for function calling
    fn get_tools(&self) -> Vec<OllamaTool> {
        vec![
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaToolFunction {
                    name: "search_library".to_string(),
                    description: "Search the user's music library. Use search_type 'track' for finding specific songs/artists/albums by name, or 'mood' for finding tracks matching a mood or vibe."
                        .to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query - song/artist/album name for 'track' search, or mood descriptors (e.g., 'upbeat', 'relaxing', 'energetic') for 'mood' search"
                            },
                            "search_type": {
                                "type": "string",
                                "enum": ["track", "mood"],
                                "description": "Search mode: 'track' (default) for semantic search using AI embeddings, 'mood' for finding tracks by mood tags like 'happy' or 'energetic'"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum number of results (default: 5, max: 20)"
                            }
                        },
                        "required": ["query"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaToolFunction {
                    name: "play_track".to_string(),
                    description: "Play a specific track by its ID".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "track_id": {
                                "type": "string",
                                "description": "UUID of the track to play"
                            }
                        },
                        "required": ["track_id"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaToolFunction {
                    name: "add_to_queue".to_string(),
                    description: "Add one or more tracks to the playback queue".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "track_ids": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Array of track UUIDs to add to queue"
                            }
                        },
                        "required": ["track_ids"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaToolFunction {
                    name: "create_playlist".to_string(),
                    description: "Create a new playlist with optional initial tracks".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Name for the new playlist"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional description for the playlist"
                            },
                            "track_ids": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional array of track UUIDs to add"
                            }
                        },
                        "required": ["name"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaToolFunction {
                    name: "get_recommendations".to_string(),
                    description: "Get track recommendations similar to a given track. For mood-based search, use search_library with search_type 'mood' instead."
                        .to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "similar_to_track_id": {
                                "type": "string",
                                "description": "Track ID to find similar tracks to (required)"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum number of recommendations (default: 5, max: 20)"
                            }
                        },
                        "required": ["similar_to_track_id"]
                    }),
                },
            },
        ]
    }

    /// Execute a tool call and return the result
    #[instrument(skip(self))]
    async fn execute_tool(&self, tool_call: &OllamaToolCall) -> (ToolResult, Option<ChatAction>) {
        let function_name = &tool_call.function.name;
        let arguments = &tool_call.function.arguments;

        debug!(
            function = %function_name,
            arguments = %arguments,
            "Executing tool"
        );

        // Helper to detect errors via JSON parsing instead of substring search
        let has_json_error = |content: &str| -> bool {
            serde_json::from_str::<serde_json::Value>(content)
                .map(|v| v.get("error").is_some())
                .unwrap_or(true) // Treat parse failures as errors
        };

        let (content, action, is_error) = match function_name.as_str() {
            "search_library" => {
                let (c, a) = self.tool_search_library(arguments).await;
                let err = has_json_error(&c);
                (c, a, err)
            }
            "play_track" => {
                let (c, a) = self.tool_play_track(arguments);
                let err = has_json_error(&c);
                (c, a, err)
            }
            "add_to_queue" => {
                let (c, a) = self.tool_add_to_queue(arguments);
                let err = has_json_error(&c);
                (c, a, err)
            }
            "create_playlist" => {
                let (c, a) = self.tool_create_playlist(arguments);
                let err = has_json_error(&c);
                (c, a, err)
            }
            "get_recommendations" => {
                let (c, a) = self.tool_get_recommendations(arguments).await;
                let err = has_json_error(&c);
                (c, a, err)
            }
            _ => (
                serde_json::json!({
                    "error": format!("Unknown function: {}", function_name)
                })
                .to_string(),
                None,
                true,
            ),
        };

        (
            ToolResult {
                tool_call_id: tool_call.id.clone(),
                content,
                is_error,
            },
            action,
        )
    }

    // ==================== Tool Implementations ====================

    /// Parse mood tags from comma-separated input string
    ///
    /// Splits by comma, trims whitespace, converts to lowercase, and filters empty strings.
    fn parse_mood_tags(input: &str) -> Vec<String> {
        input
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Format search results as JSON values for consistent response structure
    fn format_search_results(
        tracks: &[crate::services::search::ScoredTrack],
    ) -> Vec<serde_json::Value> {
        tracks
            .iter()
            .map(|t| {
                serde_json::json!({
                    "track_id": t.track_id.to_string(),
                    "title": &t.title,
                    "artist_name": t.artist_name.as_deref().unwrap_or(""),
                    "album_title": t.album_title.as_deref().unwrap_or(""),
                    "score": t.score
                })
            })
            .collect()
    }

    /// Search library tool implementation using semantic search or mood-based search
    ///
    /// Supports two search modes:
    /// - `mood`: Searches tracks by mood tags (e.g., "happy", "energetic", "melancholic")
    /// - `track`/default: Uses semantic search with AI embeddings to find matching tracks
    #[instrument(skip(self))]
    async fn tool_search_library(&self, arguments: &str) -> (String, Option<ChatAction>) {
        #[derive(Deserialize)]
        struct Args {
            query: String,
            search_type: Option<String>,
            limit: Option<i32>,
        }

        let args: Args = match serde_json::from_str(arguments) {
            Ok(a) => a,
            Err(e) => {
                return (
                    serde_json::json!({ "error": format!("Invalid arguments: {}", e) }).to_string(),
                    None,
                )
            }
        };

        // Validate query is not empty
        let query = args.query.trim();
        if query.is_empty() {
            return (
                serde_json::json!({
                    "error": "Search query cannot be empty"
                })
                .to_string(),
                None,
            );
        }

        // Prevent abuse / excessive embedding work
        const MAX_QUERY_CHARS: usize = 512;
        if query.chars().count() > MAX_QUERY_CHARS {
            return (
                serde_json::json!({
                    "error": "Search query too long",
                    "max_chars": MAX_QUERY_CHARS
                })
                .to_string(),
                None,
            );
        }

        let limit = args.limit.unwrap_or(5).clamp(1, 20); // Clamp between 1 and 20
        let search_type = args
            .search_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("track")
            .to_lowercase();

        // Validate search_type
        if search_type != "track" && search_type != "mood" {
            return (
                serde_json::json!({
                    "error": "Invalid search_type",
                    "allowed": ["track", "mood"]
                })
                .to_string(),
                None,
            );
        }

        // Use mood-based search if search_type is "mood"
        if search_type == "mood" {
            // Parse query as mood tags (could be comma-separated or single mood)
            let moods = Self::parse_mood_tags(query);

            if moods.is_empty() {
                return (
                    serde_json::json!({
                        "error": "No valid mood tags provided",
                        "hint": "Provide moods like 'happy', 'energetic', 'melancholic'"
                    })
                    .to_string(),
                    None,
                );
            }

            match self.search_service.search_by_mood(&moods, limit).await {
                Ok(tracks) => {
                    let results = Self::format_search_results(&tracks);
                    let mut result = serde_json::json!({
                        "results": results,
                        "query": query,
                        "search_type": "mood",
                        "count": tracks.len()
                    });
                    if tracks.is_empty() {
                        result["message"] =
                            serde_json::json!("No tracks found matching the specified mood");
                    }
                    (result.to_string(), None)
                }
                Err(e) => {
                    warn!(error = %e, "Mood search failed");
                    (
                        serde_json::json!({
                            "error": format!("Search failed: {}", e),
                            "query": query
                        })
                        .to_string(),
                        None,
                    )
                }
            }
        } else {
            // Use semantic search with embeddings
            let Some(ref ollama) = self.ollama_client else {
                return (
                    serde_json::json!({
                        "error": "Semantic search not available (Ollama not configured)",
                        "hint": "Try mood-based search instead with search_type: 'mood'"
                    })
                    .to_string(),
                    None,
                );
            };

            // Generate embedding for the query
            let embedding = match ollama.generate_embedding(query).await {
                Ok(emb) => emb,
                Err(e) => {
                    warn!(error = %e, query = %query, "Failed to generate embedding");
                    return (
                        serde_json::json!({
                            "error": format!("Failed to process query: {}", e),
                            "query": query
                        })
                        .to_string(),
                        None,
                    );
                }
            };

            // Search by embedding
            match self
                .search_service
                .search_by_embedding(&embedding, limit)
                .await
            {
                Ok(tracks) => {
                    let results = Self::format_search_results(&tracks);
                    let mut result = serde_json::json!({
                        "results": results,
                        "query": query,
                        "search_type": "semantic",
                        "count": tracks.len()
                    });
                    if tracks.is_empty() {
                        result["message"] =
                            serde_json::json!("No tracks found matching your query");
                    }
                    (result.to_string(), None)
                }
                Err(e) => {
                    warn!(error = %e, "Semantic search failed");
                    (
                        serde_json::json!({
                            "error": format!("Search failed: {}", e),
                            "query": query
                        })
                        .to_string(),
                        None,
                    )
                }
            }
        }
    }

    /// Play track tool implementation
    fn tool_play_track(&self, arguments: &str) -> (String, Option<ChatAction>) {
        #[derive(Deserialize)]
        struct Args {
            track_id: String,
        }

        let args: Args = match serde_json::from_str(arguments) {
            Ok(a) => a,
            Err(e) => {
                return (
                    serde_json::json!({ "error": format!("Invalid arguments: {}", e) }).to_string(),
                    None,
                )
            }
        };

        // Validate UUID format
        let track_uuid = match Uuid::parse_str(&args.track_id) {
            Ok(uuid) => uuid,
            Err(_) => return (
                serde_json::json!({ "error": "Invalid track_id format - must be a valid UUID" })
                    .to_string(),
                None,
            ),
        };

        // Create action for frontend with validated UUID
        let action = ChatAction {
            action_type: "play_track".to_string(),
            data: serde_json::json!({ "track_id": track_uuid.to_string() }),
        };

        let result = serde_json::json!({
            "success": true,
            "action": "play_track",
            "track_id": track_uuid.to_string()
        });

        (result.to_string(), Some(action))
    }

    /// Add to queue tool implementation
    fn tool_add_to_queue(&self, arguments: &str) -> (String, Option<ChatAction>) {
        #[derive(Deserialize)]
        struct Args {
            track_ids: Vec<String>,
        }

        let args: Args = match serde_json::from_str(arguments) {
            Ok(a) => a,
            Err(e) => {
                return (
                    serde_json::json!({ "error": format!("Invalid arguments: {}", e) }).to_string(),
                    None,
                )
            }
        };

        // Validate all UUIDs
        let mut validated_ids = Vec::with_capacity(args.track_ids.len());
        for (i, id) in args.track_ids.iter().enumerate() {
            match Uuid::parse_str(id) {
                Ok(uuid) => validated_ids.push(uuid.to_string()),
                Err(_) => return (
                    serde_json::json!({
                        "error": format!("Invalid track_id at index {} - must be a valid UUID", i)
                    })
                    .to_string(),
                    None,
                ),
            }
        }

        // Create action for frontend with validated UUIDs
        let action = ChatAction {
            action_type: "add_to_queue".to_string(),
            data: serde_json::json!({ "track_ids": validated_ids }),
        };

        let result = serde_json::json!({
            "success": true,
            "action": "add_to_queue",
            "count": validated_ids.len()
        });

        (result.to_string(), Some(action))
    }

    /// Create playlist tool implementation
    fn tool_create_playlist(&self, arguments: &str) -> (String, Option<ChatAction>) {
        #[derive(Deserialize)]
        struct Args {
            name: String,
            description: Option<String>,
            track_ids: Option<Vec<String>>,
        }

        let args: Args = match serde_json::from_str(arguments) {
            Ok(a) => a,
            Err(e) => {
                return (
                    serde_json::json!({ "error": format!("Invalid arguments: {}", e) }).to_string(),
                    None,
                )
            }
        };

        // Validate track IDs if provided
        let validated_track_ids = if let Some(track_ids) = args.track_ids {
            let mut validated = Vec::with_capacity(track_ids.len());
            for (i, id) in track_ids.iter().enumerate() {
                match Uuid::parse_str(id) {
                    Ok(uuid) => validated.push(uuid.to_string()),
                    Err(_) => {
                        return (
                            serde_json::json!({
                                "error": format!("Invalid track_id at index {} - must be a valid UUID", i)
                            })
                            .to_string(),
                            None,
                        )
                    }
                }
            }
            validated
        } else {
            Vec::new()
        };

        // Store name before moving into action
        let name = args.name;

        // Create action for frontend with validated UUIDs
        let action = ChatAction {
            action_type: "create_playlist".to_string(),
            data: serde_json::json!({
                "name": &name,
                "description": args.description,
                "track_ids": validated_track_ids
            }),
        };

        let result = serde_json::json!({
            "success": true,
            "action": "create_playlist",
            "name": name
        });

        (result.to_string(), Some(action))
    }

    /// Format similar tracks as JSON values for consistent response structure
    fn format_similar_tracks(
        tracks: &[crate::services::similarity::SimilarTrack],
    ) -> Vec<serde_json::Value> {
        tracks
            .iter()
            .map(|t| {
                serde_json::json!({
                    "track_id": t.track_id.to_string(),
                    "title": &t.title,
                    "artist_name": t.artist_name.as_deref().unwrap_or(""),
                    "album_title": t.album_title.as_deref().unwrap_or(""),
                    "score": t.score,
                    "similarity_type": format!("{:?}", t.similarity_type).to_lowercase()
                })
            })
            .collect()
    }

    /// Get recommendations tool implementation
    ///
    /// Finds tracks similar to a given track using combined similarity (semantic, acoustic, categorical).
    /// For mood-based searches, use `search_library` with `search_type: "mood"` instead.
    #[instrument(skip(self))]
    async fn tool_get_recommendations(&self, arguments: &str) -> (String, Option<ChatAction>) {
        #[derive(Deserialize)]
        struct Args {
            similar_to_track_id: String,
            limit: Option<i32>,
        }

        let args: Args = match serde_json::from_str(arguments) {
            Ok(a) => a,
            Err(e) => {
                return (
                    serde_json::json!({
                        "error": format!("Invalid arguments: {}", e),
                        "hint": "Provide similar_to_track_id (required). For mood-based search, use search_library with search_type: 'mood'"
                    })
                    .to_string(),
                    None,
                )
            }
        };

        let limit = args.limit.unwrap_or(5).clamp(1, 20); // Clamp between 1 and 20

        // Validate similar_to_track_id UUID format
        let track_uuid = match Uuid::parse_str(&args.similar_to_track_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return (
                    serde_json::json!({
                        "error": "Invalid similar_to_track_id - must be a valid UUID"
                    })
                    .to_string(),
                    None,
                )
            }
        };

        // Find similar tracks using combined similarity
        match self
            .similarity_service
            .find_similar_combined(track_uuid, limit)
            .await
        {
            Ok(similar_tracks) => {
                let results = Self::format_similar_tracks(&similar_tracks);
                let mut result = serde_json::json!({
                    "recommendations": results,
                    "similar_to": track_uuid.to_string(),
                    "recommendation_type": "similar_tracks",
                    "count": similar_tracks.len()
                });
                if similar_tracks.is_empty() {
                    result["message"] =
                        serde_json::json!("No similar tracks found in your library");
                }
                (result.to_string(), None)
            }
            Err(e) => {
                warn!(error = %e, track_id = %track_uuid, "Similarity recommendations failed");
                (
                    serde_json::json!({
                        "error": format!("Failed to get similar tracks: {}", e),
                        "similar_to": track_uuid.to_string()
                    })
                    .to_string(),
                    None,
                )
            }
        }
    }
}

// ==================== User Context Builder ====================

/// Builder for creating user context from database
pub struct UserContextBuilder {
    pool: PgPool,
}

impl UserContextBuilder {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Build user context by querying the database
    #[instrument(skip(self))]
    pub async fn build(&self, user_id: Uuid) -> ChatResult<UserContext> {
        // Get library stats scoped to user's listening history
        let stats: LibraryStats = sqlx::query_as(
            r#"
            WITH user_tracks AS (
                SELECT DISTINCT t.id, t.artist_id, t.album_id
                FROM tracks t
                JOIN queue_history qh ON t.id = qh.track_id
                WHERE qh.user_id = $1
            )
            SELECT
                (SELECT COUNT(*) FROM user_tracks) as track_count,
                (SELECT COUNT(DISTINCT artist_id) FROM user_tracks) as artist_count,
                (SELECT COUNT(DISTINCT album_id) FROM user_tracks) as album_count,
                (SELECT COUNT(*) FROM playlists WHERE user_id = $1 AND deleted_at IS NULL) as playlist_count
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Get top genres from user's listening history
        // For now, use a simplified query - can be enhanced later
        let top_genres: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT unnest(t.genres)
            FROM tracks t
            INNER JOIN queue_history qh ON qh.track_id = t.id AND qh.user_id = $1
            WHERE t.genres IS NOT NULL
            LIMIT 5
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|e| {
            warn!(user_id = %user_id, error = %e, "Failed to fetch top genres");
            Vec::new()
        });

        // Get current track from device presence
        let current_track: Option<CurrentTrack> = sqlx::query_as(
            r#"
            SELECT dp.current_track_id, t.title
            FROM device_presence dp
            LEFT JOIN tracks t ON t.id = dp.current_track_id
            WHERE dp.user_id = $1 AND dp.is_online = true
            ORDER BY dp.last_heartbeat DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(UserContext {
            user_id,
            track_count: stats.track_count.unwrap_or(0),
            artist_count: stats.artist_count.unwrap_or(0),
            album_count: stats.album_count.unwrap_or(0),
            playlist_count: stats.playlist_count.unwrap_or(0),
            top_genres,
            current_track_id: current_track.as_ref().and_then(|ct| ct.current_track_id),
            current_track_title: current_track.and_then(|ct| ct.title),
        })
    }
}

#[derive(sqlx::FromRow)]
struct LibraryStats {
    track_count: Option<i64>,
    artist_count: Option<i64>,
    album_count: Option<i64>,
    playlist_count: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct CurrentTrack {
    current_track_id: Option<Uuid>,
    title: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test ChatService with a lazy pool
    /// Note: This pool is never actually connected, used only for unit testing
    async fn test_service() -> ChatService {
        let pool = sqlx::PgPool::connect_lazy("postgres://test").unwrap();
        ChatService::new(
            pool.clone(),
            OllamaConfig::default(),
            SearchService::new(pool.clone()),
            SimilarityService::new(pool),
            None, // Tests don't exercise AI embedding features
        )
    }

    #[tokio::test]
    async fn test_system_prompt_generation() {
        let context = UserContext {
            user_id: Uuid::new_v4(),
            track_count: 1500,
            artist_count: 250,
            album_count: 150,
            playlist_count: 10,
            top_genres: vec![
                "rock".to_string(),
                "jazz".to_string(),
                "electronic".to_string(),
            ],
            current_track_id: Some(Uuid::new_v4()),
            current_track_title: Some("Bohemian Rhapsody".to_string()),
        };

        let service = test_service().await;
        let prompt = service.build_system_prompt(&context);

        assert!(prompt.contains("Tracks: 1500"));
        assert!(prompt.contains("rock, jazz, electronic"));
        assert!(prompt.contains("Bohemian Rhapsody"));
    }

    #[tokio::test]
    async fn test_tool_definitions() {
        let service = test_service().await;
        let tools = service.get_tools();

        assert_eq!(tools.len(), 5);

        let tool_names: Vec<&str> = tools.iter().map(|t| t.function.name.as_str()).collect();
        assert!(tool_names.contains(&"search_library"));
        assert!(tool_names.contains(&"play_track"));
        assert!(tool_names.contains(&"add_to_queue"));
        assert!(tool_names.contains(&"create_playlist"));
        assert!(tool_names.contains(&"get_recommendations"));
    }

    #[tokio::test]
    async fn test_play_track_action() {
        let service = test_service().await;

        let args = r#"{"track_id": "123e4567-e89b-12d3-a456-426614174000"}"#;
        let (result, action) = service.tool_play_track(args);

        assert!(result.contains("success"));
        assert!(action.is_some());

        let action = action.unwrap();
        assert_eq!(action.action_type, "play_track");
    }

    #[test]
    fn test_context_snapshot_from_user_context() {
        let context = UserContext {
            user_id: Uuid::new_v4(),
            track_count: 100,
            artist_count: 50,
            album_count: 25,
            playlist_count: 5,
            top_genres: vec!["pop".to_string()],
            current_track_id: None,
            current_track_title: None,
        };

        let snapshot: ContextSnapshot = (&context).into();

        assert_eq!(snapshot.track_count, 100);
        assert_eq!(snapshot.artist_count, 50);
        assert_eq!(snapshot.top_genres, vec!["pop".to_string()]);
    }

    #[tokio::test]
    async fn test_play_track_invalid_uuid() {
        let service = test_service().await;

        let args = r#"{"track_id": "not-a-valid-uuid"}"#;
        let (result, action) = service.tool_play_track(args);

        assert!(result.contains("error"));
        assert!(result.contains("Invalid track_id"));
        assert!(action.is_none());
    }

    #[tokio::test]
    async fn test_add_to_queue_invalid_uuid() {
        let service = test_service().await;

        let args = r#"{"track_ids": ["123e4567-e89b-12d3-a456-426614174000", "invalid"]}"#;
        let (result, action) = service.tool_add_to_queue(args);

        assert!(result.contains("error"));
        assert!(result.contains("index 1"));
        assert!(action.is_none());
    }

    #[tokio::test]
    async fn test_add_to_queue_valid_uuids() {
        let service = test_service().await;

        let args = r#"{"track_ids": ["123e4567-e89b-12d3-a456-426614174000", "223e4567-e89b-12d3-a456-426614174000"]}"#;
        let (result, action) = service.tool_add_to_queue(args);

        assert!(result.contains("success"));
        assert!(action.is_some());
        assert_eq!(action.unwrap().action_type, "add_to_queue");
    }

    #[test]
    fn test_chat_error_to_api_error_conversion() {
        use crate::error::ApiError;

        let chat_err = ChatError::ConversationNotFound(Uuid::new_v4());
        let api_err: ApiError = chat_err.into();
        assert!(matches!(api_err, ApiError::NotFound { .. }));

        let chat_err = ChatError::InvalidInput("test".to_string());
        let api_err: ApiError = chat_err.into();
        assert!(matches!(api_err, ApiError::ValidationError(_)));

        let chat_err = ChatError::Timeout;
        let api_err: ApiError = chat_err.into();
        assert!(matches!(api_err, ApiError::AiService(_)));
    }

    // ==================== StreamEvent Tests ====================

    #[test]
    fn test_stream_event_from_error_database() {
        let err = ChatError::Database(sqlx::Error::RowNotFound);
        let event = StreamEvent::from_error(&err);

        match event {
            StreamEvent::Error { code, .. } => {
                assert_eq!(code, StreamErrorCode::Database);
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_stream_event_from_error_conversation_not_found() {
        let conv_id = Uuid::new_v4();
        let err = ChatError::ConversationNotFound(conv_id);
        let event = StreamEvent::from_error(&err);

        match event {
            StreamEvent::Error { message, code } => {
                assert_eq!(code, StreamErrorCode::ConversationNotFound);
                assert!(message.contains(&conv_id.to_string()));
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_stream_event_from_error_ollama_response() {
        let err = ChatError::OllamaResponse("Model not available".to_string());
        let event = StreamEvent::from_error(&err);

        match event {
            StreamEvent::Error { message, code } => {
                assert_eq!(code, StreamErrorCode::OllamaResponse);
                assert_eq!(message, "Model not available");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_stream_event_from_error_invalid_input() {
        let err = ChatError::InvalidInput("Message too long".to_string());
        let event = StreamEvent::from_error(&err);

        match event {
            StreamEvent::Error { message, code } => {
                assert_eq!(code, StreamErrorCode::InvalidInput);
                assert_eq!(message, "Message too long");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_stream_event_from_error_timeout() {
        let err = ChatError::Timeout;
        let event = StreamEvent::from_error(&err);

        match event {
            StreamEvent::Error { message, code } => {
                assert_eq!(code, StreamErrorCode::Timeout);
                assert!(message.contains("timed out"));
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_stream_event_from_error_tool_execution() {
        let err = ChatError::ToolExecution {
            tool_name: "search_library".to_string(),
            message: "Search failed".to_string(),
        };
        let event = StreamEvent::from_error(&err);

        match event {
            StreamEvent::Error { message, code } => {
                assert_eq!(code, StreamErrorCode::ToolExecution);
                assert!(message.contains("search_library"));
                assert!(message.contains("Search failed"));
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_stream_error_code_equality() {
        assert_eq!(StreamErrorCode::Database, StreamErrorCode::Database);
        assert_ne!(StreamErrorCode::Database, StreamErrorCode::Timeout);
        assert_eq!(
            StreamErrorCode::ConversationNotFound,
            StreamErrorCode::ConversationNotFound
        );
    }

    // ==================== Channel Communication Tests ====================

    #[tokio::test]
    async fn test_stream_channel_receives_token_events() {
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(10);

        // Simulate sending token events
        let tokens = vec!["Hello", " ", "world", "!"];
        for token in &tokens {
            tx.send(StreamEvent::Token(token.to_string()))
                .await
                .unwrap();
        }
        drop(tx); // Close sender to end the stream

        // Collect received events
        let mut received_tokens = Vec::new();
        while let Some(event) = rx.recv().await {
            if let StreamEvent::Token(t) = event {
                received_tokens.push(t);
            }
        }

        assert_eq!(received_tokens.len(), 4);
        assert_eq!(received_tokens.join(""), "Hello world!");
    }

    #[tokio::test]
    async fn test_stream_channel_receives_complete_event() {
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(10);

        let message_id = Uuid::new_v4();
        let full_response = "This is the full response".to_string();
        let actions = vec![ChatAction {
            action_type: "play_track".to_string(),
            data: serde_json::json!({"track_id": "123"}),
        }];

        tx.send(StreamEvent::Complete {
            message_id,
            full_response: full_response.clone(),
            actions: actions.clone(),
        })
        .await
        .unwrap();
        drop(tx);

        let event = rx.recv().await.unwrap();
        match event {
            StreamEvent::Complete {
                message_id: recv_id,
                full_response: recv_response,
                actions: recv_actions,
            } => {
                assert_eq!(recv_id, message_id);
                assert_eq!(recv_response, full_response);
                assert_eq!(recv_actions.len(), 1);
                assert_eq!(recv_actions[0].action_type, "play_track");
            }
            _ => panic!("Expected Complete event"),
        }
    }

    #[tokio::test]
    async fn test_stream_channel_receives_error_event() {
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(10);

        tx.send(StreamEvent::Error {
            message: "Something went wrong".to_string(),
            code: StreamErrorCode::OllamaResponse,
        })
        .await
        .unwrap();
        drop(tx);

        let event = rx.recv().await.unwrap();
        match event {
            StreamEvent::Error { message, code } => {
                assert_eq!(message, "Something went wrong");
                assert_eq!(code, StreamErrorCode::OllamaResponse);
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[tokio::test]
    async fn test_stream_channel_full_flow_tokens_then_complete() {
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(10);

        // Simulate a realistic streaming flow: tokens followed by complete
        tokio::spawn(async move {
            // Send tokens
            tx.send(StreamEvent::Token("I".to_string())).await.unwrap();
            tx.send(StreamEvent::Token(" recommend".to_string()))
                .await
                .unwrap();
            tx.send(StreamEvent::Token(" this".to_string()))
                .await
                .unwrap();
            tx.send(StreamEvent::Token(" song".to_string()))
                .await
                .unwrap();

            // Send complete event
            tx.send(StreamEvent::Complete {
                message_id: Uuid::new_v4(),
                full_response: "I recommend this song".to_string(),
                actions: vec![],
            })
            .await
            .unwrap();
        });

        // Collect all events
        let mut token_count = 0;
        let mut complete_count = 0;
        let mut full_content = String::new();

        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Token(t) => {
                    token_count += 1;
                    full_content.push_str(&t);
                }
                StreamEvent::Complete { full_response, .. } => {
                    complete_count += 1;
                    assert_eq!(full_response, full_content);
                }
                _ => {}
            }
        }

        assert_eq!(token_count, 4);
        assert_eq!(complete_count, 1);
        assert_eq!(full_content, "I recommend this song");
    }

    #[tokio::test]
    async fn test_stream_channel_handles_dropped_receiver() {
        let (tx, rx) = mpsc::channel::<StreamEvent>(10);

        // Drop the receiver immediately
        drop(rx);

        // Sending should fail gracefully (return Err)
        let result = tx.send(StreamEvent::Token("test".to_string())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stream_channel_tool_call_events() {
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(10);

        let call_id = "call_123".to_string();

        // Simulate tool call flow
        tx.send(StreamEvent::ToolCallStart {
            tool_name: "search_library".to_string(),
            call_id: call_id.clone(),
        })
        .await
        .unwrap();

        tx.send(StreamEvent::ToolCallComplete {
            call_id: call_id.clone(),
            result: r#"{"results": []}"#.to_string(),
        })
        .await
        .unwrap();

        drop(tx);

        // Verify events received in order
        let event1 = rx.recv().await.unwrap();
        match event1 {
            StreamEvent::ToolCallStart { tool_name, call_id } => {
                assert_eq!(tool_name, "search_library");
                assert_eq!(call_id, "call_123");
            }
            _ => panic!("Expected ToolCallStart event"),
        }

        let event2 = rx.recv().await.unwrap();
        match event2 {
            StreamEvent::ToolCallComplete { call_id, result } => {
                assert_eq!(call_id, "call_123");
                assert!(result.contains("results"));
            }
            _ => panic!("Expected ToolCallComplete event"),
        }
    }

    // ==================== Message Validation Tests ====================

    #[tokio::test]
    async fn test_send_message_streaming_validates_empty_message() {
        let service = test_service().await;
        let user_id = Uuid::new_v4();
        let context = UserContext {
            user_id,
            track_count: 10,
            artist_count: 5,
            album_count: 2,
            playlist_count: 1,
            top_genres: vec![],
            current_track_id: None,
            current_track_title: None,
        };

        let result = service
            .send_message_streaming(None, user_id, "   ".to_string(), context)
            .await;

        assert!(result.is_err());
        match result {
            Err(ChatError::InvalidInput(msg)) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[tokio::test]
    async fn test_send_message_streaming_validates_message_length() {
        let service = test_service().await;
        let user_id = Uuid::new_v4();
        let context = UserContext {
            user_id,
            track_count: 10,
            artist_count: 5,
            album_count: 2,
            playlist_count: 1,
            top_genres: vec![],
            current_track_id: None,
            current_track_title: None,
        };

        // Create a message longer than MAX_MESSAGE_LENGTH (10_000)
        let long_message = "a".repeat(10_001);

        let result = service
            .send_message_streaming(None, user_id, long_message, context)
            .await;

        assert!(result.is_err());
        match result {
            Err(ChatError::InvalidInput(msg)) => {
                assert!(msg.contains("too long"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    // ==================== StreamEvent Clone & Debug Tests ====================

    #[test]
    fn test_stream_event_clone() {
        let token_event = StreamEvent::Token("test".to_string());
        let cloned = token_event.clone();
        if let (StreamEvent::Token(a), StreamEvent::Token(b)) = (&token_event, &cloned) {
            assert_eq!(a, b);
        } else {
            panic!("Clone failed");
        }

        let error_event = StreamEvent::Error {
            message: "error".to_string(),
            code: StreamErrorCode::Timeout,
        };
        let cloned_error = error_event.clone();
        if let StreamEvent::Error { code, .. } = cloned_error {
            assert_eq!(code, StreamErrorCode::Timeout);
        } else {
            panic!("Clone failed");
        }
    }

    #[test]
    fn test_stream_event_debug() {
        let token_event = StreamEvent::Token("hello".to_string());
        let debug_str = format!("{:?}", token_event);
        assert!(debug_str.contains("Token"));
        assert!(debug_str.contains("hello"));

        let complete_event = StreamEvent::Complete {
            message_id: Uuid::nil(),
            full_response: "test".to_string(),
            actions: vec![],
        };
        let debug_str = format!("{:?}", complete_event);
        assert!(debug_str.contains("Complete"));
    }

    #[test]
    fn test_stream_error_code_debug_and_copy() {
        let code = StreamErrorCode::Database;
        let copied = code; // Copy
        assert_eq!(code, copied);

        let debug_str = format!("{:?}", code);
        assert!(debug_str.contains("Database"));
    }

    // ==================== ChatAction Tests ====================

    #[test]
    fn test_chat_action_serialization() {
        let action = ChatAction {
            action_type: "play_track".to_string(),
            data: serde_json::json!({"track_id": "abc-123"}),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("play_track"));
        assert!(json.contains("abc-123"));
    }

    #[test]
    fn test_chat_action_clone() {
        let action = ChatAction {
            action_type: "add_to_queue".to_string(),
            data: serde_json::json!({"track_ids": ["id1", "id2"]}),
        };

        let cloned = action.clone();
        assert_eq!(cloned.action_type, action.action_type);
        assert_eq!(cloned.data, action.data);
    }

    // ==================== Integration-Style Unit Tests ====================

    #[tokio::test]
    async fn test_stream_channel_capacity() {
        // Test that channel with specific capacity works correctly
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(STREAM_CHANNEL_CAPACITY);

        // Send several events
        for i in 0..50 {
            tx.send(StreamEvent::Token(format!("token_{}", i)))
                .await
                .unwrap();
        }

        // Receive and verify
        let mut count = 0;
        while let Ok(event) = rx.try_recv() {
            if let StreamEvent::Token(_) = event {
                count += 1;
            }
        }
        assert_eq!(count, 50);
    }

    #[tokio::test]
    async fn test_stream_event_ordering_preserved() {
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(100);

        // Send numbered tokens
        for i in 0..20 {
            tx.send(StreamEvent::Token(i.to_string())).await.unwrap();
        }
        drop(tx);

        // Verify order is preserved
        let mut expected = 0;
        while let Some(event) = rx.recv().await {
            if let StreamEvent::Token(t) = event {
                let num: i32 = t.parse().unwrap();
                assert_eq!(num, expected);
                expected += 1;
            }
        }
        assert_eq!(expected, 20);
    }
}
