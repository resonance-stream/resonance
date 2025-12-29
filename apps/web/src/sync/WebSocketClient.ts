/**
 * WebSocket client for real-time synchronization
 *
 * Features:
 * - Automatic reconnection with exponential backoff
 * - Connection state management
 * - Heartbeat mechanism
 * - Message queuing during reconnection
 * - Rate limiting for outgoing messages
 */

import type {
  ClientMessage,
  ServerMessage,
  ConnectionState,
  DeviceInfo,
  ConnectedPayload,
} from './types';
import { getOrCreateDeviceId, getDefaultDeviceName, detectDeviceType } from './types';

/** Configuration for the WebSocket client */
export interface WebSocketClientConfig {
  /** Base URL for WebSocket connection (defaults to window.location) */
  baseUrl?: string;
  /** Path for WebSocket endpoint */
  path?: string;
  /** Device name (auto-detected if not provided) */
  deviceName?: string;
  /** Device type (auto-detected if not provided) */
  deviceType?: string;
  /** Initial reconnect delay in ms (default: 1000) */
  initialReconnectDelay?: number;
  /** Maximum reconnect delay in ms (default: 30000) */
  maxReconnectDelay?: number;
  /** Heartbeat interval in ms (default: 30000) */
  heartbeatInterval?: number;
  /** Message rate limit per second (default: 4) */
  rateLimit?: number;
  /** Maximum queued messages during reconnection (default: 100) */
  maxQueueSize?: number;
}

/** Event handler types */
export interface WebSocketClientEvents {
  onStateChange?: (state: ConnectionState) => void;
  onConnected?: (payload: ConnectedPayload) => void;
  onMessage?: (message: ServerMessage) => void;
  onError?: (error: Error) => void;
  onReconnecting?: (attempt: number) => void;
}

const DEFAULT_CONFIG: Required<Omit<WebSocketClientConfig, 'baseUrl' | 'deviceName' | 'deviceType'>> = {
  path: '/ws/sync',
  initialReconnectDelay: 1000,
  maxReconnectDelay: 30000,
  heartbeatInterval: 30000,
  rateLimit: 4,
  maxQueueSize: 100,
};

/**
 * WebSocket client for cross-device synchronization
 */
export class WebSocketClient {
  private ws: WebSocket | null = null;
  private config: Required<WebSocketClientConfig>;
  private events: WebSocketClientEvents = {};

  // Connection state
  private _state: ConnectionState = 'disconnected';
  private _deviceId: string;
  private _sessionId: string | null = null;
  private _activeDeviceId: string | null = null;

  // Reconnection
  private reconnectAttempt = 0;
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  private shouldReconnect = true;

  // Heartbeat
  private heartbeatInterval: ReturnType<typeof setInterval> | null = null;
  /** Last pong timestamp from server (for latency monitoring) */
  public lastPongTime = 0;
  /** Timestamp when last heartbeat was sent */
  private lastHeartbeatSent = 0;
  /** Heartbeat timeout (2x heartbeat interval) */
  private heartbeatTimeoutMs = 0;

  // Rate limiting
  private messageTimes: number[] = [];

  // Message queue (for messages sent during reconnection)
  private messageQueue: ClientMessage[] = [];

  // Retry timeout for flushing rate-limited messages
  private flushRetryTimeout: ReturnType<typeof setTimeout> | null = null;

  constructor(config: WebSocketClientConfig = {}, events: WebSocketClientEvents = {}) {
    this._deviceId = getOrCreateDeviceId();

    this.config = {
      baseUrl: config.baseUrl ?? this.getDefaultBaseUrl(),
      path: config.path ?? DEFAULT_CONFIG.path,
      deviceName: config.deviceName ?? getDefaultDeviceName(),
      deviceType: config.deviceType ?? detectDeviceType(),
      initialReconnectDelay: config.initialReconnectDelay ?? DEFAULT_CONFIG.initialReconnectDelay,
      maxReconnectDelay: config.maxReconnectDelay ?? DEFAULT_CONFIG.maxReconnectDelay,
      heartbeatInterval: config.heartbeatInterval ?? DEFAULT_CONFIG.heartbeatInterval,
      rateLimit: config.rateLimit ?? DEFAULT_CONFIG.rateLimit,
      maxQueueSize: config.maxQueueSize ?? DEFAULT_CONFIG.maxQueueSize,
    };

    this.events = events;
  }

  // =============================================================================
  // Public API
  // =============================================================================

  /** Current connection state */
  get state(): ConnectionState {
    return this._state;
  }

  /** Current device ID */
  get deviceId(): string {
    return this._deviceId;
  }

  /** Current session ID (set after connection) */
  get sessionId(): string | null {
    return this._sessionId;
  }

  /** Currently active device ID */
  get activeDeviceId(): string | null {
    return this._activeDeviceId;
  }

  /** Whether this device is the active device */
  get isActiveDevice(): boolean {
    return this._activeDeviceId === this._deviceId;
  }

  /**
   * Connect to the WebSocket server
   *
   * @param token - JWT auth token
   */
  connect(token: string): void {
    if (this.ws && (this._state === 'connected' || this._state === 'connecting')) {
      console.warn('[WebSocketClient] Already connected or connecting');
      return;
    }

    this.shouldReconnect = true;
    this.reconnectAttempt = 0;
    this.createConnection(token);
  }

  /**
   * Disconnect from the server
   */
  disconnect(): void {
    this.shouldReconnect = false;
    this.messageQueue = []; // Clear queued messages to prevent stale data on reconnect
    this.cleanup();
    this.setState('disconnected');
  }

  /**
   * Send a message to the server
   *
   * If not connected, the message will be queued (up to maxQueueSize).
   * Returns true if sent/queued successfully, false if rate limited or queue full.
   */
  send(message: ClientMessage): boolean {
    // Bypass rate limiting for heartbeat messages (critical for connection health)
    if (message.type !== 'Heartbeat' && !this.checkRateLimit()) {
      console.warn('[WebSocketClient] Rate limited, message dropped');
      return false;
    }

    // If connected, send immediately
    if (this.ws && this._state === 'connected') {
      try {
        this.ws.send(JSON.stringify(message));
        this.recordMessageTime();
        return true;
      } catch (error) {
        console.error('[WebSocketClient] Failed to send message:', error);
        return false;
      }
    }

    // Queue message for later
    if (this.messageQueue.length >= this.config.maxQueueSize) {
      console.warn('[WebSocketClient] Message queue full, dropping oldest message');
      this.messageQueue.shift();
    }

    this.messageQueue.push(message);
    return true;
  }

  /**
   * Request the list of connected devices
   */
  requestDeviceList(): void {
    this.send({ type: 'RequestDeviceList' });
  }

  /**
   * Request to transfer playback to another device
   */
  transferPlayback(targetDeviceId: string): void {
    this.send({ type: 'TransferPlayback', payload: { target_device_id: targetDeviceId } });
  }

  /**
   * Update event handlers
   */
  setEventHandlers(events: Partial<WebSocketClientEvents>): void {
    this.events = { ...this.events, ...events };
  }

  // =============================================================================
  // Private Methods
  // =============================================================================

  private getDefaultBaseUrl(): string {
    if (typeof window === 'undefined') return 'ws://localhost:8080';

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${protocol}//${window.location.host}`;
  }

  private createConnection(token: string): void {
    this.setState('connecting');

    const deviceInfo: DeviceInfo = {
      device_name: this.config.deviceName,
      device_type: this.config.deviceType as DeviceInfo['device_type'],
    };

    // Build WebSocket URL with query params
    const url = new URL(this.config.path, this.config.baseUrl);
    url.searchParams.set('token', token);
    url.searchParams.set('device_id', this._deviceId);
    url.searchParams.set('device_name', deviceInfo.device_name);
    url.searchParams.set('device_type', deviceInfo.device_type);

    try {
      this.ws = new WebSocket(url.toString());
      this.setupEventListeners(token);
    } catch (error) {
      console.error('[WebSocketClient] Failed to create WebSocket:', error);
      this.handleConnectionFailure(token);
    }
  }

  private setupEventListeners(token: string): void {
    if (!this.ws) return;

    this.ws.onopen = () => {
      console.log('[WebSocketClient] Connected');
      // State will be set to 'connected' when we receive the Connected message
    };

    this.ws.onclose = (event) => {
      console.log(`[WebSocketClient] Disconnected: code=${event.code}, reason=${event.reason}`);
      this.cleanup();

      if (this.shouldReconnect) {
        this.scheduleReconnect(token);
      } else {
        this.setState('disconnected');
      }
    };

    this.ws.onerror = (event) => {
      console.error('[WebSocketClient] Error:', event);
      this.events.onError?.(new Error('WebSocket error'));
    };

    this.ws.onmessage = (event) => {
      this.handleMessage(event.data);
    };
  }

  private handleMessage(data: string): void {
    try {
      const message = JSON.parse(data) as ServerMessage;

      switch (message.type) {
        case 'Connected':
          this._sessionId = message.payload.session_id;
          this._activeDeviceId = message.payload.active_device_id;
          this.setState('connected');
          this.startHeartbeat();
          this.flushMessageQueue();
          this.events.onConnected?.(message.payload);
          break;

        case 'Pong':
          this.lastPongTime = message.payload.server_time;
          // Verify heartbeat was acknowledged in time
          if (this.lastHeartbeatSent > 0) {
            const roundTrip = Date.now() - this.lastHeartbeatSent;
            if (roundTrip > this.heartbeatTimeoutMs) {
              console.warn(`[WebSocketClient] High latency detected: ${roundTrip}ms`);
            }
          }
          break;

        case 'DeviceConnected':
        case 'DeviceDisconnected':
        case 'DeviceList':
          // Update active device tracking
          if (message.type === 'DeviceList') {
            const activeDevice = message.payload.find((d) => d.is_active);
            this._activeDeviceId = activeDevice?.device_id ?? null;
          }
          break;

        case 'TransferAccepted':
          this._activeDeviceId = message.payload.to_device_id;
          break;

        case 'Error': {
          // Sanitize payload to prevent log injection (remove newlines/carriage returns)
          const sanitizedPayload = JSON.stringify(message.payload).replace(/[\r\n]+/g, ' ');
          console.error('[WebSocketClient] Server error:', sanitizedPayload);
          break;
        }
      }

      this.events.onMessage?.(message);
    } catch (error) {
      console.error('[WebSocketClient] Failed to parse message:', error);
    }
  }

  private handleConnectionFailure(token: string): void {
    this.cleanup();
    if (this.shouldReconnect) {
      this.scheduleReconnect(token);
    } else {
      this.setState('disconnected');
    }
  }

  private scheduleReconnect(token: string): void {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
    }

    this.reconnectAttempt++;
    this.setState('reconnecting');
    this.events.onReconnecting?.(this.reconnectAttempt);

    // Exponential backoff with jitter
    const delay = Math.min(
      this.config.initialReconnectDelay * Math.pow(2, this.reconnectAttempt - 1),
      this.config.maxReconnectDelay
    );
    const jitter = delay * 0.1 * Math.random();

    console.log(`[WebSocketClient] Reconnecting in ${Math.round(delay + jitter)}ms (attempt ${this.reconnectAttempt})`);

    this.reconnectTimeout = setTimeout(() => {
      this.createConnection(token);
    }, delay + jitter);
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();

    // Set timeout to 2x heartbeat interval
    this.heartbeatTimeoutMs = this.config.heartbeatInterval * 2;

    this.heartbeatInterval = setInterval(() => {
      if (this._state === 'connected') {
        // Check if previous heartbeat was acknowledged
        if (this.lastHeartbeatSent > 0 && this.lastPongTime < this.lastHeartbeatSent) {
          const elapsed = Date.now() - this.lastHeartbeatSent;
          if (elapsed > this.heartbeatTimeoutMs) {
            console.warn('[WebSocketClient] Heartbeat timeout, connection may be stale');
            // Force reconnect by closing the connection
            this.ws?.close(4000, 'Heartbeat timeout');
            return;
          }
        }

        this.lastHeartbeatSent = Date.now();
        this.send({ type: 'Heartbeat' });
      }
    }, this.config.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }

  private flushMessageQueue(): void {
    // Clear any pending retry since we're flushing now
    if (this.flushRetryTimeout) {
      clearTimeout(this.flushRetryTimeout);
      this.flushRetryTimeout = null;
    }

    // Limit messages per flush cycle to prevent tight loops
    const maxBatch = this.config.rateLimit;
    let messagesSent = 0;

    while (this.messageQueue.length > 0 && this._state === 'connected' && messagesSent < maxBatch) {
      const message = this.messageQueue[0]; // Peek at the message
      if (message) {
        const sent = this.send(message);
        if (sent) {
          this.messageQueue.shift(); // Remove only if sent successfully
          messagesSent++;
        } else {
          // Rate limited or connection lost - schedule retry after rate limit window resets
          this.flushRetryTimeout = setTimeout(() => this.flushMessageQueue(), 1000);
          return;
        }
      }
    }

    // If there are still messages and we're connected, schedule next batch
    if (this.messageQueue.length > 0 && this._state === 'connected') {
      this.flushRetryTimeout = setTimeout(() => this.flushMessageQueue(), 1000);
    }
  }

  private checkRateLimit(): boolean {
    const now = Date.now();
    const windowStart = now - 1000;

    // Remove old timestamps
    this.messageTimes = this.messageTimes.filter((t) => t > windowStart);

    return this.messageTimes.length < this.config.rateLimit;
  }

  private recordMessageTime(): void {
    this.messageTimes.push(Date.now());
  }

  private setState(state: ConnectionState): void {
    if (this._state !== state) {
      this._state = state;
      this.events.onStateChange?.(state);
    }
  }

  private cleanup(): void {
    this.stopHeartbeat();

    // Reset heartbeat tracking
    this.lastHeartbeatSent = 0;
    this.lastPongTime = 0;

    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    if (this.flushRetryTimeout) {
      clearTimeout(this.flushRetryTimeout);
      this.flushRetryTimeout = null;
    }

    if (this.ws) {
      this.ws.onopen = null;
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.onmessage = null;

      if (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING) {
        this.ws.close();
      }

      this.ws = null;
    }

    this._sessionId = null;
  }
}
