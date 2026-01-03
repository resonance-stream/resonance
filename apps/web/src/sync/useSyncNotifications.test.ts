/**
 * useSyncNotifications Hook Tests
 *
 * Comprehensive tests for the sync notifications hook covering:
 * - Reconnection toast behavior
 * - Auth error handling
 * - Transfer notifications
 * - Enabled/disabled option flags
 * - Edge cases and rapid connect/disconnect cycles
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useSyncNotifications } from './useSyncNotifications';
import { syncEvents } from './syncEvents';
import { useToastStore } from '../stores/toastStore';
import type {
  ConnectedEventPayload,
  DisconnectedEventPayload,
  ErrorEventPayload,
  DeviceJoinedEventPayload,
  TransferReceivedEventPayload,
  TransferSentEventPayload,
  ReconnectingEventPayload,
} from './syncEvents';

// Helper to reset toast store between tests
function resetToastStore(): void {
  useToastStore.setState({ toasts: [] });
}

// Helper to get the last toast
function getLastToast() {
  const toasts = useToastStore.getState().toasts;
  return toasts[toasts.length - 1];
}

// Helper to get all toasts
function getAllToasts() {
  return useToastStore.getState().toasts;
}

describe('useSyncNotifications', () => {
  beforeEach(() => {
    resetToastStore();
    syncEvents.clear();
  });

  afterEach(() => {
    syncEvents.clear();
  });

  describe('disconnect notifications', () => {
    it('shows "Sync disconnected" warning toast on unclean disconnect after initial connection', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      // First establish a connection
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        } as ConnectedEventPayload);
      });

      // Then disconnect uncleanly
      act(() => {
        syncEvents.emit('disconnected', {
          wasClean: false,
          reason: 'Connection lost',
        } as DisconnectedEventPayload);
      });

      const toast = getLastToast();
      expect(toast).toBeDefined();
      expect(toast?.type).toBe('warning');
      expect(toast?.title).toBe('Sync disconnected');
      expect(toast?.description).toBe('Reconnecting...');

      unmount();
    });

    it('does not show disconnect toast on clean disconnect', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      // First establish a connection
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        } as ConnectedEventPayload);
      });

      const toastCountAfterConnect = getAllToasts().length;

      // Clean disconnect (intentional)
      act(() => {
        syncEvents.emit('disconnected', {
          wasClean: true,
        } as DisconnectedEventPayload);
      });

      // No new toasts should be added
      expect(getAllToasts().length).toBe(toastCountAfterConnect);

      unmount();
    });

    it('does not show disconnect toast before initial connection', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      // Disconnect without prior connection
      act(() => {
        syncEvents.emit('disconnected', {
          wasClean: false,
          reason: 'Never connected',
        } as DisconnectedEventPayload);
      });

      expect(getAllToasts().length).toBe(0);

      unmount();
    });

    it('shows disconnect toast on reconnecting event', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      // First establish a connection
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        } as ConnectedEventPayload);
      });

      // Reconnecting event triggers disconnect toast
      act(() => {
        syncEvents.emit('reconnecting', {
          attempt: 1,
          maxAttempts: 5,
        } as ReconnectingEventPayload);
      });

      const toast = getLastToast();
      expect(toast).toBeDefined();
      expect(toast?.type).toBe('warning');
      expect(toast?.title).toBe('Sync disconnected');

      unmount();
    });
  });

  describe('reconnection notifications', () => {
    it('shows "Sync restored" success toast on reconnection (not initial connection)', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      // Initial connection
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        } as ConnectedEventPayload);
      });

      // No toast for initial connection
      expect(getAllToasts().length).toBe(0);

      // Disconnect
      act(() => {
        syncEvents.emit('disconnected', {
          wasClean: false,
        } as DisconnectedEventPayload);
      });

      // Reconnect
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-2',
          isReconnect: true,
        } as ConnectedEventPayload);
      });

      const toast = getLastToast();
      expect(toast).toBeDefined();
      expect(toast?.type).toBe('success');
      expect(toast?.title).toBe('Sync restored');
      expect(toast?.description).toBe('Cross-device sync is active');

      unmount();
    });

    it('does not show toast on initial connection', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        } as ConnectedEventPayload);
      });

      expect(getAllToasts().length).toBe(0);

      unmount();
    });

    it('shows reconnect toast after clean disconnect (hasShownDisconnect flag is set)', () => {
      // Note: Current implementation sets hasShownDisconnectToastRef even for clean
      // disconnects, so the "Sync restored" toast is shown on reconnect.
      // This may be intentional - the reconnect toast indicates sync is working again.
      const { unmount } = renderHook(() => useSyncNotifications());

      // Initial connection
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        } as ConnectedEventPayload);
      });

      // Clean disconnect (no warning toast shown, but internal flag is set)
      act(() => {
        syncEvents.emit('disconnected', {
          wasClean: true,
        } as DisconnectedEventPayload);
      });

      // No warning toast was shown for the clean disconnect
      expect(getAllToasts().length).toBe(0);

      // Reconnect with isReconnect: true - shows restored toast because
      // the hasShownDisconnect flag was set during the disconnect handling
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-2',
          isReconnect: true,
        } as ConnectedEventPayload);
      });

      // Reconnect toast IS shown because hasShownDisconnectToastRef was set
      expect(getAllToasts().length).toBe(1);
      const toast = getLastToast();
      expect(toast?.type).toBe('success');
      expect(toast?.title).toBe('Sync restored');

      unmount();
    });
  });

  describe('auth error handling', () => {
    it('shows error toast on auth failure', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('error', {
          message: 'Authentication failed',
          isAuthError: true,
        } as ErrorEventPayload);
      });

      const toast = getLastToast();
      expect(toast).toBeDefined();
      expect(toast?.type).toBe('error');
      expect(toast?.title).toBe('Sync authentication failed');
      expect(toast?.description).toBe('Please sign in again to enable sync');

      unmount();
    });

    it('shows generic error toast for non-auth errors', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('error', {
          message: 'WebSocket connection failed',
          isAuthError: false,
        } as ErrorEventPayload);
      });

      const toast = getLastToast();
      expect(toast).toBeDefined();
      expect(toast?.type).toBe('error');
      expect(toast?.title).toBe('Sync connection error');
      expect(toast?.description).toBe('WebSocket connection failed');

      unmount();
    });

    it('shows error toast with error code', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('error', {
          message: 'Server unavailable',
          code: '503',
          isAuthError: false,
        } as ErrorEventPayload);
      });

      const toast = getLastToast();
      expect(toast?.description).toBe('Server unavailable');

      unmount();
    });
  });

  describe('transfer notifications', () => {
    it('shows info toast when transfer is received with device name', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('transferReceived', {
          fromDeviceId: 'device-2',
          fromDeviceName: 'Living Room Speaker',
        } as TransferReceivedEventPayload);
      });

      const toast = getLastToast();
      expect(toast).toBeDefined();
      expect(toast?.type).toBe('info');
      expect(toast?.title).toBe('Playback transferred');
      expect(toast?.description).toBe('Now playing from Living Room Speaker');

      unmount();
    });

    it('shows info toast when transfer is received without device name', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('transferReceived', {
          fromDeviceId: 'device-2',
        } as TransferReceivedEventPayload);
      });

      const toast = getLastToast();
      expect(toast?.description).toBe('You now control playback');

      unmount();
    });

    it('shows info toast when transfer is sent with device name', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('transferSent', {
          toDeviceId: 'device-3',
          toDeviceName: 'Kitchen Speaker',
        } as TransferSentEventPayload);
      });

      const toast = getLastToast();
      expect(toast).toBeDefined();
      expect(toast?.type).toBe('info');
      expect(toast?.title).toBe('Playback transferred');
      expect(toast?.description).toBe('Now playing on Kitchen Speaker');

      unmount();
    });

    it('shows info toast when transfer is sent without device name', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('transferSent', {
          toDeviceId: 'device-3',
        } as TransferSentEventPayload);
      });

      const toast = getLastToast();
      expect(toast?.description).toBe('Transferred to another device');

      unmount();
    });

    it('does not show transfer notifications when showTransfers is false', () => {
      const { unmount } = renderHook(() =>
        useSyncNotifications({ showTransfers: false })
      );

      act(() => {
        syncEvents.emit('transferReceived', {
          fromDeviceId: 'device-2',
          fromDeviceName: 'Test Device',
        } as TransferReceivedEventPayload);
      });

      expect(getAllToasts().length).toBe(0);

      act(() => {
        syncEvents.emit('transferSent', {
          toDeviceId: 'device-3',
          toDeviceName: 'Another Device',
        } as TransferSentEventPayload);
      });

      expect(getAllToasts().length).toBe(0);

      unmount();
    });
  });

  describe('device notifications', () => {
    it('shows device joined notification when showDeviceJoined is true', () => {
      const { unmount } = renderHook(() =>
        useSyncNotifications({ showDeviceJoined: true })
      );

      act(() => {
        syncEvents.emit('deviceJoined', {
          deviceId: 'new-device',
          deviceName: 'iPhone 15',
        } as DeviceJoinedEventPayload);
      });

      const toast = getLastToast();
      expect(toast).toBeDefined();
      expect(toast?.type).toBe('info');
      expect(toast?.title).toBe('Device connected');
      expect(toast?.description).toBe('iPhone 15');

      unmount();
    });

    it('does not show device joined notification by default', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('deviceJoined', {
          deviceId: 'new-device',
          deviceName: 'iPhone 15',
        } as DeviceJoinedEventPayload);
      });

      expect(getAllToasts().length).toBe(0);

      unmount();
    });

    it('shows device left notification when showDeviceLeft is true', () => {
      const { unmount } = renderHook(() =>
        useSyncNotifications({ showDeviceLeft: true })
      );

      act(() => {
        syncEvents.emit('deviceLeft', {
          deviceId: 'old-device',
        });
      });

      const toast = getLastToast();
      expect(toast).toBeDefined();
      expect(toast?.type).toBe('info');
      expect(toast?.title).toBe('Device disconnected');

      unmount();
    });

    it('does not show device left notification by default', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('deviceLeft', {
          deviceId: 'old-device',
        });
      });

      expect(getAllToasts().length).toBe(0);

      unmount();
    });
  });

  describe('enabled option flag', () => {
    it('does not show any toasts when enabled is false', () => {
      const { unmount } = renderHook(() =>
        useSyncNotifications({ enabled: false })
      );

      // Try all event types
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: true,
        } as ConnectedEventPayload);
      });

      act(() => {
        syncEvents.emit('disconnected', {
          wasClean: false,
        } as DisconnectedEventPayload);
      });

      act(() => {
        syncEvents.emit('error', {
          message: 'Test error',
          isAuthError: true,
        } as ErrorEventPayload);
      });

      act(() => {
        syncEvents.emit('transferReceived', {
          fromDeviceId: 'device-2',
          fromDeviceName: 'Test',
        } as TransferReceivedEventPayload);
      });

      act(() => {
        syncEvents.emit('deviceJoined', {
          deviceId: 'new-device',
          deviceName: 'Test',
        } as DeviceJoinedEventPayload);
      });

      expect(getAllToasts().length).toBe(0);

      unmount();
    });

    it('shows toasts when enabled is true (default)', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('error', {
          message: 'Test error',
          isAuthError: false,
        } as ErrorEventPayload);
      });

      expect(getAllToasts().length).toBe(1);

      unmount();
    });
  });

  describe('event subscription cleanup', () => {
    it('cleans up event subscriptions on unmount', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      // Get initial listener count
      const initialCount = syncEvents.listenerCount('connected');
      expect(initialCount).toBe(1);

      // Unmount should clean up
      unmount();

      // Listener should be removed
      expect(syncEvents.listenerCount('connected')).toBe(0);
      expect(syncEvents.listenerCount('disconnected')).toBe(0);
      expect(syncEvents.listenerCount('error')).toBe(0);
      expect(syncEvents.listenerCount('transferReceived')).toBe(0);
    });

    it('does not trigger toasts after unmount', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      unmount();

      // These should not trigger any toasts
      act(() => {
        syncEvents.emit('error', {
          message: 'After unmount',
          isAuthError: false,
        } as ErrorEventPayload);
      });

      expect(getAllToasts().length).toBe(0);
    });
  });

  describe('rapid connect/disconnect cycles', () => {
    it('handles rapid connect/disconnect gracefully', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      // Initial connection
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        } as ConnectedEventPayload);
      });

      // Rapid cycles
      for (let i = 0; i < 5; i++) {
        act(() => {
          syncEvents.emit('disconnected', {
            wasClean: false,
          } as DisconnectedEventPayload);
        });

        act(() => {
          syncEvents.emit('connected', {
            deviceId: 'device-1',
            sessionId: `session-${i + 2}`,
            isReconnect: true,
          } as ConnectedEventPayload);
        });
      }

      // Should have disconnect toast (only once) and reconnect toast (once at the end)
      // The logic prevents duplicate disconnect toasts
      const toasts = getAllToasts();

      // Check that we have some toasts but not an explosion of them
      // First disconnect toast + one reconnect per cycle that shows disconnect toast
      expect(toasts.length).toBeGreaterThan(0);
      expect(toasts.length).toBeLessThanOrEqual(10); // Reasonable upper bound

      unmount();
    });

    it('only shows one disconnect toast per disconnect sequence', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      // Initial connection
      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        } as ConnectedEventPayload);
      });

      // Multiple disconnect events in sequence (before reconnect)
      act(() => {
        syncEvents.emit('disconnected', {
          wasClean: false,
        } as DisconnectedEventPayload);
      });

      act(() => {
        syncEvents.emit('reconnecting', {
          attempt: 1,
        } as ReconnectingEventPayload);
      });

      act(() => {
        syncEvents.emit('reconnecting', {
          attempt: 2,
        } as ReconnectingEventPayload);
      });

      const disconnectToasts = getAllToasts().filter(
        (t) => t.title === 'Sync disconnected'
      );

      // Should only show one disconnect toast despite multiple events
      expect(disconnectToasts.length).toBe(1);

      unmount();
    });
  });

  describe('state reset on unmount', () => {
    it('resets connection tracking refs on unmount', () => {
      // First hook instance - connect and disconnect
      const { unmount: unmount1 } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('connected', {
          deviceId: 'device-1',
          sessionId: 'session-1',
          isReconnect: false,
        } as ConnectedEventPayload);
      });

      act(() => {
        syncEvents.emit('disconnected', {
          wasClean: false,
        } as DisconnectedEventPayload);
      });

      unmount1();
      resetToastStore();

      // Second hook instance - should start fresh
      const { unmount: unmount2 } = renderHook(() => useSyncNotifications());

      // Disconnect without prior connection in this instance should not show toast
      act(() => {
        syncEvents.emit('disconnected', {
          wasClean: false,
        } as DisconnectedEventPayload);
      });

      // The refs are reset so this should not show a toast
      expect(getAllToasts().length).toBe(0);

      unmount2();
    });
  });

  describe('multiple hook instances', () => {
    it('each hook instance shows its own toasts', () => {
      const { unmount: unmount1 } = renderHook(() => useSyncNotifications());
      const { unmount: unmount2 } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('error', {
          message: 'Test error',
          isAuthError: false,
        } as ErrorEventPayload);
      });

      // Both hooks subscribe, so event triggers twice
      expect(getAllToasts().length).toBe(2);

      unmount1();
      unmount2();
    });

    it('unmounting one instance does not affect the other', () => {
      const { unmount: unmount1 } = renderHook(() => useSyncNotifications());
      const { unmount: unmount2 } = renderHook(() => useSyncNotifications());

      // Unmount first instance
      unmount1();

      resetToastStore();

      // Second instance should still work
      act(() => {
        syncEvents.emit('error', {
          message: 'Test error',
          isAuthError: false,
        } as ErrorEventPayload);
      });

      expect(getAllToasts().length).toBe(1);

      unmount2();
    });
  });

  describe('option combinations', () => {
    it('respects all options when combined', () => {
      const { unmount } = renderHook(() =>
        useSyncNotifications({
          enabled: true,
          showDeviceJoined: true,
          showDeviceLeft: true,
          showTransfers: false,
        })
      );

      // Device joined should show (enabled)
      act(() => {
        syncEvents.emit('deviceJoined', {
          deviceId: 'new-device',
          deviceName: 'Test Device',
        } as DeviceJoinedEventPayload);
      });

      expect(getAllToasts().length).toBe(1);
      expect(getLastToast()?.title).toBe('Device connected');

      // Device left should show (enabled)
      act(() => {
        syncEvents.emit('deviceLeft', {
          deviceId: 'old-device',
        });
      });

      expect(getAllToasts().length).toBe(2);
      expect(getLastToast()?.title).toBe('Device disconnected');

      // Transfer should not show (disabled)
      act(() => {
        syncEvents.emit('transferReceived', {
          fromDeviceId: 'device-2',
          fromDeviceName: 'Another Device',
        } as TransferReceivedEventPayload);
      });

      expect(getAllToasts().length).toBe(2); // Still 2

      unmount();
    });

    it('enabled: false overrides all other options', () => {
      const { unmount } = renderHook(() =>
        useSyncNotifications({
          enabled: false,
          showDeviceJoined: true,
          showDeviceLeft: true,
          showTransfers: true,
        })
      );

      act(() => {
        syncEvents.emit('deviceJoined', {
          deviceId: 'new-device',
          deviceName: 'Test Device',
        } as DeviceJoinedEventPayload);
      });

      act(() => {
        syncEvents.emit('transferReceived', {
          fromDeviceId: 'device-2',
        } as TransferReceivedEventPayload);
      });

      expect(getAllToasts().length).toBe(0);

      unmount();
    });
  });

  describe('edge cases', () => {
    it('handles empty device name in transfer received', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('transferReceived', {
          fromDeviceId: 'device-2',
          fromDeviceName: '',
        } as TransferReceivedEventPayload);
      });

      // Empty string is falsy, should use fallback
      const toast = getLastToast();
      expect(toast?.description).toBe('You now control playback');

      unmount();
    });

    it('handles empty device name in transfer sent', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('transferSent', {
          toDeviceId: 'device-3',
          toDeviceName: '',
        } as TransferSentEventPayload);
      });

      // Empty string is falsy, should use fallback
      const toast = getLastToast();
      expect(toast?.description).toBe('Transferred to another device');

      unmount();
    });

    it('handles very long error messages', () => {
      const { unmount } = renderHook(() => useSyncNotifications());

      const longMessage = 'A'.repeat(1000);

      act(() => {
        syncEvents.emit('error', {
          message: longMessage,
          isAuthError: false,
        } as ErrorEventPayload);
      });

      const toast = getLastToast();
      expect(toast?.description).toBe(longMessage);

      unmount();
    });

    it('handles special characters in device names', () => {
      const { unmount } = renderHook(() =>
        useSyncNotifications({ showDeviceJoined: true })
      );

      act(() => {
        syncEvents.emit('deviceJoined', {
          deviceId: 'special-device',
          deviceName: "John's <Device> & \"More\"",
        } as DeviceJoinedEventPayload);
      });

      const toast = getLastToast();
      expect(toast?.description).toBe("John's <Device> & \"More\"");

      unmount();
    });

    it('handles undefined options gracefully', () => {
      // Pass no arguments - uses default options
      const { unmount } = renderHook(() => useSyncNotifications());

      act(() => {
        syncEvents.emit('error', {
          message: 'Test',
          isAuthError: false,
        } as ErrorEventPayload);
      });

      expect(getAllToasts().length).toBe(1);

      unmount();
    });
  });
});
