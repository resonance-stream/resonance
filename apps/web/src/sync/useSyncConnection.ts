/**
 * React hook for managing WebSocket sync connection
 *
 * Handles:
 * - Automatic connection on auth token availability
 * - Reconnection on auth token change
 * - Cleanup on unmount
 * - Integration with deviceStore
 */

import { useEffect, useRef, useCallback, useMemo } from 'react';
import { WebSocketClient, type WebSocketClientConfig } from './WebSocketClient';
import type { ServerMessage, PlaybackState, QueueState, SyncedSettings } from './types';
import { useDeviceStore } from '../stores/deviceStore';
import { useAuthStore } from '../stores/authStore';

export interface UseSyncConnectionOptions {
  /** Custom WebSocket client configuration */
  config?: WebSocketClientConfig;
  /** Called when playback state sync is received */
  onPlaybackSync?: (state: PlaybackState) => void;
  /** Called when seek sync is received */
  onSeekSync?: (positionMs: number, timestamp: number) => void;
  /** Called when queue sync is received */
  onQueueSync?: (state: QueueState) => void;
  /** Called when settings sync is received */
  onSettingsSync?: (settings: SyncedSettings) => void;
  /** Called when transfer is requested to this device */
  onTransferRequested?: (fromDeviceId: string) => void;
  /** Called on any server message */
  onMessage?: (message: ServerMessage) => void;
  /** Whether to auto-connect when token is available (default: true) */
  autoConnect?: boolean;
}

export interface SyncConnectionState {
  /** The WebSocket client instance */
  client: WebSocketClient | null;
  /** Whether we're connected */
  isConnected: boolean;
  /** Whether this device is the active device */
  isActiveDevice: boolean;
  /** Manually connect (if autoConnect is false) */
  connect: () => void;
  /** Disconnect from the sync server */
  disconnect: () => void;
  /** Send a playback state update */
  sendPlaybackUpdate: (state: PlaybackState) => void;
  /** Send a seek update */
  sendSeek: (positionMs: number) => void;
  /** Send a queue update */
  sendQueueUpdate: (state: QueueState) => void;
  /** Send a settings update */
  sendSettingsUpdate: (settings: SyncedSettings) => void;
  /** Request transfer to another device */
  requestTransfer: (targetDeviceId: string) => void;
  /** Request the device list */
  requestDeviceList: () => void;
}

/**
 * Hook to manage WebSocket sync connection
 *
 * Automatically connects when an auth token is available and
 * syncs connection state with the device store.
 */
export function useSyncConnection(options: UseSyncConnectionOptions = {}): SyncConnectionState {
  const {
    config,
    onPlaybackSync,
    onSeekSync,
    onQueueSync,
    onSettingsSync,
    onTransferRequested,
    onMessage,
    autoConnect = true,
  } = options;

  // Get auth token from auth store
  const accessToken = useAuthStore((s) => s.accessToken);

  // Get device store actions
  const setConnectionState = useDeviceStore((s) => s.setConnectionState);
  const setSessionId = useDeviceStore((s) => s.setSessionId);
  const setError = useDeviceStore((s) => s.setError);
  const setReconnectAttempt = useDeviceStore((s) => s.setReconnectAttempt);
  const setDevices = useDeviceStore((s) => s.setDevices);
  const addDevice = useDeviceStore((s) => s.addDevice);
  const removeDevice = useDeviceStore((s) => s.removeDevice);
  const setActiveDeviceId = useDeviceStore((s) => s.setActiveDeviceId);
  const deviceName = useDeviceStore((s) => s.deviceName);
  const deviceType = useDeviceStore((s) => s.deviceType);

  // Get current state
  const connectionState = useDeviceStore((s) => s.connectionState);
  const deviceId = useDeviceStore((s) => s.deviceId);
  const activeDeviceId = useDeviceStore((s) => s.activeDeviceId);

  // Stable ref for callbacks
  const callbacksRef = useRef({
    onPlaybackSync,
    onSeekSync,
    onQueueSync,
    onSettingsSync,
    onTransferRequested,
    onMessage,
  });

  useEffect(() => {
    callbacksRef.current = {
      onPlaybackSync,
      onSeekSync,
      onQueueSync,
      onSettingsSync,
      onTransferRequested,
      onMessage,
    };
  }, [onPlaybackSync, onSeekSync, onQueueSync, onSettingsSync, onTransferRequested, onMessage]);

  // WebSocket client ref
  const clientRef = useRef<WebSocketClient | null>(null);

  // Token ref for reconnection
  const tokenRef = useRef<string | null>(accessToken);

  useEffect(() => {
    tokenRef.current = accessToken;
  }, [accessToken]);

  // Create client with event handlers
  useEffect(() => {
    // Create client with device info from store
    const client = new WebSocketClient(
      {
        ...config,
        deviceName,
        deviceType,
      },
      {
        onStateChange: (state) => {
          setConnectionState(state);
          if (state === 'disconnected') {
            setSessionId(null);
            setReconnectAttempt(0);
          }
        },
        onConnected: (payload) => {
          setSessionId(payload.session_id);
          setActiveDeviceId(payload.active_device_id);
          setError(null);
          setReconnectAttempt(0);
          // Request device list on connect
          client.requestDeviceList();
        },
        onReconnecting: (attempt) => {
          setReconnectAttempt(attempt);
        },
        onError: (error) => {
          setError(error.message);
        },
        onMessage: (message) => {
          // Handle device-related messages
          switch (message.type) {
            case 'DeviceList':
              setDevices(message.payload);
              break;
            case 'DeviceConnected':
              addDevice(message.payload);
              break;
            case 'DeviceDisconnected':
              removeDevice(message.payload.device_id);
              break;
            case 'PlaybackSync':
              callbacksRef.current.onPlaybackSync?.(message.payload);
              break;
            case 'SeekSync':
              callbacksRef.current.onSeekSync?.(
                message.payload.position_ms,
                message.payload.timestamp
              );
              break;
            case 'QueueSync':
              callbacksRef.current.onQueueSync?.(message.payload);
              break;
            case 'SettingsSync':
              callbacksRef.current.onSettingsSync?.(message.payload);
              break;
            case 'TransferRequested':
              callbacksRef.current.onTransferRequested?.(message.payload.from_device_id);
              break;
            case 'TransferAccepted':
              setActiveDeviceId(message.payload.to_device_id);
              break;
          }

          callbacksRef.current.onMessage?.(message);
        },
      }
    );

    clientRef.current = client;

    // Auto-connect if enabled and we have a token
    if (autoConnect && accessToken) {
      client.connect(accessToken);
    }

    return () => {
      client.disconnect();
      clientRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [deviceName, deviceType]); // Only recreate client when device info changes

  // Handle token changes (reconnect with new token)
  useEffect(() => {
    const client = clientRef.current;
    if (!client) return;

    if (accessToken && autoConnect) {
      // Connect with new token (will disconnect existing connection first)
      if (connectionState === 'disconnected' || connectionState === 'reconnecting') {
        client.connect(accessToken);
      }
    } else if (!accessToken) {
      // Disconnect when token is removed (logout)
      client.disconnect();
    }
  }, [accessToken, autoConnect, connectionState]);

  // Manual connect function
  const connect = useCallback(() => {
    const client = clientRef.current;
    const token = tokenRef.current;
    if (client && token) {
      client.connect(token);
    }
  }, []);

  // Disconnect function
  const disconnect = useCallback(() => {
    clientRef.current?.disconnect();
  }, []);

  // Send functions
  const sendPlaybackUpdate = useCallback((state: PlaybackState) => {
    clientRef.current?.send({ type: 'PlaybackStateUpdate', payload: state });
  }, []);

  const sendSeek = useCallback((positionMs: number) => {
    clientRef.current?.send({ type: 'Seek', payload: { position_ms: positionMs } });
  }, []);

  const sendQueueUpdate = useCallback((state: QueueState) => {
    clientRef.current?.send({ type: 'QueueUpdate', payload: state });
  }, []);

  const sendSettingsUpdate = useCallback((settings: SyncedSettings) => {
    clientRef.current?.send({ type: 'SettingsUpdate', payload: settings });
  }, []);

  const requestTransfer = useCallback((targetDeviceId: string) => {
    clientRef.current?.transferPlayback(targetDeviceId);
  }, []);

  const requestDeviceList = useCallback(() => {
    clientRef.current?.requestDeviceList();
  }, []);

  // Return value
  return useMemo(
    () => ({
      client: clientRef.current,
      isConnected: connectionState === 'connected',
      isActiveDevice: deviceId === activeDeviceId,
      connect,
      disconnect,
      sendPlaybackUpdate,
      sendSeek,
      sendQueueUpdate,
      sendSettingsUpdate,
      requestTransfer,
      requestDeviceList,
    }),
    [
      connectionState,
      deviceId,
      activeDeviceId,
      connect,
      disconnect,
      sendPlaybackUpdate,
      sendSeek,
      sendQueueUpdate,
      sendSettingsUpdate,
      requestTransfer,
      requestDeviceList,
    ]
  );
}
