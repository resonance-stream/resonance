/**
 * Device Store
 *
 * Manages device presence and sync state for cross-device playback synchronization.
 * This store tracks:
 * - Connection state to the sync WebSocket
 * - List of connected devices
 * - Active device (controlling playback)
 * - Current device info
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { useShallow } from 'zustand/react/shallow';
import type { DevicePresence, ConnectionState, DeviceType } from '../sync/types';
import { getOrCreateDeviceId, getDefaultDeviceName, detectDeviceType } from '../sync/types';

export interface DeviceState {
  // Connection
  connectionState: ConnectionState;
  sessionId: string | null;
  lastError: string | null;
  reconnectAttempt: number;

  // This device
  deviceId: string;
  deviceName: string;
  deviceType: DeviceType;

  // All devices
  devices: DevicePresence[];
  activeDeviceId: string | null;

  // Actions
  setConnectionState: (state: ConnectionState) => void;
  setSessionId: (sessionId: string | null) => void;
  setError: (error: string | null) => void;
  setReconnectAttempt: (attempt: number) => void;
  setDeviceName: (name: string) => void;
  setDevices: (devices: DevicePresence[]) => void;
  addDevice: (device: DevicePresence) => void;
  removeDevice: (deviceId: string) => void;
  updateDevice: (deviceId: string, updates: Partial<DevicePresence>) => void;
  setActiveDeviceId: (deviceId: string | null) => void;
  reset: () => void;
}

// Initial state
const createInitialState = () => ({
  connectionState: 'disconnected' as ConnectionState,
  sessionId: null,
  lastError: null,
  reconnectAttempt: 0,
  deviceId: getOrCreateDeviceId(),
  deviceName: getDefaultDeviceName(),
  deviceType: detectDeviceType(),
  devices: [] as DevicePresence[],
  activeDeviceId: null,
});

export const useDeviceStore = create<DeviceState>()(
  persist(
    (set, get) => ({
      ...createInitialState(),

      setConnectionState: (connectionState) => set({ connectionState }),

      setSessionId: (sessionId) => set({ sessionId }),

      setError: (lastError) => set({ lastError }),

      setReconnectAttempt: (reconnectAttempt) => set({ reconnectAttempt }),

      setDeviceName: (deviceName) => set({ deviceName }),

      setDevices: (devices) => {
        // Update active device from the list
        const activeDevice = devices.find((d) => d.is_active);
        set({
          devices,
          activeDeviceId: activeDevice?.device_id ?? null,
        });
      },

      addDevice: (device) => {
        const { devices, activeDeviceId } = get();
        const existingDevice = devices.some((d) => d.device_id === device.device_id);

        const newDevices = existingDevice
          ? devices.map((d) => (d.device_id === device.device_id ? device : d))
          : [...devices, device];

        let newActiveDeviceId = activeDeviceId;
        if (device.is_active) {
          // If the new/updated device is active, it becomes the active device
          newActiveDeviceId = device.device_id;
        } else if (activeDeviceId === device.device_id) {
          // If the updated device was the active one but is no longer,
          // scan for another active device in the list
          const otherActive = newDevices.find(
            (d) => d.device_id !== device.device_id && d.is_active
          );
          newActiveDeviceId = otherActive?.device_id ?? null;
        }

        set({
          devices: newDevices,
          activeDeviceId: newActiveDeviceId,
        });
      },

      removeDevice: (deviceId) => {
        const { devices, activeDeviceId } = get();
        set({
          devices: devices.filter((d) => d.device_id !== deviceId),
          // Clear active device if it was removed
          activeDeviceId: activeDeviceId === deviceId ? null : activeDeviceId,
        });
      },

      updateDevice: (deviceId, updates) => {
        const { devices } = get();
        set({
          devices: devices.map((d) => (d.device_id === deviceId ? { ...d, ...updates } : d)),
        });

        // Update active device if needed
        if (updates.is_active === true) {
          set({ activeDeviceId: deviceId });
        } else if (updates.is_active === false && get().activeDeviceId === deviceId) {
          // Find new active device
          const newActive = get().devices.find((d) => d.is_active);
          set({ activeDeviceId: newActive?.device_id ?? null });
        }
      },

      setActiveDeviceId: (activeDeviceId) => set({ activeDeviceId }),

      reset: () => {
        const { deviceId, deviceName, deviceType } = get();
        set({
          ...createInitialState(),
          // Preserve device identity
          deviceId,
          deviceName,
          deviceType,
        });
      },
    }),
    {
      name: 'resonance-device',
      partialize: (state) => ({
        // Only persist device identity
        deviceId: state.deviceId,
        deviceName: state.deviceName,
      }),
    }
  )
);

// =============================================================================
// Selectors
// =============================================================================

/** Get the current device's presence info from the device list */
export const useCurrentDevicePresence = (): DevicePresence | undefined => {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const devices = useDeviceStore((s) => s.devices);
  return devices.find((d) => d.device_id === deviceId);
};

/** Check if this device is the active device */
export const useIsActiveDevice = (): boolean => {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const activeDeviceId = useDeviceStore((s) => s.activeDeviceId);
  return deviceId === activeDeviceId;
};

/** Check if we're connected to the sync server */
export const useIsConnected = (): boolean => {
  return useDeviceStore((s) => s.connectionState === 'connected');
};

/** Get other connected devices (excluding current device) */
export const useOtherDevices = (): DevicePresence[] => {
  return useDeviceStore(
    useShallow((s) => s.devices.filter((d) => d.device_id !== s.deviceId))
  );
};

/** Get the active device's presence info */
export const useActiveDevicePresence = (): DevicePresence | undefined => {
  const activeDeviceId = useDeviceStore((s) => s.activeDeviceId);
  const devices = useDeviceStore((s) => s.devices);
  return devices.find((d) => d.device_id === activeDeviceId);
};
