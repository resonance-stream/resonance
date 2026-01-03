/**
 * useTransferControl Hook Tests
 *
 * Unit tests for the transfer control hook covering:
 * - transferToDevice function behavior
 * - requestControl function behavior
 * - Edge cases (already-active device, disconnected state)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useTransferControl } from './useTransferControl';
import { useDeviceStore } from '../stores/deviceStore';

// Mock the sync/types module functions
vi.mock('../sync/types', async (importOriginal) => {
  const original = await importOriginal<typeof import('./types')>();
  return {
    ...original,
    getOrCreateDeviceId: vi.fn(() => 'test-device-id'),
    getDefaultDeviceName: vi.fn(() => 'Test Device'),
    detectDeviceType: vi.fn(() => 'web'),
  };
});

// Helper to reset store between tests
function resetDeviceStore(): void {
  useDeviceStore.setState({
    connectionState: 'connected',
    sessionId: 'test-session',
    lastError: null,
    reconnectAttempt: 0,
    deviceId: 'test-device-id',
    deviceName: 'Test Device',
    deviceType: 'web',
    devices: [],
    activeDeviceId: null,
  });
}

describe('useTransferControl', () => {
  const mockRequestTransfer = vi.fn();

  beforeEach(() => {
    resetDeviceStore();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('transferToDevice', () => {
    it('does nothing when not connected', () => {
      useDeviceStore.setState({ activeDeviceId: 'other-device' });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: false,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.transferToDevice('target-device');
      });

      expect(mockRequestTransfer).not.toHaveBeenCalled();
    });

    it('skips if target device is already active', () => {
      useDeviceStore.setState({ activeDeviceId: 'already-active-device' });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.transferToDevice('already-active-device');
      });

      expect(mockRequestTransfer).not.toHaveBeenCalled();
    });

    it('calls requestTransfer with correct device ID', () => {
      useDeviceStore.setState({ activeDeviceId: 'current-active' });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.transferToDevice('new-target-device');
      });

      expect(mockRequestTransfer).toHaveBeenCalledTimes(1);
      expect(mockRequestTransfer).toHaveBeenCalledWith('new-target-device');
    });

    it('transfers to a different device when no device is currently active', () => {
      useDeviceStore.setState({ activeDeviceId: null });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.transferToDevice('target-device');
      });

      expect(mockRequestTransfer).toHaveBeenCalledWith('target-device');
    });

    it('handles multiple transfer calls correctly', () => {
      useDeviceStore.setState({ activeDeviceId: 'device-a' });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.transferToDevice('device-b');
        result.current.transferToDevice('device-c');
      });

      expect(mockRequestTransfer).toHaveBeenCalledTimes(2);
      expect(mockRequestTransfer).toHaveBeenNthCalledWith(1, 'device-b');
      expect(mockRequestTransfer).toHaveBeenNthCalledWith(2, 'device-c');
    });

    it('does not transfer when connection changes to disconnected', () => {
      useDeviceStore.setState({ activeDeviceId: 'some-device' });

      const { result, rerender } = renderHook(
        ({ isConnected }) =>
          useTransferControl({
            isConnected,
            requestTransfer: mockRequestTransfer,
          }),
        { initialProps: { isConnected: true } }
      );

      // First call should work
      act(() => {
        result.current.transferToDevice('device-x');
      });
      expect(mockRequestTransfer).toHaveBeenCalledTimes(1);

      // Simulate disconnection
      rerender({ isConnected: false });

      // Second call should not work
      act(() => {
        result.current.transferToDevice('device-y');
      });
      expect(mockRequestTransfer).toHaveBeenCalledTimes(1);
    });
  });

  describe('requestControl', () => {
    it('does nothing when not connected', () => {
      useDeviceStore.setState({
        deviceId: 'test-device-id',
        activeDeviceId: 'other-device',
      });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: false,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.requestControl();
      });

      expect(mockRequestTransfer).not.toHaveBeenCalled();
    });

    it('skips when already the active device', () => {
      useDeviceStore.setState({
        deviceId: 'test-device-id',
        activeDeviceId: 'test-device-id',
      });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.requestControl();
      });

      expect(mockRequestTransfer).not.toHaveBeenCalled();
    });

    it('transfers to self (own device ID)', () => {
      useDeviceStore.setState({
        deviceId: 'my-device-id',
        activeDeviceId: 'other-device',
      });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.requestControl();
      });

      expect(mockRequestTransfer).toHaveBeenCalledWith('my-device-id');
    });

    it('requests control when no device is currently active', () => {
      useDeviceStore.setState({
        deviceId: 'test-device-id',
        activeDeviceId: null,
      });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.requestControl();
      });

      expect(mockRequestTransfer).toHaveBeenCalledWith('test-device-id');
    });

    it('uses the correct device ID from the store', () => {
      useDeviceStore.setState({
        deviceId: 'unique-device-abc-123',
        activeDeviceId: 'another-device',
      });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.requestControl();
      });

      expect(mockRequestTransfer).toHaveBeenCalledWith('unique-device-abc-123');
    });

    it('respects connection state changes', () => {
      useDeviceStore.setState({
        deviceId: 'test-device-id',
        activeDeviceId: 'other-device',
      });

      const { result, rerender } = renderHook(
        ({ isConnected }) =>
          useTransferControl({
            isConnected,
            requestTransfer: mockRequestTransfer,
          }),
        { initialProps: { isConnected: true } }
      );

      // Request control while connected
      act(() => {
        result.current.requestControl();
      });
      expect(mockRequestTransfer).toHaveBeenCalledTimes(1);

      // Disconnect and try again
      rerender({ isConnected: false });

      act(() => {
        result.current.requestControl();
      });
      expect(mockRequestTransfer).toHaveBeenCalledTimes(1);

      // Reconnect and request again
      rerender({ isConnected: true });

      act(() => {
        result.current.requestControl();
      });
      expect(mockRequestTransfer).toHaveBeenCalledTimes(2);
    });
  });

  describe('edge cases', () => {
    it('handles rapid switching between transferToDevice and requestControl', () => {
      useDeviceStore.setState({
        deviceId: 'my-device',
        activeDeviceId: 'device-a',
      });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.transferToDevice('device-b');
        result.current.requestControl();
        result.current.transferToDevice('device-c');
      });

      expect(mockRequestTransfer).toHaveBeenCalledTimes(3);
      expect(mockRequestTransfer).toHaveBeenNthCalledWith(1, 'device-b');
      expect(mockRequestTransfer).toHaveBeenNthCalledWith(2, 'my-device');
      expect(mockRequestTransfer).toHaveBeenNthCalledWith(3, 'device-c');
    });

    it('handles empty string device IDs gracefully', () => {
      useDeviceStore.setState({
        deviceId: '',
        activeDeviceId: 'other-device',
      });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      // requestControl with empty deviceId - should still call requestTransfer
      act(() => {
        result.current.requestControl();
      });

      expect(mockRequestTransfer).toHaveBeenCalledWith('');
    });

    it('handles transferToDevice with empty string target', () => {
      useDeviceStore.setState({ activeDeviceId: 'some-device' });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        result.current.transferToDevice('');
      });

      // Empty string is different from activeDeviceId, so transfer should proceed
      expect(mockRequestTransfer).toHaveBeenCalledWith('');
    });

    it('updates correctly when activeDeviceId changes', () => {
      useDeviceStore.setState({ activeDeviceId: 'device-1' });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      // Transfer to device-1 should skip (it's active)
      act(() => {
        result.current.transferToDevice('device-1');
      });
      expect(mockRequestTransfer).not.toHaveBeenCalled();

      // Update active device
      act(() => {
        useDeviceStore.setState({ activeDeviceId: 'device-2' });
      });

      // Now transfer to device-1 should work
      act(() => {
        result.current.transferToDevice('device-1');
      });
      expect(mockRequestTransfer).toHaveBeenCalledWith('device-1');
    });

    it('updates requestControl behavior when isActiveDevice changes', () => {
      useDeviceStore.setState({
        deviceId: 'my-device',
        activeDeviceId: 'my-device', // Start as active
      });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      // Should skip since already active
      act(() => {
        result.current.requestControl();
      });
      expect(mockRequestTransfer).not.toHaveBeenCalled();

      // Change to non-active
      act(() => {
        useDeviceStore.setState({ activeDeviceId: 'other-device' });
      });

      // Now requestControl should work
      act(() => {
        result.current.requestControl();
      });
      expect(mockRequestTransfer).toHaveBeenCalledWith('my-device');
    });

    it('handles both functions being called when both conditions would skip', () => {
      useDeviceStore.setState({
        deviceId: 'my-device',
        activeDeviceId: 'my-device',
      });

      const { result } = renderHook(() =>
        useTransferControl({
          isConnected: false, // Disconnected
          requestTransfer: mockRequestTransfer,
        })
      );

      act(() => {
        // transferToDevice would skip due to disconnection
        result.current.transferToDevice('other-device');
        // requestControl would skip due to disconnection AND being active
        result.current.requestControl();
      });

      expect(mockRequestTransfer).not.toHaveBeenCalled();
    });
  });

  describe('return value stability', () => {
    it('returns stable function references when dependencies do not change', () => {
      const { result, rerender } = renderHook(() =>
        useTransferControl({
          isConnected: true,
          requestTransfer: mockRequestTransfer,
        })
      );

      const initialTransferToDevice = result.current.transferToDevice;
      const initialRequestControl = result.current.requestControl;

      // Rerender with same props
      rerender();

      // Functions should be the same reference (memoized)
      expect(result.current.transferToDevice).toBe(initialTransferToDevice);
      expect(result.current.requestControl).toBe(initialRequestControl);
    });

    it('returns new function references when isConnected changes', () => {
      const { result, rerender } = renderHook(
        ({ isConnected }) =>
          useTransferControl({
            isConnected,
            requestTransfer: mockRequestTransfer,
          }),
        { initialProps: { isConnected: true } }
      );

      const initialTransferToDevice = result.current.transferToDevice;
      const initialRequestControl = result.current.requestControl;

      // Rerender with changed isConnected
      rerender({ isConnected: false });

      // Functions should be different references
      expect(result.current.transferToDevice).not.toBe(initialTransferToDevice);
      expect(result.current.requestControl).not.toBe(initialRequestControl);
    });

    it('returns new function references when requestTransfer changes', () => {
      const newMockRequestTransfer = vi.fn();

      const { result, rerender } = renderHook(
        ({ requestTransfer }) =>
          useTransferControl({
            isConnected: true,
            requestTransfer,
          }),
        { initialProps: { requestTransfer: mockRequestTransfer } }
      );

      const initialTransferToDevice = result.current.transferToDevice;

      // Rerender with new requestTransfer function
      rerender({ requestTransfer: newMockRequestTransfer });

      // Function should be different reference
      expect(result.current.transferToDevice).not.toBe(initialTransferToDevice);
    });
  });
});
