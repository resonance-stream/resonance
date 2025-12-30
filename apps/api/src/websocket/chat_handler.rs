//! Chat message handler for WebSocket AI assistant
//!
//! This module handles the chat-specific WebSocket messages,
//! integrating with the ChatService for AI responses.

use resonance_shared_config::OllamaConfig;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::connection::ConnectionManager;
use super::messages::{
    ChatAction, ChatCompletePayload, ChatErrorPayload, ChatSendPayload, ServerMessage,
};
use crate::services::chat::{
    ChatAction as ServiceChatAction, ChatError, ChatService, UserContextBuilder,
};

/// Maximum messages per minute per user
const MAX_MESSAGES_PER_MINUTE: u32 = 20;

/// Minimum interval between messages in seconds
const MIN_MESSAGE_INTERVAL_SECS: u64 = 2;

/// Channel capacity for pending chat messages
const CHAT_CHANNEL_CAPACITY: usize = 4;

/// Handles chat messages for a WebSocket connection
pub struct ChatHandler {
    user_id: Uuid,
    device_id: String,
    chat_service: ChatService,
    context_builder: UserContextBuilder,
    connection_manager: ConnectionManager,
    /// Last message timestamp for rate limiting
    last_message_time: Arc<Mutex<Instant>>,
    /// Message count in current window for rate limiting
    message_count: Arc<AtomicU32>,
    /// Window start time for rate limiting
    window_start: Arc<Mutex<Instant>>,
}

impl ChatHandler {
    /// Create a new chat handler
    pub fn new(
        user_id: Uuid,
        device_id: String,
        pool: PgPool,
        config: OllamaConfig,
        connection_manager: ConnectionManager,
    ) -> Self {
        let now = Instant::now();
        Self {
            user_id,
            device_id,
            chat_service: ChatService::new(pool.clone(), config),
            context_builder: UserContextBuilder::new(pool),
            connection_manager,
            last_message_time: Arc::new(Mutex::new(now)),
            message_count: Arc::new(AtomicU32::new(0)),
            window_start: Arc::new(Mutex::new(now)),
        }
    }

    /// Check rate limit and return error payload if exceeded
    async fn check_rate_limit(&self) -> Option<ChatErrorPayload> {
        let now = Instant::now();

        // Check minimum interval between messages
        {
            let mut last_time = self.last_message_time.lock().await;
            let elapsed = now.duration_since(*last_time);
            if elapsed < Duration::from_secs(MIN_MESSAGE_INTERVAL_SECS) {
                // Calculate remaining time, ensuring at least 1 second is shown
                let remaining_millis = Duration::from_secs(MIN_MESSAGE_INTERVAL_SECS)
                    .saturating_sub(elapsed)
                    .as_millis();
                let wait_secs = remaining_millis.div_ceil(1000).max(1) as u64;
                let plural = if wait_secs == 1 { "" } else { "s" };
                return Some(ChatErrorPayload::new(
                    None,
                    "RATE_LIMITED",
                    format!(
                        "Please wait {} second{} before sending another message",
                        wait_secs, plural
                    ),
                ));
            }
            *last_time = now;
        }

        // Check messages per minute limit
        {
            let mut window_start = self.window_start.lock().await;
            let window_elapsed = now.duration_since(*window_start);

            if window_elapsed >= Duration::from_secs(60) {
                // Reset window
                *window_start = now;
                self.message_count.store(1, Ordering::SeqCst);
            } else {
                let count = self.message_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count > MAX_MESSAGES_PER_MINUTE {
                    let remaining_secs = 60 - window_elapsed.as_secs();
                    return Some(ChatErrorPayload::new(
                        None,
                        "RATE_LIMITED",
                        format!(
                            "Message limit exceeded. Please wait {} seconds",
                            remaining_secs
                        ),
                    ));
                }
            }
        }

        None
    }

    /// Handle an incoming chat message
    ///
    /// This processes the chat message asynchronously, sending the
    /// complete response back to the client.
    pub async fn handle_chat_send(&self, payload: ChatSendPayload) {
        let conversation_id = payload.conversation_id;
        let message = payload.message;

        // Check rate limit before processing
        if let Some(error_payload) = self.check_rate_limit().await {
            warn!(
                user_id = %self.user_id,
                device_id = %self.device_id,
                "Chat message rate limited"
            );
            self.send_to_self(ServerMessage::ChatError(error_payload));
            return;
        }

        info!(
            user_id = %self.user_id,
            device_id = %self.device_id,
            conversation_id = ?conversation_id,
            message_len = message.len(),
            "Processing chat message"
        );

        // Build user context for this request
        let context = match self.context_builder.build(self.user_id).await {
            Ok(ctx) => ctx,
            Err(e) => {
                warn!(
                    user_id = %self.user_id,
                    error = %e,
                    "Failed to build user context"
                );
                let error_payload = ChatErrorPayload::new(
                    conversation_id,
                    "CONTEXT_ERROR",
                    "Failed to build user context",
                );
                self.send_to_self(ServerMessage::ChatError(error_payload));
                return;
            }
        };

        // Send the message to the AI service
        match self
            .chat_service
            .send_message(conversation_id, self.user_id, message, &context)
            .await
        {
            Ok((conversation, assistant_message, actions)) => {
                // Convert service actions to WebSocket actions
                let ws_actions: Vec<ChatAction> =
                    actions.into_iter().filter_map(convert_action).collect();

                let complete_msg = ServerMessage::ChatComplete(ChatCompletePayload {
                    conversation_id: conversation.id,
                    message_id: assistant_message.id,
                    full_response: assistant_message.content.unwrap_or_default(),
                    actions: ws_actions,
                });

                self.send_to_self(complete_msg);
            }
            Err(e) => {
                warn!(
                    user_id = %self.user_id,
                    device_id = %self.device_id,
                    error = %e,
                    "Chat message failed"
                );

                let error_payload = convert_chat_error(conversation_id, e);
                self.send_to_self(ServerMessage::ChatError(error_payload));
            }
        }
    }

    /// Send a message to this device
    fn send_to_self(&self, msg: ServerMessage) {
        if let Err(e) = self
            .connection_manager
            .send_to_device(self.user_id, &self.device_id, msg)
        {
            error!(
                user_id = %self.user_id,
                device_id = %self.device_id,
                error = %e,
                "Failed to send chat response"
            );
        }
    }
}

/// Convert a service ChatAction (struct with action_type and data) to a WebSocket ChatAction (enum)
fn convert_action(action: ServiceChatAction) -> Option<ChatAction> {
    match action.action_type.as_str() {
        "play_track" => {
            let track_id: Uuid = action.data.get("track_id")?.as_str()?.parse().ok()?;
            Some(ChatAction::PlayTrack { track_id })
        }
        "add_to_queue" => {
            let track_ids: Vec<Uuid> = action
                .data
                .get("track_ids")?
                .as_array()?
                .iter()
                .filter_map(|v| v.as_str()?.parse().ok())
                .collect();
            if track_ids.is_empty() {
                return None;
            }
            Some(ChatAction::AddToQueue { track_ids })
        }
        "create_playlist" => {
            let name = action.data.get("name")?.as_str()?.to_string();
            let description = action
                .data
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from);
            let track_ids: Vec<Uuid> = action
                .data
                .get("track_ids")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str()?.parse().ok())
                        .collect()
                })
                .unwrap_or_default();
            Some(ChatAction::CreatePlaylist {
                name,
                description,
                track_ids,
            })
        }
        "search" | "search_library" => {
            let query = action.data.get("query")?.as_str()?.to_string();
            let result_type = action
                .data
                .get("type")
                .or_else(|| action.data.get("search_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("track")
                .to_string();
            Some(ChatAction::ShowSearch { query, result_type })
        }
        _ => {
            warn!(
                action_type = %action.action_type,
                "Unknown chat action type"
            );
            None
        }
    }
}

/// Convert a ChatError to a ChatErrorPayload
///
/// Note: Database and serialization errors are sanitized to avoid exposing
/// internal details to clients. Full error details are logged server-side.
fn convert_chat_error(conversation_id: Option<Uuid>, error: ChatError) -> ChatErrorPayload {
    match error {
        ChatError::ConversationNotFound(id) => ChatErrorPayload::conversation_not_found(id),
        ChatError::OllamaRequest(_) | ChatError::OllamaResponse(_) => {
            ChatErrorPayload::ai_unavailable(conversation_id)
        }
        ChatError::InvalidInput(msg) => ChatErrorPayload::invalid_message(conversation_id, msg),
        ChatError::Timeout => ChatErrorPayload::new(
            conversation_id,
            "TIMEOUT",
            "Request timed out. Please try again.",
        ),
        ChatError::Database(_) => {
            // Don't expose internal database errors to clients
            ChatErrorPayload::new(
                conversation_id,
                "DATABASE_ERROR",
                "A database error occurred. Please try again.",
            )
        }
        ChatError::Serialization(_) => {
            // Don't expose serialization details to clients
            ChatErrorPayload::new(
                conversation_id,
                "PROCESSING_ERROR",
                "Failed to process the response. Please try again.",
            )
        }
        ChatError::ToolExecution { tool_name, .. } => {
            // Don't expose internal tool error details to clients
            ChatErrorPayload::new(
                conversation_id,
                "TOOL_ERROR",
                format!(
                    "The '{}' action could not be completed. Please try again.",
                    tool_name
                ),
            )
        }
    }
}

/// Spawn a chat handler task for processing chat messages
///
/// This creates a dedicated task for handling chat messages, allowing
/// long-running AI requests without blocking other WebSocket operations.
///
/// Returns a tuple of (sender, join_handle) so the caller can:
/// - Send messages via the sender
/// - Abort the task via the join_handle when the connection closes
pub fn spawn_chat_handler(
    user_id: Uuid,
    device_id: String,
    pool: PgPool,
    config: OllamaConfig,
    connection_manager: ConnectionManager,
) -> (mpsc::Sender<ChatSendPayload>, JoinHandle<()>) {
    let (tx, mut rx) = mpsc::channel::<ChatSendPayload>(CHAT_CHANNEL_CAPACITY);

    let handler = ChatHandler::new(user_id, device_id.clone(), pool, config, connection_manager);

    let handle = tokio::spawn(async move {
        while let Some(payload) = rx.recv().await {
            handler.handle_chat_send(payload).await;
        }
        info!(device_id = %device_id, "Chat handler task ended");
    });

    (tx, handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_action_play_track() {
        let action = ServiceChatAction {
            action_type: "play_track".to_string(),
            data: serde_json::json!({
                "track_id": "00000000-0000-0000-0000-000000000000"
            }),
        };
        let converted = convert_action(action);
        assert!(matches!(converted, Some(ChatAction::PlayTrack { .. })));
    }

    #[test]
    fn test_convert_action_add_to_queue() {
        let action = ServiceChatAction {
            action_type: "add_to_queue".to_string(),
            data: serde_json::json!({
                "track_ids": ["00000000-0000-0000-0000-000000000000"]
            }),
        };
        let converted = convert_action(action);
        assert!(matches!(converted, Some(ChatAction::AddToQueue { .. })));
    }

    #[test]
    fn test_convert_action_create_playlist() {
        let action = ServiceChatAction {
            action_type: "create_playlist".to_string(),
            data: serde_json::json!({
                "name": "Test Playlist",
                "description": "A test playlist",
                "track_ids": []
            }),
        };
        let converted = convert_action(action);
        match converted {
            Some(ChatAction::CreatePlaylist {
                name, description, ..
            }) => {
                assert_eq!(name, "Test Playlist");
                assert_eq!(description, Some("A test playlist".to_string()));
            }
            _ => panic!("Expected CreatePlaylist action"),
        }
    }

    #[test]
    fn test_convert_action_search() {
        let action = ServiceChatAction {
            action_type: "search_library".to_string(),
            data: serde_json::json!({
                "query": "rock",
                "type": "track"
            }),
        };
        let converted = convert_action(action);
        match converted {
            Some(ChatAction::ShowSearch { query, result_type }) => {
                assert_eq!(query, "rock");
                assert_eq!(result_type, "track");
            }
            _ => panic!("Expected ShowSearch action"),
        }
    }

    #[test]
    fn test_convert_action_unknown() {
        let action = ServiceChatAction {
            action_type: "unknown_action".to_string(),
            data: serde_json::json!({}),
        };
        let converted = convert_action(action);
        assert!(converted.is_none());
    }

    #[test]
    fn test_convert_chat_error_conversation_not_found() {
        let id = Uuid::new_v4();
        let error = ChatError::ConversationNotFound(id);
        let payload = convert_chat_error(Some(id), error);
        assert_eq!(payload.code, "CONVERSATION_NOT_FOUND");
        assert_eq!(payload.conversation_id, Some(id));
    }

    #[test]
    fn test_convert_chat_error_timeout() {
        let error = ChatError::Timeout;
        let payload = convert_chat_error(None, error);
        assert_eq!(payload.code, "TIMEOUT");
        assert!(payload.error.contains("timed out"));
    }

    #[test]
    fn test_convert_chat_error_invalid_input() {
        let error = ChatError::InvalidInput("Message too long".to_string());
        let payload = convert_chat_error(None, error);
        assert_eq!(payload.code, "INVALID_MESSAGE");
        assert!(payload.error.contains("Message too long"));
    }

    #[test]
    fn test_convert_chat_error_database_sanitized() {
        let error = ChatError::Database(sqlx::Error::RowNotFound);
        let payload = convert_chat_error(None, error);
        assert_eq!(payload.code, "DATABASE_ERROR");
        // Should NOT contain internal error details
        assert!(!payload.error.contains("RowNotFound"));
        assert!(payload.error.contains("database error occurred"));
    }

    #[test]
    fn test_convert_chat_error_serialization_sanitized() {
        let error = ChatError::Serialization(serde_json::from_str::<()>("invalid").unwrap_err());
        let payload = convert_chat_error(None, error);
        assert_eq!(payload.code, "PROCESSING_ERROR");
        // Should NOT contain internal error details
        assert!(!payload.error.contains("invalid"));
        assert!(payload.error.contains("Failed to process"));
    }
}
