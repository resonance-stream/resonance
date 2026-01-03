/**
 * Device Store Tests
 *
 * Comprehensive tests for the Zustand device store covering:
 * - Initial state
 * - Connection state management
 * - Device operations (add, remove, update)
 * - Active device tracking
 * - Reset behavior (preserving device identity)
 * - Selectors
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import {
  useDeviceStore,
  useIsActiveDevice,
  useIsConnected,
  useOtherDevices,
  useCurrentDevicePresence,
  useActiveDevicePresence,
} from './deviceStore'
import type { DevicePresence, ConnectionState, DeviceType } from '../sync/types'

// Mock the sync/types module
vi.mock('../sync/types', async (importOriginal) => {
  const original = await importOriginal<typeof import('../sync/types')>()
  return {
    ...original,
    getOrCreateDeviceId: vi.fn(() => 'mock-device-id'),
    getDefaultDeviceName: vi.fn(() => 'Mock Device'),
    detectDeviceType: vi.fn(() => 'web' as DeviceType),
  }
})

// Helper to reset the store between tests
function resetStore(): void {
  useDeviceStore.setState({
    connectionState: 'disconnected',
    sessionId: null,
    lastError: null,
    reconnectAttempt: 0,
    deviceId: 'mock-device-id',
    deviceName: 'Mock Device',
    deviceType: 'web',
    devices: [],
    activeDeviceId: null,
  })
}

// Factory for creating test device presence objects
function createDevicePresence(overrides: Partial<DevicePresence> = {}): DevicePresence {
  return {
    device_id: 'device-1',
    device_name: 'Test Device 1',
    device_type: 'web',
    is_active: false,
    current_track: null,
    volume: 1.0,
    last_seen: Date.now(),
    ...overrides,
  }
}

describe('deviceStore', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('initial state', () => {
    it('has disconnected connection state', () => {
      expect(useDeviceStore.getState().connectionState).toBe('disconnected')
    })

    it('has null session ID', () => {
      expect(useDeviceStore.getState().sessionId).toBeNull()
    })

    it('has null last error', () => {
      expect(useDeviceStore.getState().lastError).toBeNull()
    })

    it('has zero reconnect attempts', () => {
      expect(useDeviceStore.getState().reconnectAttempt).toBe(0)
    })

    it('has device ID from getOrCreateDeviceId', () => {
      expect(useDeviceStore.getState().deviceId).toBe('mock-device-id')
    })

    it('has device name from getDefaultDeviceName', () => {
      expect(useDeviceStore.getState().deviceName).toBe('Mock Device')
    })

    it('has device type from detectDeviceType', () => {
      expect(useDeviceStore.getState().deviceType).toBe('web')
    })

    it('has empty devices array', () => {
      expect(useDeviceStore.getState().devices).toEqual([])
    })

    it('has null active device ID', () => {
      expect(useDeviceStore.getState().activeDeviceId).toBeNull()
    })
  })

  describe('setConnectionState', () => {
    it.each<ConnectionState>(['connecting', 'connected', 'disconnected', 'reconnecting'])(
      'sets connection state to %s',
      (state) => {
        useDeviceStore.getState().setConnectionState(state)
        expect(useDeviceStore.getState().connectionState).toBe(state)
      }
    )

    it('can transition through connection states', () => {
      const { setConnectionState } = useDeviceStore.getState()

      setConnectionState('connecting')
      expect(useDeviceStore.getState().connectionState).toBe('connecting')

      setConnectionState('connected')
      expect(useDeviceStore.getState().connectionState).toBe('connected')

      setConnectionState('reconnecting')
      expect(useDeviceStore.getState().connectionState).toBe('reconnecting')

      setConnectionState('disconnected')
      expect(useDeviceStore.getState().connectionState).toBe('disconnected')
    })
  })

  describe('setSessionId', () => {
    it('sets the session ID', () => {
      useDeviceStore.getState().setSessionId('session-123')
      expect(useDeviceStore.getState().sessionId).toBe('session-123')
    })

    it('can set session ID to null', () => {
      useDeviceStore.setState({ sessionId: 'session-123' })
      useDeviceStore.getState().setSessionId(null)
      expect(useDeviceStore.getState().sessionId).toBeNull()
    })

    it('can overwrite existing session ID', () => {
      useDeviceStore.setState({ sessionId: 'old-session' })
      useDeviceStore.getState().setSessionId('new-session')
      expect(useDeviceStore.getState().sessionId).toBe('new-session')
    })
  })

  describe('setError', () => {
    it('sets the last error', () => {
      useDeviceStore.getState().setError('Connection failed')
      expect(useDeviceStore.getState().lastError).toBe('Connection failed')
    })

    it('can clear the error by setting null', () => {
      useDeviceStore.setState({ lastError: 'Previous error' })
      useDeviceStore.getState().setError(null)
      expect(useDeviceStore.getState().lastError).toBeNull()
    })

    it('can overwrite existing error', () => {
      useDeviceStore.setState({ lastError: 'Old error' })
      useDeviceStore.getState().setError('New error')
      expect(useDeviceStore.getState().lastError).toBe('New error')
    })
  })

  describe('setReconnectAttempt', () => {
    it('sets the reconnect attempt count', () => {
      useDeviceStore.getState().setReconnectAttempt(3)
      expect(useDeviceStore.getState().reconnectAttempt).toBe(3)
    })

    it('can reset to zero', () => {
      useDeviceStore.setState({ reconnectAttempt: 5 })
      useDeviceStore.getState().setReconnectAttempt(0)
      expect(useDeviceStore.getState().reconnectAttempt).toBe(0)
    })
  })

  describe('setDeviceName', () => {
    it('sets the device name', () => {
      useDeviceStore.getState().setDeviceName('My Custom Device')
      expect(useDeviceStore.getState().deviceName).toBe('My Custom Device')
    })

    it('can overwrite existing device name', () => {
      useDeviceStore.setState({ deviceName: 'Old Name' })
      useDeviceStore.getState().setDeviceName('New Name')
      expect(useDeviceStore.getState().deviceName).toBe('New Name')
    })
  })

  describe('setDevices', () => {
    it('sets the devices array', () => {
      const devices = [
        createDevicePresence({ device_id: 'device-1' }),
        createDevicePresence({ device_id: 'device-2' }),
      ]

      useDeviceStore.getState().setDevices(devices)

      expect(useDeviceStore.getState().devices).toEqual(devices)
    })

    it('extracts activeDeviceId from active device in list', () => {
      const devices = [
        createDevicePresence({ device_id: 'device-1', is_active: false }),
        createDevicePresence({ device_id: 'device-2', is_active: true }),
        createDevicePresence({ device_id: 'device-3', is_active: false }),
      ]

      useDeviceStore.getState().setDevices(devices)

      expect(useDeviceStore.getState().activeDeviceId).toBe('device-2')
    })

    it('sets activeDeviceId to null when no device is active', () => {
      const devices = [
        createDevicePresence({ device_id: 'device-1', is_active: false }),
        createDevicePresence({ device_id: 'device-2', is_active: false }),
      ]

      useDeviceStore.getState().setDevices(devices)

      expect(useDeviceStore.getState().activeDeviceId).toBeNull()
    })

    it('clears activeDeviceId when setting empty device list', () => {
      useDeviceStore.setState({ activeDeviceId: 'device-1' })

      useDeviceStore.getState().setDevices([])

      expect(useDeviceStore.getState().activeDeviceId).toBeNull()
      expect(useDeviceStore.getState().devices).toEqual([])
    })

    it('uses first active device when multiple are marked active', () => {
      const devices = [
        createDevicePresence({ device_id: 'device-1', is_active: true }),
        createDevicePresence({ device_id: 'device-2', is_active: true }),
      ]

      useDeviceStore.getState().setDevices(devices)

      // find() returns the first match
      expect(useDeviceStore.getState().activeDeviceId).toBe('device-1')
    })
  })

  describe('addDevice', () => {
    it('adds a new device to empty list', () => {
      const device = createDevicePresence({ device_id: 'new-device' })

      useDeviceStore.getState().addDevice(device)

      expect(useDeviceStore.getState().devices).toHaveLength(1)
      expect(useDeviceStore.getState().devices[0]).toEqual(device)
    })

    it('adds a new device to existing list', () => {
      const existing = createDevicePresence({ device_id: 'existing' })
      useDeviceStore.setState({ devices: [existing] })

      const newDevice = createDevicePresence({ device_id: 'new-device' })
      useDeviceStore.getState().addDevice(newDevice)

      expect(useDeviceStore.getState().devices).toHaveLength(2)
      expect(useDeviceStore.getState().devices).toContainEqual(existing)
      expect(useDeviceStore.getState().devices).toContainEqual(newDevice)
    })

    it('updates existing device instead of duplicating', () => {
      const existing = createDevicePresence({
        device_id: 'device-1',
        device_name: 'Old Name',
        volume: 0.5,
      })
      useDeviceStore.setState({ devices: [existing] })

      const updated = createDevicePresence({
        device_id: 'device-1',
        device_name: 'New Name',
        volume: 0.8,
      })
      useDeviceStore.getState().addDevice(updated)

      expect(useDeviceStore.getState().devices).toHaveLength(1)
      expect(useDeviceStore.getState().devices[0]!.device_name).toBe('New Name')
      expect(useDeviceStore.getState().devices[0]!.volume).toBe(0.8)
    })

    it('sets activeDeviceId when adding an active device', () => {
      const device = createDevicePresence({ device_id: 'active-device', is_active: true })

      useDeviceStore.getState().addDevice(device)

      expect(useDeviceStore.getState().activeDeviceId).toBe('active-device')
    })

    it('updates activeDeviceId when updating device to active', () => {
      const existingActive = createDevicePresence({ device_id: 'device-1', is_active: true })
      const existingInactive = createDevicePresence({ device_id: 'device-2', is_active: false })
      useDeviceStore.setState({
        devices: [existingActive, existingInactive],
        activeDeviceId: 'device-1',
      })

      const updatedDevice = createDevicePresence({ device_id: 'device-2', is_active: true })
      useDeviceStore.getState().addDevice(updatedDevice)

      expect(useDeviceStore.getState().activeDeviceId).toBe('device-2')
    })

    it('finds new active device when current active is updated to inactive', () => {
      const device1 = createDevicePresence({ device_id: 'device-1', is_active: true })
      const device2 = createDevicePresence({ device_id: 'device-2', is_active: false })
      useDeviceStore.setState({
        devices: [device1, device2],
        activeDeviceId: 'device-1',
      })

      // Update device-1 to inactive, device-2 is now active
      const updatedDevice1 = createDevicePresence({ device_id: 'device-1', is_active: false })
      const updatedDevice2 = createDevicePresence({ device_id: 'device-2', is_active: true })
      // First update device-2 to be active
      useDeviceStore.getState().addDevice(updatedDevice2)
      // Then update device-1 to be inactive
      useDeviceStore.getState().addDevice(updatedDevice1)

      expect(useDeviceStore.getState().activeDeviceId).toBe('device-2')
    })

    it('sets activeDeviceId to null when active device becomes inactive and no other active exists', () => {
      const device = createDevicePresence({ device_id: 'device-1', is_active: true })
      useDeviceStore.setState({
        devices: [device],
        activeDeviceId: 'device-1',
      })

      const inactiveDevice = createDevicePresence({ device_id: 'device-1', is_active: false })
      useDeviceStore.getState().addDevice(inactiveDevice)

      expect(useDeviceStore.getState().activeDeviceId).toBeNull()
    })
  })

  describe('removeDevice', () => {
    it('removes a device from the list', () => {
      const device1 = createDevicePresence({ device_id: 'device-1' })
      const device2 = createDevicePresence({ device_id: 'device-2' })
      useDeviceStore.setState({ devices: [device1, device2] })

      useDeviceStore.getState().removeDevice('device-1')

      expect(useDeviceStore.getState().devices).toHaveLength(1)
      expect(useDeviceStore.getState().devices[0]!.device_id).toBe('device-2')
    })

    it('does nothing when removing non-existent device', () => {
      const device = createDevicePresence({ device_id: 'device-1' })
      useDeviceStore.setState({ devices: [device] })

      useDeviceStore.getState().removeDevice('non-existent')

      expect(useDeviceStore.getState().devices).toHaveLength(1)
    })

    it('clears activeDeviceId when removing the active device', () => {
      const device = createDevicePresence({ device_id: 'device-1', is_active: true })
      useDeviceStore.setState({
        devices: [device],
        activeDeviceId: 'device-1',
      })

      useDeviceStore.getState().removeDevice('device-1')

      expect(useDeviceStore.getState().activeDeviceId).toBeNull()
    })

    it('preserves activeDeviceId when removing non-active device', () => {
      const activeDevice = createDevicePresence({ device_id: 'device-1', is_active: true })
      const inactiveDevice = createDevicePresence({ device_id: 'device-2', is_active: false })
      useDeviceStore.setState({
        devices: [activeDevice, inactiveDevice],
        activeDeviceId: 'device-1',
      })

      useDeviceStore.getState().removeDevice('device-2')

      expect(useDeviceStore.getState().activeDeviceId).toBe('device-1')
      expect(useDeviceStore.getState().devices).toHaveLength(1)
    })

    it('can remove all devices', () => {
      const device = createDevicePresence({ device_id: 'device-1' })
      useDeviceStore.setState({ devices: [device] })

      useDeviceStore.getState().removeDevice('device-1')

      expect(useDeviceStore.getState().devices).toEqual([])
    })
  })

  describe('updateDevice', () => {
    it('updates a device with partial data', () => {
      const device = createDevicePresence({
        device_id: 'device-1',
        device_name: 'Old Name',
        volume: 0.5,
      })
      useDeviceStore.setState({ devices: [device] })

      useDeviceStore.getState().updateDevice('device-1', { volume: 0.8 })

      const updatedDevice = useDeviceStore.getState().devices[0]
      expect(updatedDevice!.volume).toBe(0.8)
      expect(updatedDevice!.device_name).toBe('Old Name') // Preserved
    })

    it('does nothing when updating non-existent device', () => {
      const device = createDevicePresence({ device_id: 'device-1' })
      useDeviceStore.setState({ devices: [device] })

      useDeviceStore.getState().updateDevice('non-existent', { volume: 0.5 })

      expect(useDeviceStore.getState().devices).toHaveLength(1)
      expect(useDeviceStore.getState().devices[0]!.device_id).toBe('device-1')
    })

    it('sets activeDeviceId when updating device to active', () => {
      const device = createDevicePresence({ device_id: 'device-1', is_active: false })
      useDeviceStore.setState({ devices: [device], activeDeviceId: null })

      useDeviceStore.getState().updateDevice('device-1', { is_active: true })

      expect(useDeviceStore.getState().activeDeviceId).toBe('device-1')
    })

    it('clears activeDeviceId when updating active device to inactive', () => {
      const device = createDevicePresence({ device_id: 'device-1', is_active: true })
      useDeviceStore.setState({ devices: [device], activeDeviceId: 'device-1' })

      useDeviceStore.getState().updateDevice('device-1', { is_active: false })

      expect(useDeviceStore.getState().activeDeviceId).toBeNull()
    })

    it('finds new active device when updating active device to inactive', () => {
      const device1 = createDevicePresence({ device_id: 'device-1', is_active: true })
      const device2 = createDevicePresence({ device_id: 'device-2', is_active: false })
      useDeviceStore.setState({
        devices: [device1, device2],
        activeDeviceId: 'device-1',
      })

      // First make device-2 active
      useDeviceStore.getState().updateDevice('device-2', { is_active: true })
      // Then make device-1 inactive
      useDeviceStore.getState().updateDevice('device-1', { is_active: false })

      expect(useDeviceStore.getState().activeDeviceId).toBe('device-2')
    })

    it('can update multiple properties at once', () => {
      const device = createDevicePresence({
        device_id: 'device-1',
        device_name: 'Old Name',
        volume: 0.5,
        current_track: null,
      })
      useDeviceStore.setState({ devices: [device] })

      useDeviceStore.getState().updateDevice('device-1', {
        device_name: 'New Name',
        volume: 1.0,
        current_track: { id: 'track-1', title: 'Test Track', artist: 'Test Artist' },
      })

      const updated = useDeviceStore.getState().devices[0]
      expect(updated!.device_name).toBe('New Name')
      expect(updated!.volume).toBe(1.0)
      expect(updated!.current_track).toEqual({
        id: 'track-1',
        title: 'Test Track',
        artist: 'Test Artist',
      })
    })

    it('does not affect activeDeviceId when updating other properties', () => {
      const device = createDevicePresence({ device_id: 'device-1', is_active: true })
      useDeviceStore.setState({ devices: [device], activeDeviceId: 'device-1' })

      useDeviceStore.getState().updateDevice('device-1', { volume: 0.5 })

      expect(useDeviceStore.getState().activeDeviceId).toBe('device-1')
    })
  })

  describe('setActiveDeviceId', () => {
    it('sets the active device ID', () => {
      useDeviceStore.getState().setActiveDeviceId('device-1')
      expect(useDeviceStore.getState().activeDeviceId).toBe('device-1')
    })

    it('can set active device ID to null', () => {
      useDeviceStore.setState({ activeDeviceId: 'device-1' })
      useDeviceStore.getState().setActiveDeviceId(null)
      expect(useDeviceStore.getState().activeDeviceId).toBeNull()
    })
  })

  describe('reset', () => {
    it('resets connection state to disconnected', () => {
      useDeviceStore.setState({ connectionState: 'connected' })
      useDeviceStore.getState().reset()
      expect(useDeviceStore.getState().connectionState).toBe('disconnected')
    })

    it('clears session ID', () => {
      useDeviceStore.setState({ sessionId: 'session-123' })
      useDeviceStore.getState().reset()
      expect(useDeviceStore.getState().sessionId).toBeNull()
    })

    it('clears last error', () => {
      useDeviceStore.setState({ lastError: 'Some error' })
      useDeviceStore.getState().reset()
      expect(useDeviceStore.getState().lastError).toBeNull()
    })

    it('resets reconnect attempt to zero', () => {
      useDeviceStore.setState({ reconnectAttempt: 5 })
      useDeviceStore.getState().reset()
      expect(useDeviceStore.getState().reconnectAttempt).toBe(0)
    })

    it('clears devices array', () => {
      useDeviceStore.setState({
        devices: [createDevicePresence({ device_id: 'device-1' })],
      })
      useDeviceStore.getState().reset()
      expect(useDeviceStore.getState().devices).toEqual([])
    })

    it('clears active device ID', () => {
      useDeviceStore.setState({ activeDeviceId: 'device-1' })
      useDeviceStore.getState().reset()
      expect(useDeviceStore.getState().activeDeviceId).toBeNull()
    })

    it('preserves device ID', () => {
      useDeviceStore.setState({ deviceId: 'my-unique-device-id' })
      useDeviceStore.getState().reset()
      expect(useDeviceStore.getState().deviceId).toBe('my-unique-device-id')
    })

    it('preserves device name', () => {
      useDeviceStore.setState({ deviceName: 'My Custom Device' })
      useDeviceStore.getState().reset()
      expect(useDeviceStore.getState().deviceName).toBe('My Custom Device')
    })

    it('preserves device type', () => {
      useDeviceStore.setState({ deviceType: 'mobile' })
      useDeviceStore.getState().reset()
      expect(useDeviceStore.getState().deviceType).toBe('mobile')
    })

    it('resets all transient state while preserving identity', () => {
      // Set up a fully populated state
      useDeviceStore.setState({
        connectionState: 'connected',
        sessionId: 'session-123',
        lastError: 'Some error',
        reconnectAttempt: 3,
        deviceId: 'my-device',
        deviceName: 'My Device',
        deviceType: 'tablet',
        devices: [createDevicePresence({ device_id: 'other-device' })],
        activeDeviceId: 'other-device',
      })

      useDeviceStore.getState().reset()

      const state = useDeviceStore.getState()
      // Transient state is reset
      expect(state.connectionState).toBe('disconnected')
      expect(state.sessionId).toBeNull()
      expect(state.lastError).toBeNull()
      expect(state.reconnectAttempt).toBe(0)
      expect(state.devices).toEqual([])
      expect(state.activeDeviceId).toBeNull()
      // Identity is preserved
      expect(state.deviceId).toBe('my-device')
      expect(state.deviceName).toBe('My Device')
      expect(state.deviceType).toBe('tablet')
    })
  })
})

describe('deviceStore selectors', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  describe('useIsActiveDevice', () => {
    it('returns true when current device is the active device', () => {
      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        activeDeviceId: 'mock-device-id',
      })

      const { result } = renderHook(() => useIsActiveDevice())

      expect(result.current).toBe(true)
    })

    it('returns false when current device is not the active device', () => {
      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        activeDeviceId: 'other-device-id',
      })

      const { result } = renderHook(() => useIsActiveDevice())

      expect(result.current).toBe(false)
    })

    it('returns false when there is no active device', () => {
      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        activeDeviceId: null,
      })

      const { result } = renderHook(() => useIsActiveDevice())

      expect(result.current).toBe(false)
    })

    it('updates when active device changes', () => {
      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        activeDeviceId: null,
      })

      const { result } = renderHook(() => useIsActiveDevice())
      expect(result.current).toBe(false)

      act(() => {
        useDeviceStore.getState().setActiveDeviceId('mock-device-id')
      })

      expect(result.current).toBe(true)
    })
  })

  describe('useIsConnected', () => {
    it('returns true when connection state is connected', () => {
      useDeviceStore.setState({ connectionState: 'connected' })

      const { result } = renderHook(() => useIsConnected())

      expect(result.current).toBe(true)
    })

    it('returns false when connection state is disconnected', () => {
      useDeviceStore.setState({ connectionState: 'disconnected' })

      const { result } = renderHook(() => useIsConnected())

      expect(result.current).toBe(false)
    })

    it('returns false when connection state is connecting', () => {
      useDeviceStore.setState({ connectionState: 'connecting' })

      const { result } = renderHook(() => useIsConnected())

      expect(result.current).toBe(false)
    })

    it('returns false when connection state is reconnecting', () => {
      useDeviceStore.setState({ connectionState: 'reconnecting' })

      const { result } = renderHook(() => useIsConnected())

      expect(result.current).toBe(false)
    })

    it('updates when connection state changes', () => {
      useDeviceStore.setState({ connectionState: 'disconnected' })

      const { result } = renderHook(() => useIsConnected())
      expect(result.current).toBe(false)

      act(() => {
        useDeviceStore.getState().setConnectionState('connected')
      })

      expect(result.current).toBe(true)
    })
  })

  describe('useOtherDevices', () => {
    it('returns empty array when no devices', () => {
      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        devices: [],
      })

      const { result } = renderHook(() => useOtherDevices())

      expect(result.current).toEqual([])
    })

    it('excludes current device from results', () => {
      const currentDevice = createDevicePresence({ device_id: 'mock-device-id' })
      const otherDevice = createDevicePresence({ device_id: 'other-device' })

      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        devices: [currentDevice, otherDevice],
      })

      const { result } = renderHook(() => useOtherDevices())

      expect(result.current).toHaveLength(1)
      expect(result.current[0]!.device_id).toBe('other-device')
    })

    it('returns all devices except current', () => {
      const devices = [
        createDevicePresence({ device_id: 'mock-device-id' }),
        createDevicePresence({ device_id: 'device-2' }),
        createDevicePresence({ device_id: 'device-3' }),
      ]

      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        devices,
      })

      const { result } = renderHook(() => useOtherDevices())

      expect(result.current).toHaveLength(2)
      expect(result.current.map((d) => d.device_id)).toEqual(['device-2', 'device-3'])
    })

    it('returns all devices when current device is not in list', () => {
      const devices = [
        createDevicePresence({ device_id: 'device-1' }),
        createDevicePresence({ device_id: 'device-2' }),
      ]

      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        devices,
      })

      const { result } = renderHook(() => useOtherDevices())

      expect(result.current).toHaveLength(2)
    })

    it('updates when devices change', () => {
      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        devices: [],
      })

      const { result } = renderHook(() => useOtherDevices())
      expect(result.current).toHaveLength(0)

      act(() => {
        useDeviceStore.getState().addDevice(createDevicePresence({ device_id: 'new-device' }))
      })

      expect(result.current).toHaveLength(1)
    })
  })

  describe('useCurrentDevicePresence', () => {
    it('returns undefined when current device is not in device list', () => {
      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        devices: [],
      })

      const { result } = renderHook(() => useCurrentDevicePresence())

      expect(result.current).toBeUndefined()
    })

    it('returns current device presence when found', () => {
      const currentDevicePresence = createDevicePresence({
        device_id: 'mock-device-id',
        device_name: 'Current Device',
        is_active: true,
      })

      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        devices: [currentDevicePresence],
      })

      const { result } = renderHook(() => useCurrentDevicePresence())

      expect(result.current).toEqual(currentDevicePresence)
    })

    it('finds current device among multiple devices', () => {
      const otherDevice = createDevicePresence({ device_id: 'other-device' })
      const currentDevicePresence = createDevicePresence({
        device_id: 'mock-device-id',
        device_name: 'My Device',
      })

      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        devices: [otherDevice, currentDevicePresence],
      })

      const { result } = renderHook(() => useCurrentDevicePresence())

      expect(result.current!.device_id).toBe('mock-device-id')
      expect(result.current!.device_name).toBe('My Device')
    })

    it('updates when devices change', () => {
      useDeviceStore.setState({
        deviceId: 'mock-device-id',
        devices: [],
      })

      const { result } = renderHook(() => useCurrentDevicePresence())
      expect(result.current).toBeUndefined()

      act(() => {
        useDeviceStore.getState().addDevice(
          createDevicePresence({
            device_id: 'mock-device-id',
            device_name: 'My Device',
          })
        )
      })

      expect(result.current!.device_id).toBe('mock-device-id')
    })
  })

  describe('useActiveDevicePresence', () => {
    it('returns undefined when there is no active device', () => {
      useDeviceStore.setState({
        activeDeviceId: null,
        devices: [],
      })

      const { result } = renderHook(() => useActiveDevicePresence())

      expect(result.current).toBeUndefined()
    })

    it('returns undefined when active device is not in device list', () => {
      useDeviceStore.setState({
        activeDeviceId: 'missing-device',
        devices: [],
      })

      const { result } = renderHook(() => useActiveDevicePresence())

      expect(result.current).toBeUndefined()
    })

    it('returns active device presence when found', () => {
      const activeDevice = createDevicePresence({
        device_id: 'active-device',
        device_name: 'Active Device',
        is_active: true,
      })

      useDeviceStore.setState({
        activeDeviceId: 'active-device',
        devices: [activeDevice],
      })

      const { result } = renderHook(() => useActiveDevicePresence())

      expect(result.current).toEqual(activeDevice)
    })

    it('finds active device among multiple devices', () => {
      const inactiveDevice = createDevicePresence({
        device_id: 'inactive',
        is_active: false,
      })
      const activeDevice = createDevicePresence({
        device_id: 'active-device',
        device_name: 'Active Device',
        is_active: true,
      })

      useDeviceStore.setState({
        activeDeviceId: 'active-device',
        devices: [inactiveDevice, activeDevice],
      })

      const { result } = renderHook(() => useActiveDevicePresence())

      expect(result.current!.device_id).toBe('active-device')
      expect(result.current!.device_name).toBe('Active Device')
    })

    it('updates when active device changes', () => {
      const device1 = createDevicePresence({
        device_id: 'device-1',
        device_name: 'Device 1',
        is_active: true,
      })
      const device2 = createDevicePresence({
        device_id: 'device-2',
        device_name: 'Device 2',
        is_active: false,
      })

      useDeviceStore.setState({
        activeDeviceId: 'device-1',
        devices: [device1, device2],
      })

      const { result } = renderHook(() => useActiveDevicePresence())
      expect(result.current!.device_id).toBe('device-1')

      act(() => {
        useDeviceStore.getState().setActiveDeviceId('device-2')
      })

      expect(result.current!.device_id).toBe('device-2')
    })
  })
})
