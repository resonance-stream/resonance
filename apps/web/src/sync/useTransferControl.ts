/**
 * Transfer Control Hook
 *
 * Handles device transfer logic for moving playback control
 * between devices.
 *
 * Extracted from useSyncState.ts for better modularity.
 */

import { useCallback } from 'react';
import { useDeviceStore, useIsActiveDevice } from '../stores/deviceStore';

/**
 * Configuration options for the useTransferControl hook.
 *
 * Provides the connection state and transfer request function needed
 * for device control transfers.
 */
export interface UseTransferControlOptions {
  /**
   * Whether the WebSocket sync connection is currently active.
   * When false, all transfer operations are no-ops.
   */
  isConnected: boolean;

  /**
   * Function to request a control transfer to a target device.
   * Provided by the parent useSyncConnection hook.
   *
   * @param targetDeviceId - The ID of the device to transfer control to
   */
  requestTransfer: (targetDeviceId: string) => void;
}

/**
 * Return value interface for the useTransferControl hook.
 *
 * Provides functions for transferring playback control between devices.
 */
export interface UseTransferControlValue {
  /**
   * Transfer playback control to another device.
   * The target device will become the active device and start broadcasting state.
   * No-op if not connected or if the target is already the active device.
   *
   * @param deviceId - The ID of the device to transfer control to
   */
  transferToDevice: (deviceId: string) => void;

  /**
   * Request to become the active device (take control from the current active device).
   * Equivalent to calling `transferToDevice` with the local device ID.
   * No-op if not connected or if already the active device.
   */
  requestControl: () => void;
}

/**
 * Hook for managing playback control transfers between devices.
 *
 * Provides functions to transfer playback control to another device or
 * to request control for the current device. This hook is typically composed
 * by {@link useSyncState} rather than used directly.
 *
 * ## Device control model:
 * - Only one device can be "active" (controlling) at a time
 * - The active device broadcasts playback and queue state
 * - Other devices receive and apply state from the active device
 * - Control can be transferred explicitly via these functions
 *
 * @param options - Configuration options including connection state and transfer function
 * @returns Object with transfer control functions
 *
 * @see useSyncState - The facade hook that composes this with other sync hooks
 */
export function useTransferControl(options: UseTransferControlOptions): UseTransferControlValue {
  const { isConnected, requestTransfer } = options;

  // Device state
  const deviceId = useDeviceStore((s) => s.deviceId);
  const activeDeviceId = useDeviceStore((s) => s.activeDeviceId);
  const isActiveDevice = useIsActiveDevice();

  // Transfer to another device
  const transferToDevice = useCallback(
    (targetDeviceId: string) => {
      if (!isConnected) return;
      // Skip if target is already the active device
      if (targetDeviceId === activeDeviceId) return;
      requestTransfer(targetDeviceId);
    },
    [isConnected, activeDeviceId, requestTransfer]
  );

  // Request to become active device
  const requestControl = useCallback(() => {
    if (!isConnected) return;
    // Skip if already the active device
    if (isActiveDevice) return;
    // Transfer to self
    requestTransfer(deviceId);
  }, [isConnected, isActiveDevice, requestTransfer, deviceId]);

  return {
    transferToDevice,
    requestControl,
  };
}
