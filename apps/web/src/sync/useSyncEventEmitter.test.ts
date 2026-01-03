/**
 * useSyncEventEmitter Hook Tests
 *
 * Comprehensive tests for the sync event emitter hook covering:
 * - Enabled/disabled state behavior
 * - Connection state transitions (disconnected->connecting->connected, connected->disconnected)
 * - Error detection with isAuthError flag
 * - Reconnecting event with attempt count
 * - Cleanup on unmount
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useSyncEventEmitter } from './useSyncEventEmitter';
import { syncEvents } from './syncEvents';
import { useDeviceStore } from '../stores/deviceStore';
import { resetDeviceStore } from './test-utils';
import type { ConnectionState } from './types';

// =============================================================================
// Test Helpers
// =============================================================================

/** Captured event payloads for testing */
interface CapturedEvents {
  connected: Array<{ deviceId: string; sessionId: string; isReconnect: boolean }>;
  disconnected: Array<{ reason?: string; wasClean: boolean }>;
  reconnecting: Array<{ attempt: number; maxAttempts?: number }>;
  error: Array<{ message: string; code?: string; isAuthError: boolean }>;
}

/** Create a fresh event capture object */
function createEventCapture(): CapturedEvents {
  return {
    connected: [],
    disconnected: [],
    reconnecting: [],
    error: [],
  };
}

/** Subscribe to all relevant events and capture their payloads */
function captureEvents(capture: CapturedEvents): () => void {
  const unsubConnected = syncEvents.on('connected', (payload) => {
    capture.connected.push(payload);
  });
  const unsubDisconnected = syncEvents.on('disconnected', (payload) => {
    capture.disconnected.push(payload);
  });
  const unsubReconnecting = syncEvents.on('reconnecting', (payload) => {
    capture.reconnecting.push(payload);
  });
  const unsubError = syncEvents.on('error', (payload) => {
    capture.error.push(payload);
  });

  return () => {
    unsubConnected();
    unsubDisconnected();
    unsubReconnecting();
    unsubError();
  };
}

/** Helper to set device store connection state */
function setConnectionState(state: ConnectionState): void {
  useDeviceStore.setState({ connectionState: state });
}

/** Helper to set device store error */
function setConnectionError(error: string | null): void {
  useDeviceStore.setState({ lastError: error });
}

/** Helper to set device ID and session ID */
function setDeviceInfo(deviceId: string, sessionId: string | null): void {
  useDeviceStore.setState({ deviceId, sessionId });
}

// =============================================================================
// Tests
// =============================================================================

describe('useSyncEventEmitter', () => {
  let eventCapture: CapturedEvents;
  let unsubscribeCapture: () => void;

  beforeEach(() => {
    // Reset stores and event emitter
    resetDeviceStore();
    syncEvents.clear();

    // Set initial disconnected state for clean slate
    useDeviceStore.setState({
      connectionState: 'disconnected',
      lastError: null,
      deviceId: 'test-device-id',
      sessionId: null,
    });

    // Set up event capture
    eventCapture = createEventCapture();
    unsubscribeCapture = captureEvents(eventCapture);
  });

  afterEach(() => {
    unsubscribeCapture();
    syncEvents.clear();
    vi.clearAllMocks();
  });

  // ===========================================================================
  // Enabled/Disabled State Tests
  // ===========================================================================

  describe('enabled/disabled state', () => {
    it('emits events when enabled is true (default)', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      // Set device info before connecting
      act(() => {
        setDeviceInfo('test-device-id', 'test-session-id');
      });

      // Transition to connected
      act(() => {
        setConnectionState('connected');
      });

      expect(eventCapture.connected).toHaveLength(1);
      expect(eventCapture.connected[0]).toEqual({
        deviceId: 'test-device-id',
        sessionId: 'test-session-id',
        isReconnect: false,
      });

      unmount();
    });

    it('does not emit events when enabled is false', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter({ enabled: false }));

      // Set device info
      act(() => {
        setDeviceInfo('test-device-id', 'test-session-id');
      });

      // Transition to connected
      act(() => {
        setConnectionState('connected');
      });

      expect(eventCapture.connected).toHaveLength(0);

      // Transition to disconnected
      act(() => {
        setConnectionState('disconnected');
      });

      expect(eventCapture.disconnected).toHaveLength(0);

      unmount();
    });

    it('does not emit error events when disabled', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter({ enabled: false }));

      act(() => {
        setConnectionError('Test error');
      });

      expect(eventCapture.error).toHaveLength(0);

      unmount();
    });

    it('resumes emitting events when enabled changes from false to true', () => {
      let enabled = false;
      const { rerender, unmount } = renderHook(() =>
        useSyncEventEmitter({ enabled })
      );

      // Set device info
      act(() => {
        setDeviceInfo('test-device-id', 'test-session-id');
      });

      // Connect while disabled - no event
      act(() => {
        setConnectionState('connected');
      });
      expect(eventCapture.connected).toHaveLength(0);

      // Enable and change state
      enabled = true;
      rerender();

      // Disconnect - now should emit
      act(() => {
        setConnectionState('disconnected');
      });
      expect(eventCapture.disconnected).toHaveLength(1);

      unmount();
    });
  });

  // ===========================================================================
  // Connection State Transition Tests
  // ===========================================================================

  describe('connection state transitions', () => {
    describe('disconnected -> connecting -> connected', () => {
      it('emits connected event when transitioning from disconnected to connected', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        act(() => {
          setDeviceInfo('device-1', 'session-1');
        });

        // Start from disconnected (already set in beforeEach)
        expect(useDeviceStore.getState().connectionState).toBe('disconnected');

        // Transition to connected
        act(() => {
          setConnectionState('connected');
        });

        expect(eventCapture.connected).toHaveLength(1);
        expect(eventCapture.connected[0]).toEqual({
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        });

        unmount();
      });

      it('emits connected event when transitioning from connecting to connected', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        act(() => {
          setDeviceInfo('device-1', 'session-1');
        });

        // Transition to connecting first
        act(() => {
          setConnectionState('connecting');
        });

        expect(eventCapture.connected).toHaveLength(0);

        // Then transition to connected
        act(() => {
          setConnectionState('connected');
        });

        expect(eventCapture.connected).toHaveLength(1);
        expect(eventCapture.connected[0]?.isReconnect).toBe(false);

        unmount();
      });

      it('marks isReconnect as true after first connection', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        act(() => {
          setDeviceInfo('device-1', 'session-1');
        });

        // First connection
        act(() => {
          setConnectionState('connected');
        });

        expect(eventCapture.connected).toHaveLength(1);
        expect(eventCapture.connected[0]?.isReconnect).toBe(false);

        // Disconnect
        act(() => {
          setConnectionState('disconnected');
        });

        // Reconnect
        act(() => {
          useDeviceStore.setState({ sessionId: 'session-2' });
          setConnectionState('connected');
        });

        expect(eventCapture.connected).toHaveLength(2);
        expect(eventCapture.connected[1]?.isReconnect).toBe(true);

        unmount();
      });
    });

    describe('connected -> disconnected', () => {
      it('emits disconnected event when transitioning from connected to disconnected', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        act(() => {
          setDeviceInfo('device-1', 'session-1');
        });

        // First establish connection
        act(() => {
          setConnectionState('connected');
        });

        // Then disconnect
        act(() => {
          setConnectionState('disconnected');
        });

        expect(eventCapture.disconnected).toHaveLength(1);
        expect(eventCapture.disconnected[0]).toEqual({
          reason: undefined,
          wasClean: false,
        });

        unmount();
      });

      it('does not emit disconnected event when transitioning from connecting to disconnected', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        // Transition to connecting
        act(() => {
          setConnectionState('connecting');
        });

        // Then disconnect (connection failed)
        act(() => {
          setConnectionState('disconnected');
        });

        // No disconnected event because we were never fully connected
        expect(eventCapture.disconnected).toHaveLength(0);

        unmount();
      });

      it('does not emit disconnected event from initial disconnected state', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        // Already disconnected, set it again (no-op state change)
        act(() => {
          setConnectionState('disconnected');
        });

        expect(eventCapture.disconnected).toHaveLength(0);

        unmount();
      });
    });

    describe('reconnecting state', () => {
      it('emits reconnecting event when entering reconnecting state', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        act(() => {
          setDeviceInfo('device-1', 'session-1');
        });

        // First connect
        act(() => {
          setConnectionState('connected');
        });

        // Then enter reconnecting state
        act(() => {
          setConnectionState('reconnecting');
        });

        expect(eventCapture.reconnecting).toHaveLength(1);
        expect(eventCapture.reconnecting[0]).toEqual({
          attempt: 1,
        });

        unmount();
      });

      it('does not emit duplicate reconnecting events if already in reconnecting state', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        act(() => {
          setDeviceInfo('device-1', 'session-1');
        });

        // First connect
        act(() => {
          setConnectionState('connected');
        });

        // Enter reconnecting state
        act(() => {
          setConnectionState('reconnecting');
        });

        expect(eventCapture.reconnecting).toHaveLength(1);

        // Set reconnecting again (should not emit duplicate)
        act(() => {
          setConnectionState('reconnecting');
        });

        expect(eventCapture.reconnecting).toHaveLength(1);

        unmount();
      });

      it('emits reconnecting event from disconnected state', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        // Transition directly to reconnecting from disconnected
        act(() => {
          setConnectionState('reconnecting');
        });

        expect(eventCapture.reconnecting).toHaveLength(1);

        unmount();
      });
    });

    describe('full connection lifecycle', () => {
      it('handles complete connect -> disconnect -> reconnect cycle', () => {
        const { unmount } = renderHook(() => useSyncEventEmitter());

        act(() => {
          setDeviceInfo('device-1', 'session-1');
        });

        // Initial connection
        act(() => {
          setConnectionState('connected');
        });

        expect(eventCapture.connected).toHaveLength(1);
        expect(eventCapture.connected[0]?.isReconnect).toBe(false);

        // Disconnect
        act(() => {
          setConnectionState('disconnected');
        });

        expect(eventCapture.disconnected).toHaveLength(1);

        // Reconnecting
        act(() => {
          setConnectionState('reconnecting');
        });

        expect(eventCapture.reconnecting).toHaveLength(1);

        // Reconnected
        act(() => {
          useDeviceStore.setState({ sessionId: 'session-2' });
          setConnectionState('connected');
        });

        expect(eventCapture.connected).toHaveLength(2);
        expect(eventCapture.connected[1]?.isReconnect).toBe(true);
        expect(eventCapture.connected[1]?.sessionId).toBe('session-2');

        unmount();
      });
    });
  });

  // ===========================================================================
  // Error Detection Tests
  // ===========================================================================

  describe('error detection', () => {
    it('emits error event when connection error occurs', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setConnectionError('Connection failed');
      });

      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]).toEqual({
        message: 'Connection failed',
        isAuthError: false,
      });

      unmount();
    });

    it('detects auth error with "auth" in message', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setConnectionError('Authentication failed');
      });

      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.isAuthError).toBe(true);

      unmount();
    });

    it('detects auth error with "unauthorized" in message', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setConnectionError('Unauthorized access');
      });

      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.isAuthError).toBe(true);

      unmount();
    });

    it('detects auth error with "token" in message', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setConnectionError('Token expired');
      });

      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.isAuthError).toBe(true);

      unmount();
    });

    it('detects auth error with "401" in message', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setConnectionError('HTTP 401: Not authorized');
      });

      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.isAuthError).toBe(true);

      unmount();
    });

    it('detects auth error case-insensitively', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setConnectionError('AUTH_FAILED: Invalid credentials');
      });

      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.isAuthError).toBe(true);

      unmount();
    });

    it('marks non-auth errors correctly', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setConnectionError('Network timeout');
      });

      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.isAuthError).toBe(false);

      act(() => {
        setConnectionError('Server error 500');
      });

      expect(eventCapture.error).toHaveLength(2);
      expect(eventCapture.error[1]?.isAuthError).toBe(false);

      unmount();
    });

    it('does not emit error event when error is null', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      // Set an error first
      act(() => {
        setConnectionError('Some error');
      });

      expect(eventCapture.error).toHaveLength(1);

      // Clear the error
      act(() => {
        setConnectionError(null);
      });

      // Should not emit another event
      expect(eventCapture.error).toHaveLength(1);

      unmount();
    });

    it('emits new error event when error changes', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setConnectionError('First error');
      });

      expect(eventCapture.error).toHaveLength(1);

      act(() => {
        setConnectionError('Second error');
      });

      expect(eventCapture.error).toHaveLength(2);
      expect(eventCapture.error[1]?.message).toBe('Second error');

      unmount();
    });
  });

  // ===========================================================================
  // Cleanup on Unmount Tests
  // ===========================================================================

  describe('cleanup on unmount', () => {
    it('does not emit events after unmount', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', 'session-1');
      });

      // Connect while mounted
      act(() => {
        setConnectionState('connected');
      });

      expect(eventCapture.connected).toHaveLength(1);

      // Unmount
      unmount();

      // Try to trigger state changes after unmount
      act(() => {
        setConnectionState('disconnected');
      });

      // No new events should be emitted
      expect(eventCapture.connected).toHaveLength(1);
      expect(eventCapture.disconnected).toHaveLength(0);

      act(() => {
        setConnectionError('Error after unmount');
      });

      expect(eventCapture.error).toHaveLength(0);
    });

    it('resets internal state on remount', () => {
      // First mount - connect and then unmount
      const { unmount: unmount1 } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', 'session-1');
        setConnectionState('connected');
      });

      expect(eventCapture.connected).toHaveLength(1);
      expect(eventCapture.connected[0]?.isReconnect).toBe(false);

      unmount1();

      // Reset store to disconnected state
      act(() => {
        setConnectionState('disconnected');
      });

      // Clear captured events for clean comparison
      eventCapture.connected = [];

      // Second mount - should start fresh
      const { unmount: unmount2 } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', 'session-2');
        setConnectionState('connected');
      });

      // Should be first connection for this hook instance (not reconnect)
      expect(eventCapture.connected).toHaveLength(1);
      expect(eventCapture.connected[0]?.isReconnect).toBe(false);

      unmount2();
    });

    it('multiple instances track their own state independently', () => {
      // Mount first instance
      const { unmount: unmount1 } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', 'session-1');
        setConnectionState('connected');
      });

      // First instance emits connected event
      expect(eventCapture.connected).toHaveLength(1);

      // Mount second instance while first is still mounted
      const { unmount: unmount2 } = renderHook(() => useSyncEventEmitter());

      // Disconnect - both instances respond to the store change
      act(() => {
        setConnectionState('disconnected');
      });

      // Both instances emit disconnect event since both are watching the same store
      // and both have seen a connected->disconnected transition
      expect(eventCapture.disconnected).toHaveLength(2);

      // Reconnect
      act(() => {
        useDeviceStore.setState({ sessionId: 'session-2' });
        setConnectionState('connected');
      });

      // Both instances emit connected events
      // Instance 1: isReconnect = true (saw first connection)
      // Instance 2: isReconnect = false (mounted after first connection, didn't see it)
      expect(eventCapture.connected).toHaveLength(3);

      // First connection (instance 1)
      expect(eventCapture.connected[0]?.isReconnect).toBe(false);
      // Reconnection events from both instances
      expect(eventCapture.connected[1]?.isReconnect).toBe(true); // instance 1
      expect(eventCapture.connected[2]?.isReconnect).toBe(false); // instance 2 (first connection for this instance)

      unmount1();
      unmount2();
    });
  });

  // ===========================================================================
  // Edge Cases
  // ===========================================================================

  describe('edge cases', () => {
    it('handles rapid state changes', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', 'session-1');
      });

      // Rapid state changes
      act(() => {
        setConnectionState('connecting');
        setConnectionState('connected');
        setConnectionState('disconnected');
        setConnectionState('reconnecting');
        setConnectionState('connected');
      });

      // Should track all transitions
      expect(eventCapture.connected.length).toBeGreaterThanOrEqual(1);

      unmount();
    });

    it('handles empty device ID gracefully', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('', 'session-1');
        setConnectionState('connected');
      });

      expect(eventCapture.connected).toHaveLength(1);
      expect(eventCapture.connected[0]?.deviceId).toBe('');

      unmount();
    });

    it('handles null session ID', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', null);
        setConnectionState('connected');
      });

      expect(eventCapture.connected).toHaveLength(1);
      expect(eventCapture.connected[0]?.sessionId).toBe('');

      unmount();
    });

    it('handles very long error messages', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      const longError = 'A'.repeat(10000);

      act(() => {
        setConnectionError(longError);
      });

      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.message).toBe(longError);
      expect(eventCapture.error[0]?.isAuthError).toBe(false);

      unmount();
    });

    it('handles special characters in error messages', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setConnectionError('Error: <script>alert("xss")</script> & more');
      });

      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.message).toBe(
        'Error: <script>alert("xss")</script> & more'
      );

      unmount();
    });

    it('correctly identifies auth errors in complex messages', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      // Test various auth error patterns
      const authPatterns = [
        'JWT token has expired',
        'User is not authorized to access this resource',
        'Authentication required',
        'Error 401: Please log in',
        'Invalid authentication credentials',
        'Access token invalid',
        'UNAUTHORIZED_REQUEST',
      ];

      for (const pattern of authPatterns) {
        act(() => {
          setConnectionError(pattern);
        });
      }

      expect(eventCapture.error).toHaveLength(authPatterns.length);
      eventCapture.error.forEach((err) => {
        expect(err.isAuthError).toBe(true);
      });

      unmount();
    });
  });

  // ===========================================================================
  // Reconnect Attempt Tracking Tests
  // ===========================================================================

  describe('reconnect attempt tracking', () => {
    it('emits reconnecting event with attempt number', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', 'session-1');
      });

      // Connect first
      act(() => {
        setConnectionState('connected');
      });

      // Start reconnecting
      act(() => {
        setConnectionState('reconnecting');
      });

      expect(eventCapture.reconnecting).toHaveLength(1);
      expect(eventCapture.reconnecting[0]?.attempt).toBe(1);

      // Fail reconnect, go back to disconnected
      act(() => {
        setConnectionState('disconnected');
      });

      // Try reconnecting again
      act(() => {
        setConnectionState('reconnecting');
      });

      expect(eventCapture.reconnecting).toHaveLength(2);
      expect(eventCapture.reconnecting[1]?.attempt).toBe(1);

      unmount();
    });

    it('handles reconnecting from multiple source states', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', 'session-1');
      });

      // From disconnected - first transition to reconnecting
      act(() => {
        setConnectionState('reconnecting');
      });

      expect(eventCapture.reconnecting).toHaveLength(1);

      // Need to leave reconnecting state before we can trigger another event
      // First connect, then go back to reconnecting
      act(() => {
        setConnectionState('connected');
      });

      act(() => {
        setConnectionState('reconnecting');
      });

      expect(eventCapture.reconnecting).toHaveLength(2);

      unmount();
    });
  });

  // ===========================================================================
  // Integration Tests
  // ===========================================================================

  describe('integration scenarios', () => {
    it('handles realistic connection failure scenario', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', 'session-1');
      });

      // Initial connection attempt
      act(() => {
        setConnectionState('connecting');
      });

      // Connection successful
      act(() => {
        setConnectionState('connected');
      });

      expect(eventCapture.connected).toHaveLength(1);
      expect(eventCapture.connected[0]?.isReconnect).toBe(false);

      // Network drops
      act(() => {
        setConnectionError('WebSocket connection closed unexpectedly');
        setConnectionState('disconnected');
      });

      expect(eventCapture.disconnected).toHaveLength(1);
      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.isAuthError).toBe(false);

      // Auto-reconnect attempts
      act(() => {
        setConnectionState('reconnecting');
      });

      expect(eventCapture.reconnecting).toHaveLength(1);

      // Reconnection successful
      act(() => {
        useDeviceStore.setState({ sessionId: 'session-2', lastError: null });
        setConnectionState('connected');
      });

      expect(eventCapture.connected).toHaveLength(2);
      expect(eventCapture.connected[1]?.isReconnect).toBe(true);

      unmount();
    });

    it('handles auth failure during connection', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      act(() => {
        setDeviceInfo('device-1', null);
      });

      // Attempt to connect
      act(() => {
        setConnectionState('connecting');
      });

      // Auth failure
      act(() => {
        setConnectionError('401 Unauthorized: Token expired');
        setConnectionState('disconnected');
      });

      expect(eventCapture.connected).toHaveLength(0);
      expect(eventCapture.error).toHaveLength(1);
      expect(eventCapture.error[0]?.isAuthError).toBe(true);
      expect(eventCapture.error[0]?.message).toBe('401 Unauthorized: Token expired');

      unmount();
    });

    it('handles multiple error types in sequence', () => {
      const { unmount } = renderHook(() => useSyncEventEmitter());

      // Network error
      act(() => {
        setConnectionError('Network unreachable');
      });

      expect(eventCapture.error[0]?.isAuthError).toBe(false);

      // Clear and set auth error
      act(() => {
        setConnectionError(null);
      });

      act(() => {
        setConnectionError('Authentication failed');
      });

      expect(eventCapture.error[1]?.isAuthError).toBe(true);

      // Another network error
      act(() => {
        setConnectionError(null);
      });

      act(() => {
        setConnectionError('Connection timed out');
      });

      expect(eventCapture.error[2]?.isAuthError).toBe(false);

      expect(eventCapture.error).toHaveLength(3);

      unmount();
    });
  });
});
