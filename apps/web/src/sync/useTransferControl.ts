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

export interface UseTransferControlOptions {
  /** Whether sync connection is active */
  isConnected: boolean;
  /** Request transfer to a target device */
  requestTransfer: (targetDeviceId: string) => void;
}

export interface UseTransferControlValue {
  /** Transfer control to another device */
  transferToDevice: (deviceId: string) => void;
  /** Request to become the active device */
  requestControl: () => void;
}

/**
 * Hook for managing device transfer control
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
