// Allow unused code in this module - utility functions are provided for future use
// and may not all be called immediately
#![allow(dead_code)]

//! WebSocket handlers for real-time features
//!
//! This module provides:
//! - Cross-device playback synchronization via WebSocket
//! - Real-time presence updates for connected devices
//! - Playback transfer between devices (Spotify Connect style)
//! - Redis pub/sub for multi-instance synchronization
//!
//! # Architecture
//!
//! ```text
//! Device A ──┐
//!            │     ┌─────────────────┐
//! Device B ──┼────►│  Redis Pub/Sub  │◄───► WebSocket Server
//!            │     │  sync:user:{id} │
//! Device C ──┘     └─────────────────┘
//! ```
//!
//! # Authentication
//!
//! WebSocket connections are authenticated via JWT token passed as a query parameter:
//! `wss://api.example.com/ws/sync?token=<jwt>&device_id=<id>&device_name=<name>`
//!
//! # Message Protocol
//!
//! See [`messages`] module for the full message type definitions.
//!
//! ## Client → Server Messages
//! - `PlaybackStateUpdate` - Update playback state (from active device)
//! - `Seek` - Seek to position (from active device)
//! - `QueueUpdate` - Update queue state
//! - `TransferPlayback` - Request to transfer playback to another device
//! - `RequestDeviceList` - Request list of connected devices
//! - `Heartbeat` - Keep connection alive
//! - `SettingsUpdate` - Update synced settings
//!
//! ## Server → Client Messages
//! - `Connected` - Connection established successfully
//! - `PlaybackSync` - Playback state sync from another device
//! - `SeekSync` - Seek sync from another device
//! - `QueueSync` - Queue state sync
//! - `DeviceList` - List of connected devices
//! - `DeviceConnected` / `DeviceDisconnected` - Device presence changes
//! - `TransferRequested` / `TransferAccepted` - Playback transfer flow
//! - `Pong` - Heartbeat response
//! - `SettingsSync` - Settings sync
//! - `Error` - Error occurred
//!
//! # Control Model
//!
//! This implementation uses an **explicit transfer** model (Spotify Connect style):
//! - Only the "active device" can send playback updates
//! - Other devices must request a transfer to become active
//! - New/reconnecting devices automatically sync to the active device's state

pub mod connection;
pub mod handler;
pub mod messages;
pub mod presence;
pub mod pubsub;
pub mod sync;

pub use connection::ConnectionManager;
pub use handler::ws_handler;
pub use pubsub::SyncPubSub;
