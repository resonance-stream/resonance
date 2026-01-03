/**
 * Sync Notifications Hook
 *
 * Subscribes to sync events and displays appropriate toast notifications.
 * This hook should be used in a high-level component (like App.tsx or MainLayout)
 * to ensure notifications are shown regardless of which component emits the event.
 *
 * @example
 * ```tsx
 * // In App.tsx or MainLayout.tsx
 * function App() {
 *   useSyncNotifications();
 *
 *   return (
 *     <SyncProvider>
 *       {/* ... *\/}
 *     </SyncProvider>
 *   );
 * }
 * ```
 */

import { useCallback, useRef, useEffect } from 'react';
import { useToastStore } from '../stores/toastStore';
import {
  useSyncEvents,
  type ConnectedEventPayload,
  type DisconnectedEventPayload,
  type ErrorEventPayload,
  type DeviceJoinedEventPayload,
  type TransferReceivedEventPayload,
  type TransferSentEventPayload,
} from './syncEvents';

export interface UseSyncNotificationsOptions {
  /** Whether to show notifications (default: true) */
  enabled?: boolean;
  /** Whether to show device join notifications (default: false) */
  showDeviceJoined?: boolean;
  /** Whether to show device left notifications (default: false) */
  showDeviceLeft?: boolean;
  /** Whether to show transfer notifications (default: true) */
  showTransfers?: boolean;
}

/**
 * Hook that displays toast notifications for sync events
 *
 * @param options - Configuration options
 */
export function useSyncNotifications(options: UseSyncNotificationsOptions = {}): void {
  const {
    enabled = true,
    showDeviceJoined = false,
    showDeviceLeft = false,
    showTransfers = true,
  } = options;

  const addToast = useToastStore((s) => s.addToast);

  // Track if we've had a previous connection (to differentiate initial connect from reconnect)
  const hasConnectedBeforeRef = useRef(false);
  const hasShownDisconnectToastRef = useRef(false);

  // Handle 'connected' event
  const handleConnected = useCallback((payload: ConnectedEventPayload) => {
    if (!enabled) return;

    // Only show "restored" message if this is a reconnection after a disconnect
    if (payload.isReconnect && hasShownDisconnectToastRef.current) {
      addToast({
        type: 'success',
        title: 'Sync restored',
        description: 'Cross-device sync is active',
      });
    }

    hasConnectedBeforeRef.current = true;
    hasShownDisconnectToastRef.current = false;
  }, [enabled, addToast]);

  useSyncEvents('connected', handleConnected);

  // Handle 'disconnected' event
  const handleDisconnected = useCallback((payload: DisconnectedEventPayload) => {
    if (!enabled) return;

    // Only show disconnect toast if we had a previous successful connection
    if (hasConnectedBeforeRef.current && !hasShownDisconnectToastRef.current) {
      hasShownDisconnectToastRef.current = true;

      // Don't show toast for clean disconnects (e.g., intentional close)
      if (!payload.wasClean) {
        addToast({
          type: 'warning',
          title: 'Sync disconnected',
          description: 'Reconnecting...',
        });
      }
    }
  }, [enabled, addToast]);

  useSyncEvents('disconnected', handleDisconnected);

  // Handle 'reconnecting' event
  const handleReconnecting = useCallback(() => {
    if (!enabled) return;

    // Show disconnect toast on reconnecting if we haven't already
    if (hasConnectedBeforeRef.current && !hasShownDisconnectToastRef.current) {
      hasShownDisconnectToastRef.current = true;
      addToast({
        type: 'warning',
        title: 'Sync disconnected',
        description: 'Reconnecting...',
      });
    }
  }, [enabled, addToast]);

  useSyncEvents('reconnecting', handleReconnecting);

  // Handle 'error' event
  const handleError = useCallback((payload: ErrorEventPayload) => {
    if (!enabled) return;

    if (payload.isAuthError) {
      addToast({
        type: 'error',
        title: 'Sync authentication failed',
        description: 'Please sign in again to enable sync',
      });
    } else {
      addToast({
        type: 'error',
        title: 'Sync connection error',
        description: payload.message,
      });
    }
  }, [enabled, addToast]);

  useSyncEvents('error', handleError);

  // Handle 'deviceJoined' event
  const handleDeviceJoined = useCallback((payload: DeviceJoinedEventPayload) => {
    if (!enabled || !showDeviceJoined) return;

    addToast({
      type: 'info',
      title: 'Device connected',
      description: payload.deviceName,
    });
  }, [enabled, showDeviceJoined, addToast]);

  useSyncEvents('deviceJoined', handleDeviceJoined);

  // Handle 'deviceLeft' event
  const handleDeviceLeft = useCallback(() => {
    if (!enabled || !showDeviceLeft) return;

    addToast({
      type: 'info',
      title: 'Device disconnected',
    });
  }, [enabled, showDeviceLeft, addToast]);

  useSyncEvents('deviceLeft', handleDeviceLeft);

  // Handle 'transferReceived' event
  const handleTransferReceived = useCallback((payload: TransferReceivedEventPayload) => {
    if (!enabled || !showTransfers) return;

    addToast({
      type: 'info',
      title: 'Playback transferred',
      description: payload.fromDeviceName
        ? `Now playing from ${payload.fromDeviceName}`
        : 'You now control playback',
    });
  }, [enabled, showTransfers, addToast]);

  useSyncEvents('transferReceived', handleTransferReceived);

  // Handle 'transferSent' event
  const handleTransferSent = useCallback((payload: TransferSentEventPayload) => {
    if (!enabled || !showTransfers) return;

    addToast({
      type: 'info',
      title: 'Playback transferred',
      description: payload.toDeviceName
        ? `Now playing on ${payload.toDeviceName}`
        : 'Transferred to another device',
    });
  }, [enabled, showTransfers, addToast]);

  useSyncEvents('transferSent', handleTransferSent);

  // Reset state on unmount
  useEffect(() => {
    return () => {
      hasConnectedBeforeRef.current = false;
      hasShownDisconnectToastRef.current = false;
    };
  }, []);
}
