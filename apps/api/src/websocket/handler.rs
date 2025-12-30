//! WebSocket upgrade handler with JWT authentication
//!
//! This module handles the WebSocket upgrade request and authenticates
//! clients using JWT tokens passed via query parameter.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, Extension, Query,
    },
    http::HeaderMap,
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use uuid::Uuid;

use resonance_shared_config::OllamaConfig;

use crate::middleware::extract_client_ip;
use crate::services::auth::AuthService;
use crate::services::search::SearchService;
use crate::services::similarity::SimilarityService;
use resonance_ollama_client::OllamaClient;

use super::chat_handler::spawn_chat_handler;
use super::connection::{ConnectionManager, DeviceInfo};
use super::messages::{ClientMessage, ConnectedPayload, DeviceType, ErrorPayload, ServerMessage};
use super::pubsub::SyncPubSub;
use super::sync::SyncHandler;

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct WsQueryParams {
    /// JWT access token for authentication
    token: String,
    /// Device ID (client-generated, persistent per device)
    device_id: String,
    /// Human-readable device name
    #[serde(default = "default_device_name")]
    device_name: String,
    /// Device type hint
    #[serde(default)]
    device_type: Option<String>,
}

fn default_device_name() -> String {
    "Unknown Device".to_string()
}

/// Validate device ID format
fn validate_device_id(device_id: &str) -> Result<(), &'static str> {
    if device_id.is_empty() {
        return Err("device_id cannot be empty");
    }
    if device_id.len() > 128 {
        return Err("device_id must be at most 128 characters");
    }
    if !device_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err("device_id contains invalid characters");
    }
    Ok(())
}

/// WebSocket upgrade handler
///
/// Authenticates the connection via JWT token in query parameter,
/// then upgrades to WebSocket and manages the connection.
#[allow(clippy::too_many_arguments)]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    Extension(auth_service): Extension<AuthService>,
    Extension(connection_manager): Extension<ConnectionManager>,
    Extension(pubsub): Extension<SyncPubSub>,
    Extension(pool): Extension<PgPool>,
    Extension(ollama_config): Extension<OllamaConfig>,
    Extension(search_service): Extension<SearchService>,
    Extension(similarity_service): Extension<SimilarityService>,
    Extension(ollama_client): Extension<Option<OllamaClient>>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
) -> Response {
    // Verify JWT token
    let claims = match auth_service.verify_access_token(&params.token) {
        Ok(claims) => claims,
        Err(e) => {
            tracing::warn!(error = %e, "WebSocket auth failed");
            return ws.on_upgrade(|mut socket| async move {
                let error_msg = ServerMessage::Error(ErrorPayload::auth_failed(e.to_string()));
                if let Ok(json) = serde_json::to_string(&error_msg) {
                    let _ = socket.send(Message::Text(json)).await;
                }
                let _ = socket.close().await;
            });
        }
    };

    // Validate device ID format
    if let Err(e) = validate_device_id(&params.device_id) {
        tracing::warn!(device_id = %params.device_id, "Invalid device ID: {}", e);
        let error_message = e.to_string();
        return ws.on_upgrade(move |mut socket| async move {
            let error_msg =
                ServerMessage::Error(ErrorPayload::new("INVALID_DEVICE_ID", error_message));
            if let Ok(json) = serde_json::to_string(&error_msg) {
                let _ = socket.send(Message::Text(json)).await;
            }
            let _ = socket.close().await;
        });
    }

    let user_id = claims.sub;
    let session_id = claims.sid;
    let device_id = params.device_id.clone();
    let device_name = params.device_name.clone();
    let device_type = params
        .device_type
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DeviceType::Unknown);

    // Extract client IP for logging
    let client_ip = extract_client_ip(&headers, connect_info.as_ref());

    // Extract user agent
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    tracing::info!(
        user_id = %user_id,
        device_id = %device_id,
        device_name = %device_name,
        device_type = %device_type,
        client_ip = %client_ip,
        "WebSocket connection authenticated"
    );

    // Upgrade the connection
    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            user_id,
            session_id,
            DeviceInfo {
                device_id,
                device_name,
                device_type,
                user_agent,
            },
            connection_manager,
            pubsub,
            pool,
            ollama_config,
            search_service,
            similarity_service,
            ollama_client,
        )
    })
}

/// Handle an established WebSocket connection
#[allow(clippy::too_many_arguments)]
async fn handle_socket(
    socket: WebSocket,
    user_id: Uuid,
    session_id: Uuid,
    device_info: DeviceInfo,
    connection_manager: ConnectionManager,
    pubsub: SyncPubSub,
    pool: PgPool,
    ollama_config: OllamaConfig,
    search_service: SearchService,
    similarity_service: SimilarityService,
    ollama_client: Option<OllamaClient>,
) {
    let device_id = device_info.device_id.clone();
    let device_name = device_info.device_name.clone();

    // Create unbounded channel for sending messages to this connection
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Add connection to manager
    connection_manager.add_connection(user_id, device_id.clone(), tx, device_info.clone());

    // Split the socket into sender and receiver
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Spawn chat handler for this connection
    let (chat_tx, chat_handle) = spawn_chat_handler(
        user_id,
        device_id.clone(),
        pool.clone(),
        ollama_config,
        search_service,
        similarity_service,
        ollama_client,
        connection_manager.clone(),
    );

    // Create sync handler for processing messages with database persistence
    let sync_handler = SyncHandler::with_pool(
        user_id,
        device_id.clone(),
        connection_manager.clone(),
        pubsub.clone(),
        pool.clone(),
    );

    // Get current state for the connecting device
    let active_device_id = connection_manager.get_active_device(user_id);

    // Send connected message
    let connected_msg = ServerMessage::Connected(ConnectedPayload {
        device_id: device_id.clone(),
        session_id,
        active_device_id: active_device_id.clone(),
    });

    if let Ok(json) = serde_json::to_string(&connected_msg) {
        if ws_sender.send(Message::Text(json)).await.is_err() {
            tracing::warn!(
                user_id = %user_id,
                device_id = %device_id,
                "Failed to send connected message"
            );
            connection_manager.remove_connection(user_id, &device_id);
            return;
        }
    }

    // Notify other devices about new connection
    sync_handler
        .handle_device_connected(device_info.clone())
        .await;

    // If there's an active device, sync the new device to its state
    if active_device_id.is_some() {
        if let Some(state) = connection_manager.get_playback_state(user_id) {
            let sync_msg = ServerMessage::PlaybackSync(state);
            if let Ok(json) = serde_json::to_string(&sync_msg) {
                let _ = ws_sender.send(Message::Text(json)).await;
            }
        }
    }

    // Send current device list
    let devices = connection_manager.get_device_list(user_id);
    let device_list_msg = ServerMessage::DeviceList(devices);
    if let Ok(json) = serde_json::to_string(&device_list_msg) {
        let _ = ws_sender.send(Message::Text(json)).await;
    }

    // Subscribe to Redis pub/sub for this user
    let mut pubsub_receiver = pubsub.subscribe(user_id).await;

    // Spawn task to forward messages from channel to WebSocket
    let device_id_clone = device_id.clone();
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Messages from internal channel (from other handlers)
                Some(msg) = rx.recv() => {
                    match serde_json::to_string(&msg) {
                        Ok(json) => {
                            if ws_sender.send(Message::Text(json)).await.is_err() {
                                tracing::debug!(device_id = %device_id_clone, "WebSocket send failed");
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to serialize message");
                        }
                    }
                }
                // Messages from Redis pub/sub
                result = pubsub_receiver.recv() => {
                    match result {
                        Ok(event) => {
                            // Convert sync event to server message and send
                            if let Some(msg) = super::sync::sync_event_to_server_message(&event, &device_id_clone) {
                                match serde_json::to_string(&msg) {
                                    Ok(json) => {
                                        if ws_sender.send(Message::Text(json)).await.is_err() {
                                            tracing::debug!(device_id = %device_id_clone, "WebSocket send failed");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(error = %e, "Failed to serialize message");
                                    }
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(device_id = %device_id_clone, lagged = n, "Pub/sub receiver lagged");
                            // Continue - we'll catch up
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            tracing::debug!(device_id = %device_id_clone, "Pub/sub channel closed");
                            break;
                        }
                    }
                }
                else => break,
            }
        }
    });

    // Handle incoming messages
    let device_id_recv = device_id.clone();
    let chat_tx_recv = chat_tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(msg) => {
                            // Route ChatSend messages to the chat handler
                            if let ClientMessage::ChatSend(payload) = msg {
                                match chat_tx_recv.try_send(payload) {
                                    Ok(_) => {}
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        tracing::warn!(
                                            device_id = %device_id_recv,
                                            "Chat message queue full, message dropped"
                                        );
                                        // Send error back to client
                                        let error_msg = super::messages::ChatErrorPayload::new(
                                            None,
                                            "QUEUE_FULL",
                                            "Too many pending messages. Please wait.",
                                        );
                                        sync_handler.send_to_device(
                                            &device_id_recv,
                                            ServerMessage::ChatError(error_msg),
                                        );
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => {
                                        tracing::warn!(
                                            device_id = %device_id_recv,
                                            "Chat handler channel closed"
                                        );
                                        // Notify client that chat is unavailable
                                        let error_msg = super::messages::ChatErrorPayload::new(
                                            None,
                                            "CHAT_UNAVAILABLE",
                                            "Chat service is temporarily unavailable. Please try again.",
                                        );
                                        sync_handler.send_to_device(
                                            &device_id_recv,
                                            ServerMessage::ChatError(error_msg),
                                        );
                                    }
                                }
                            } else {
                                // All other messages go to sync handler
                                if let Err(e) = sync_handler.handle_message(msg).await {
                                    tracing::warn!(
                                        error = %e,
                                        device_id = %device_id_recv,
                                        "Error handling client message"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::debug!(
                                error = %e,
                                device_id = %device_id_recv,
                                "Failed to parse client message"
                            );
                            // Send error back to client
                            let error_msg = ErrorPayload::invalid_message(e.to_string());
                            sync_handler
                                .send_to_device(&device_id_recv, ServerMessage::Error(error_msg));
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    // Binary messages not supported for sync protocol
                    tracing::debug!(device_id = %device_id_recv, "Received unsupported binary message");
                }
                Ok(Message::Ping(data)) => {
                    // Pings are handled automatically by axum-ws
                    tracing::trace!(device_id = %device_id_recv, "Received ping: {:?}", data);
                }
                Ok(Message::Pong(_)) => {
                    // Update last seen time
                    tracing::trace!(device_id = %device_id_recv, "Received pong");
                }
                Ok(Message::Close(_)) => {
                    tracing::debug!(device_id = %device_id_recv, "WebSocket close received");
                    break;
                }
                Err(e) => {
                    tracing::debug!(error = %e, device_id = %device_id_recv, "WebSocket error");
                    break;
                }
            }
        }
    });

    // Wait for either task to complete, then abort the other
    tokio::select! {
        _ = &mut send_task => {
            tracing::debug!(device_id = %device_id, "Send task completed");
            recv_task.abort();
        }
        _ = &mut recv_task => {
            tracing::debug!(device_id = %device_id, "Receive task completed");
            send_task.abort();
        }
    }

    // Gracefully stop the chat handler by dropping the sender (closes channel)
    // The handler will receive None and exit its loop naturally
    drop(chat_tx);

    // Wait briefly for graceful shutdown, then abort if still running
    let shutdown_timeout = std::time::Duration::from_secs(1);
    match tokio::time::timeout(shutdown_timeout, chat_handle).await {
        Ok(Ok(_)) => {
            tracing::debug!(device_id = %device_id, "Chat handler stopped gracefully");
        }
        Ok(Err(e)) if e.is_cancelled() => {
            tracing::debug!(device_id = %device_id, "Chat handler was cancelled");
        }
        Ok(Err(e)) => {
            tracing::warn!(device_id = %device_id, error = %e, "Chat handler panicked");
        }
        Err(_) => {
            // Timeout elapsed - the handle was consumed so we can't abort explicitly,
            // but the task will be cleaned up when dropping its resources
            tracing::debug!(device_id = %device_id, "Chat handler shutdown timed out");
        }
    }

    // Check if this device was active BEFORE removing connection (to avoid race condition)
    let was_active = connection_manager.get_active_device(user_id) == Some(device_id.clone());

    // Clean up: remove connection
    connection_manager.remove_connection(user_id, &device_id);

    // Notify other devices about disconnection
    let disconnect_handler = SyncHandler::new(
        user_id,
        device_id.clone(),
        connection_manager.clone(),
        pubsub,
    );
    disconnect_handler
        .handle_device_disconnected(was_active)
        .await;

    tracing::info!(
        user_id = %user_id,
        device_id = %device_id,
        device_name = %device_name,
        "WebSocket connection closed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_device_id_valid() {
        assert!(validate_device_id("device-123").is_ok());
        assert!(validate_device_id("device_123").is_ok());
        assert!(validate_device_id("abc123").is_ok());
        assert!(validate_device_id("a").is_ok());
    }

    #[test]
    fn test_validate_device_id_empty() {
        assert!(validate_device_id("").is_err());
    }

    #[test]
    fn test_validate_device_id_too_long() {
        let long_id = "a".repeat(129);
        assert!(validate_device_id(&long_id).is_err());

        let max_length_id = "a".repeat(128);
        assert!(validate_device_id(&max_length_id).is_ok());
    }

    #[test]
    fn test_validate_device_id_invalid_chars() {
        assert!(validate_device_id("device<script>").is_err());
        assert!(validate_device_id("device/path").is_err());
        assert!(validate_device_id("device with spaces").is_err());
        assert!(validate_device_id("device.name").is_err());
        assert!(validate_device_id("device@name").is_err());
    }

    #[test]
    fn test_ws_query_params_deserialization() {
        let json = r#"{"token":"abc123","device_id":"dev-1","device_name":"My Phone","device_type":"mobile"}"#;
        let params: WsQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.token, "abc123");
        assert_eq!(params.device_id, "dev-1");
        assert_eq!(params.device_name, "My Phone");
        assert_eq!(params.device_type, Some("mobile".to_string()));
    }

    #[test]
    fn test_ws_query_params_defaults() {
        let json = r#"{"token":"abc123","device_id":"dev-1"}"#;
        let params: WsQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.token, "abc123");
        assert_eq!(params.device_id, "dev-1");
        assert_eq!(params.device_name, "Unknown Device");
        assert_eq!(params.device_type, None);
    }
}
