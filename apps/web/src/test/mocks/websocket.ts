/**
 * Mock WebSocket Infrastructure for Testing
 *
 * Provides a comprehensive MockWebSocket class that can be used to test
 * components relying on WebSocket connections, particularly for chat
 * and real-time sync functionality.
 *
 * Features:
 * - Simulates incoming messages from server
 * - Tracks all sent messages for assertions
 * - Tracks all instances for cleanup
 * - Supports readyState simulation
 * - Event handler support (onopen, onmessage, onclose, onerror)
 * - Easy reset for test isolation
 *
 * @example
 * ```typescript
 * import { MockWebSocket, mockWebSocketInstances, resetMockWebSocket } from './websocket';
 *
 * beforeEach(() => {
 *   resetMockWebSocket();
 *   vi.stubGlobal('WebSocket', MockWebSocket);
 * });
 *
 * it('handles incoming messages', () => {
 *   // Create a WebSocket instance (component under test does this)
 *   const ws = new WebSocket('ws://localhost:8080');
 *
 *   // Simulate server connected message
 *   MockWebSocket.simulateMessage({
 *     type: 'Connected',
 *     payload: { session_id: 'test-session', device_id: 'device-1' }
 *   });
 *
 *   // Assert on sent messages
 *   expect(MockWebSocket.sentMessages).toContainEqual(
 *     expect.objectContaining({ type: 'ChatSend' })
 *   );
 * });
 * ```
 */

import type { ServerMessage, ClientMessage } from '../../sync/types';

// =============================================================================
// Type Definitions
// =============================================================================

/** Event types supported by MockWebSocket */
export type MockWebSocketEventType = 'open' | 'message' | 'close' | 'error';

/** Event listener function type */
export type MockEventListener = (event: Event | MessageEvent | CloseEvent) => void;

/** Mock close event data */
export interface MockCloseEventInit {
  code?: number;
  reason?: string;
  wasClean?: boolean;
}

// =============================================================================
// MockWebSocket Class
// =============================================================================

/**
 * Mock WebSocket implementation for testing
 *
 * Provides a complete mock of the WebSocket API with additional testing utilities:
 * - Static methods to control all instances
 * - Message simulation for testing response handlers
 * - Sent message tracking for assertions
 */
export class MockWebSocket implements WebSocket {
  // =============================================================================
  // Static Properties & Methods (for controlling all instances)
  // =============================================================================

  /** All created MockWebSocket instances */
  private static _instances: MockWebSocket[] = [];

  /** Get all active instances */
  static get instances(): readonly MockWebSocket[] {
    return this._instances;
  }

  /** Get the most recently created instance */
  static get lastInstance(): MockWebSocket | undefined {
    return this._instances[this._instances.length - 1];
  }

  /** All messages sent across all instances */
  private static _sentMessages: ClientMessage[] = [];

  /** Get all sent messages for assertions */
  static get sentMessages(): readonly ClientMessage[] {
    return this._sentMessages;
  }

  /** Clear all sent messages */
  static clearSentMessages(): void {
    this._sentMessages = [];
  }

  /**
   * Simulate a message being received from the server
   *
   * This dispatches the message to all open instances.
   *
   * @param message - The ServerMessage to simulate receiving
   * @param targetInstance - Optional specific instance to target (defaults to all open)
   */
  static simulateMessage(message: ServerMessage, targetInstance?: MockWebSocket): void {
    const messageEvent = new MessageEvent('message', {
      data: JSON.stringify(message),
    });

    if (targetInstance) {
      if (targetInstance._readyState === WebSocket.OPEN) {
        targetInstance._dispatchEvent('message', messageEvent);
      }
    } else {
      // Dispatch to all open instances
      for (const instance of this._instances) {
        if (instance._readyState === WebSocket.OPEN) {
          instance._dispatchEvent('message', messageEvent);
        }
      }
    }
  }

  /**
   * Simulate the WebSocket connection being opened
   *
   * @param instance - Optional specific instance (defaults to last created)
   */
  static simulateOpen(instance?: MockWebSocket): void {
    const target = instance ?? this.lastInstance;
    if (target && target._readyState === WebSocket.CONNECTING) {
      target._readyState = WebSocket.OPEN;
      target._dispatchEvent('open', new Event('open'));
    }
  }

  /**
   * Simulate the WebSocket connection being closed
   *
   * @param options - Optional close event data
   * @param instance - Optional specific instance (defaults to last created)
   */
  static simulateClose(options: MockCloseEventInit = {}, instance?: MockWebSocket): void {
    const target = instance ?? this.lastInstance;
    if (target && target._readyState !== WebSocket.CLOSED) {
      target._readyState = WebSocket.CLOSED;
      const closeEvent = new CloseEvent('close', {
        code: options.code ?? 1000,
        reason: options.reason ?? '',
        wasClean: options.wasClean ?? true,
      });
      target._dispatchEvent('close', closeEvent);
    }
  }

  /**
   * Simulate a connection error
   *
   * @param instance - Optional specific instance (defaults to last created)
   */
  static simulateError(instance?: MockWebSocket): void {
    const target = instance ?? this.lastInstance;
    if (target) {
      target._dispatchEvent('error', new Event('error'));
    }
  }

  /**
   * Reset all mock state - call this in beforeEach/afterEach
   */
  static reset(): void {
    // Close all instances
    for (const instance of this._instances) {
      instance._readyState = WebSocket.CLOSED;
    }
    this._instances = [];
    this._sentMessages = [];
  }

  // =============================================================================
  // WebSocket Constants
  // =============================================================================

  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  readonly CONNECTING = MockWebSocket.CONNECTING;
  readonly OPEN = MockWebSocket.OPEN;
  readonly CLOSING = MockWebSocket.CLOSING;
  readonly CLOSED = MockWebSocket.CLOSED;

  // =============================================================================
  // Instance Properties
  // =============================================================================

  /** The URL passed to the constructor */
  readonly url: string;

  /** Binary type (not used in mocks but required by interface) */
  binaryType: BinaryType = 'blob';

  /** Amount of buffered data (always 0 in mock) */
  readonly bufferedAmount: number = 0;

  /** Protocol extensions (empty in mock) */
  readonly extensions: string = '';

  /** Subprotocol (empty in mock) */
  readonly protocol: string = '';

  // Event handlers
  onopen: ((this: WebSocket, ev: Event) => void) | null = null;
  onmessage: ((this: WebSocket, ev: MessageEvent) => void) | null = null;
  onclose: ((this: WebSocket, ev: CloseEvent) => void) | null = null;
  onerror: ((this: WebSocket, ev: Event) => void) | null = null;

  // Private state
  private _readyState: number = WebSocket.CONNECTING;
  private _eventListeners: Map<string, Set<MockEventListener>> = new Map();

  // Instance-specific sent messages
  private _instanceSentMessages: ClientMessage[] = [];

  /** Get messages sent by this specific instance */
  get instanceSentMessages(): readonly ClientMessage[] {
    return this._instanceSentMessages;
  }

  // =============================================================================
  // Constructor
  // =============================================================================

  /* eslint-disable @typescript-eslint/no-unused-vars */
  constructor(url: string | URL, _protocols?: string | string[]) {
    /* eslint-enable @typescript-eslint/no-unused-vars */
    this.url = typeof url === 'string' ? url : url.toString();

    // Register this instance
    MockWebSocket._instances.push(this);

    // By default, auto-open after a microtask (simulates async connection)
    // Tests can prevent this by calling reset() or closing the instance
    queueMicrotask(() => {
      if (this._readyState === WebSocket.CONNECTING) {
        this._readyState = WebSocket.OPEN;
        this._dispatchEvent('open', new Event('open'));
      }
    });
  }

  // =============================================================================
  // WebSocket Interface Implementation
  // =============================================================================

  get readyState(): number {
    return this._readyState;
  }

  /**
   * Send a message through the WebSocket
   *
   * The message is parsed and stored for test assertions.
   */
  send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
    if (this._readyState !== WebSocket.OPEN) {
      throw new DOMException('WebSocket is not open', 'InvalidStateError');
    }

    // Parse and store the message
    if (typeof data === 'string') {
      try {
        const message = JSON.parse(data) as ClientMessage;
        this._instanceSentMessages.push(message);
        MockWebSocket._sentMessages.push(message);
      } catch {
        // Not JSON - store as-is won't work for our typing, so skip
        console.warn('[MockWebSocket] Received non-JSON message:', data);
      }
    }
  }

  /**
   * Close the WebSocket connection
   */
  close(code?: number, reason?: string): void {
    if (this._readyState === WebSocket.CLOSED || this._readyState === WebSocket.CLOSING) {
      return;
    }

    this._readyState = WebSocket.CLOSING;

    // Simulate async close
    queueMicrotask(() => {
      this._readyState = WebSocket.CLOSED;
      const closeEvent = new CloseEvent('close', {
        code: code ?? 1000,
        reason: reason ?? '',
        wasClean: true,
      });
      this._dispatchEvent('close', closeEvent);
    });
  }

  addEventListener<K extends keyof WebSocketEventMap>(
    type: K,
    listener: (this: WebSocket, ev: WebSocketEventMap[K]) => void,
    options?: boolean | AddEventListenerOptions
  ): void;
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions
  ): void;
  /* eslint-disable @typescript-eslint/no-unused-vars */
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject | ((this: WebSocket, ev: Event) => void),
    _options?: boolean | AddEventListenerOptions
  ): void {
    /* eslint-enable @typescript-eslint/no-unused-vars */
    if (!this._eventListeners.has(type)) {
      this._eventListeners.set(type, new Set());
    }
    const fn =
      typeof listener === 'function'
        ? listener
        : (listener as EventListenerObject).handleEvent.bind(listener);
    this._eventListeners.get(type)!.add(fn as MockEventListener);
  }

  removeEventListener<K extends keyof WebSocketEventMap>(
    type: K,
    listener: (this: WebSocket, ev: WebSocketEventMap[K]) => void,
    options?: boolean | EventListenerOptions
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | EventListenerOptions
  ): void;
  /* eslint-disable @typescript-eslint/no-unused-vars */
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject | ((this: WebSocket, ev: Event) => void),
    _options?: boolean | EventListenerOptions
  ): void {
    /* eslint-enable @typescript-eslint/no-unused-vars */
    const listeners = this._eventListeners.get(type);
    if (listeners) {
      const fn =
        typeof listener === 'function'
          ? listener
          : (listener as EventListenerObject).handleEvent.bind(listener);
      listeners.delete(fn as MockEventListener);
    }
  }

  dispatchEvent(event: Event): boolean {
    this._dispatchEvent(event.type, event);
    return true;
  }

  // =============================================================================
  // Private Helpers
  // =============================================================================

  /**
   * Internal method to dispatch events to all listeners
   */
  private _dispatchEvent(type: string, event: Event | MessageEvent | CloseEvent): void {
    // Call the on* handler if set
    switch (type) {
      case 'open':
        this.onopen?.call(this, event);
        break;
      case 'message':
        this.onmessage?.call(this, event as MessageEvent);
        break;
      case 'close':
        this.onclose?.call(this, event as CloseEvent);
        break;
      case 'error':
        this.onerror?.call(this, event);
        break;
    }

    // Call all addEventListener handlers
    const listeners = this._eventListeners.get(type);
    if (listeners) {
      for (const listener of listeners) {
        listener(event);
      }
    }
  }
}

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Convenience function to get all MockWebSocket instances
 *
 * @deprecated Use MockWebSocket.instances instead
 */
export const mockWebSocketInstances = (): readonly MockWebSocket[] => MockWebSocket.instances;

/**
 * Reset all mock WebSocket state - call in beforeEach/afterEach
 */
export function resetMockWebSocket(): void {
  MockWebSocket.reset();
}

/**
 * Install MockWebSocket globally
 *
 * @returns Cleanup function to restore original WebSocket
 */
export function installMockWebSocket(): () => void {
  const original = globalThis.WebSocket;
  globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket;

  return () => {
    globalThis.WebSocket = original;
    MockWebSocket.reset();
  };
}

// =============================================================================
// Test Helpers for Common Scenarios
// =============================================================================

/**
 * Create a mock Connected message payload
 */
export function createConnectedMessage(
  overrides: Partial<{ session_id: string; device_id: string; active_device_id: string | null }> = {}
): ServerMessage {
  return {
    type: 'Connected',
    payload: {
      session_id: overrides.session_id ?? 'test-session-123',
      device_id: overrides.device_id ?? 'test-device-456',
      active_device_id: overrides.active_device_id ?? null,
    },
  };
}

/**
 * Create a mock ChatToken message for streaming responses
 */
export function createChatTokenMessage(
  conversationId: string,
  token: string,
  isFinal: boolean = false
): ServerMessage {
  return {
    type: 'ChatToken',
    payload: {
      conversation_id: conversationId,
      token,
      is_final: isFinal,
    },
  };
}

/**
 * Create a mock ChatComplete message
 */
export function createChatCompleteMessage(
  conversationId: string,
  fullResponse: string,
  options: {
    messageId?: string;
    actions?: ServerMessage extends { type: 'ChatComplete'; payload: infer P }
      ? P extends { actions: infer A }
        ? A
        : never
      : never;
  } = {}
): ServerMessage {
  return {
    type: 'ChatComplete',
    payload: {
      conversation_id: conversationId,
      message_id: options.messageId ?? `msg-${Date.now()}`,
      full_response: fullResponse,
      actions: options.actions ?? [],
      created_at: new Date().toISOString(),
    },
  };
}

/**
 * Create a mock ChatError message
 */
export function createChatErrorMessage(
  conversationId: string | null,
  error: string,
  code?: string
): ServerMessage {
  return {
    type: 'ChatError',
    payload: {
      conversation_id: conversationId,
      error,
      code,
    },
  };
}

/**
 * Create a mock Pong message
 */
export function createPongMessage(serverTime: number = Date.now()): ServerMessage {
  return {
    type: 'Pong',
    payload: {
      server_time: serverTime,
    },
  };
}

/**
 * Create a mock DeviceList message
 */
export function createDeviceListMessage(
  devices: Array<{
    device_id: string;
    device_name: string;
    device_type?: 'web' | 'desktop' | 'mobile' | 'tablet' | 'speaker' | 'unknown';
    is_active?: boolean;
    volume?: number;
  }> = []
): ServerMessage {
  return {
    type: 'DeviceList',
    payload: devices.map((d) => ({
      device_id: d.device_id,
      device_name: d.device_name,
      device_type: d.device_type ?? 'web',
      is_active: d.is_active ?? false,
      current_track: null,
      volume: d.volume ?? 0.75,
      last_seen: Date.now(),
    })),
  };
}

/**
 * Create a mock PlaybackSync message
 */
export function createPlaybackSyncMessage(
  overrides: Partial<{
    track_id: string | null;
    is_playing: boolean;
    position_ms: number;
    volume: number;
    is_muted: boolean;
    shuffle: boolean;
    repeat: 'off' | 'track' | 'queue';
  }> = {}
): ServerMessage {
  return {
    type: 'PlaybackSync',
    payload: {
      track_id: overrides.track_id ?? 'track-123',
      is_playing: overrides.is_playing ?? true,
      position_ms: overrides.position_ms ?? 30000,
      timestamp: Date.now(),
      volume: overrides.volume ?? 0.75,
      is_muted: overrides.is_muted ?? false,
      shuffle: overrides.shuffle ?? false,
      repeat: overrides.repeat ?? 'off',
    },
  };
}
