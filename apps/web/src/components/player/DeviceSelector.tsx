/**
 * Device Selector Component
 *
 * Displays connected devices and allows transferring playback between them.
 * Shows device status, active device indicator, and connection state.
 * Allows editing the current device's nickname inline.
 */

import { useState, useRef, useEffect, useCallback } from 'react';
import { Pencil, Check, X } from 'lucide-react';
import { useDeviceStore, useIsConnected, useIsActiveDevice, useOtherDevices } from '../../stores/deviceStore';
import { useSync } from '../../providers/SyncProvider';
import { Input } from '../ui/Input';
import { Button } from '../ui/Button';
import type { DevicePresence } from '../../sync/types';

interface DeviceSelectorProps {
  /** Callback when user requests to transfer playback to a device */
  onTransfer?: (deviceId: string) => void;
  /** Whether selector is in compact mode (e.g., in player bar) */
  compact?: boolean;
}

/** Map device type to icon name (for use with icon library) */
function getDeviceIcon(type: DevicePresence['device_type']): string {
  switch (type) {
    case 'mobile':
      return 'smartphone';
    case 'tablet':
      return 'tablet';
    case 'desktop':
      return 'monitor';
    case 'speaker':
      return 'speaker';
    case 'web':
      return 'globe';
    default:
      return 'device';
  }
}

/** Format last seen timestamp */
function formatLastSeen(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;

  if (diff < 60_000) {
    return 'Just now';
  } else if (diff < 3600_000) {
    const mins = Math.floor(diff / 60_000);
    return `${mins}m ago`;
  } else if (diff < 86400_000) {
    const hours = Math.floor(diff / 3600_000);
    return `${hours}h ago`;
  } else {
    const days = Math.floor(diff / 86400_000);
    return `${days}d ago`;
  }
}

export function DeviceSelector({ onTransfer, compact = false }: DeviceSelectorProps): JSX.Element {
  const isConnected = useIsConnected();
  const isActiveDevice = useIsActiveDevice();
  const otherDevices = useOtherDevices();
  const connectionState = useDeviceStore((s) => s.connectionState);
  const deviceName = useDeviceStore((s) => s.deviceName);
  const deviceType = useDeviceStore((s) => s.deviceType);
  const setDeviceName = useDeviceStore((s) => s.setDeviceName);

  // Get requestControl from sync context for taking control of playback
  const { requestControl } = useSync();

  // Editing state for device nickname
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(deviceName);
  const [isHovering, setIsHovering] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus input when entering edit mode
  useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isEditing]);

  // Start editing
  const handleStartEdit = useCallback(() => {
    setEditName(deviceName);
    setIsEditing(true);
  }, [deviceName]);

  // Save the edited name
  const handleSave = useCallback(() => {
    const trimmedName = editName.trim();
    if (trimmedName && trimmedName !== deviceName) {
      setDeviceName(trimmedName);
    }
    setIsEditing(false);
  }, [editName, deviceName, setDeviceName]);

  // Cancel editing
  const handleCancel = useCallback(() => {
    setEditName(deviceName);
    setIsEditing(false);
  }, [deviceName]);

  // Handle keyboard events
  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSave();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      handleCancel();
    }
  }, [handleSave, handleCancel]);

  // Handle transfer request
  const handleTransfer = (targetDeviceId: string) => {
    onTransfer?.(targetDeviceId);
  };

  // Compact mode - just show icon with device count
  if (compact) {
    const deviceCount = otherDevices.length + 1; // +1 for this device

    return (
      <button
        className="device-selector-compact"
        title={`${deviceCount} device${deviceCount !== 1 ? 's' : ''} connected`}
        disabled={!isConnected}
      >
        <span className="device-icon">{getDeviceIcon(deviceType)}</span>
        {otherDevices.length > 0 && (
          <span className="device-count">{deviceCount}</span>
        )}
      </button>
    );
  }

  // Full mode - show device list
  return (
    <div className="device-selector" role="region" aria-label="Device selection">
      <div className="device-selector-header">
        <h3 id="device-selector-title">Devices</h3>
        <span
          className={`connection-status connection-status--${connectionState}`}
          role="status"
          aria-live="polite"
        >
          {connectionState === 'connected' ? 'Connected' :
           connectionState === 'connecting' ? 'Connecting...' :
           connectionState === 'reconnecting' ? 'Reconnecting...' :
           'Disconnected'}
        </span>
      </div>

      <div
        className="device-list"
        role="listbox"
        aria-labelledby="device-selector-title"
        aria-activedescendant={isActiveDevice ? 'device-current' : undefined}
      >
        {/* This device */}
        <div
          id="device-current"
          role="option"
          aria-selected={isActiveDevice}
          className={`device-item device-item--current ${isActiveDevice ? 'device-item--active' : ''}`}
          onMouseEnter={() => setIsHovering(true)}
          onMouseLeave={() => setIsHovering(false)}
        >
          <span className="device-icon" aria-hidden="true">{getDeviceIcon(deviceType)}</span>
          <div className="device-info">
            {isEditing ? (
              <div className="device-name-edit">
                <Input
                  ref={inputRef}
                  type="text"
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  onKeyDown={handleKeyDown}
                  onBlur={handleCancel}
                  className="device-name-input"
                  aria-label="Device nickname"
                  maxLength={50}
                />
                <div className="device-name-actions">
                  <Button
                    variant="icon"
                    size="icon"
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={handleSave}
                    aria-label="Save device name"
                    title="Save"
                    className="device-name-save"
                  >
                    <Check size={16} />
                  </Button>
                  <Button
                    variant="icon"
                    size="icon"
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={handleCancel}
                    aria-label="Cancel editing"
                    title="Cancel"
                    className="device-name-cancel"
                  >
                    <X size={16} />
                  </Button>
                </div>
              </div>
            ) : (
              <span className="device-name">
                {deviceName}
                {isHovering && !isEditing && (
                  <button
                    className="device-name-edit-btn"
                    onClick={handleStartEdit}
                    aria-label="Edit device name"
                    title="Edit device name"
                  >
                    <Pencil size={14} />
                  </button>
                )}
              </span>
            )}
            <span className="device-label">
              This device {isActiveDevice && '(Playing)'}
            </span>
          </div>
          {!isActiveDevice && isConnected && !isEditing && (
            <button
              className="px-3 py-1.5 rounded-lg font-medium text-sm bg-navy text-white hover:bg-navy-hover active:bg-navy-active shadow-[0_0_20px_rgba(37,99,235,0.3)] hover:shadow-[0_0_25px_rgba(37,99,235,0.4)] focus:outline-none focus-visible:ring-2 focus-visible:ring-navy-glow transition-all duration-150"
              onClick={requestControl}
              aria-label={`Take control of playback on ${deviceName}`}
            >
              Take Control
            </button>
          )}
        </div>

        {/* Other devices */}
        {otherDevices.map((device) => (
          <div
            key={device.device_id}
            id={`device-${device.device_id}`}
            role="option"
            aria-selected={device.is_active}
            className={`device-item ${device.is_active ? 'device-item--active' : ''}`}
          >
            <span className="device-icon" aria-hidden="true">{getDeviceIcon(device.device_type)}</span>
            <div className="device-info">
              <span className="device-name">{device.device_name}</span>
              <span className="device-label">
                {device.is_active && device.current_track
                  ? `Playing: ${device.current_track.title}`
                  : device.is_active
                    ? 'Playing'
                    : formatLastSeen(device.last_seen)}
              </span>
            </div>
            {!device.is_active && (
              <button
                className="device-transfer-btn"
                onClick={() => handleTransfer(device.device_id)}
                aria-label={`Transfer playback to ${device.device_name}`}
              >
                Transfer
              </button>
            )}
          </div>
        ))}

        {otherDevices.length === 0 && (
          <div className="device-empty">
            <span>No other devices connected</span>
            <span className="device-empty-hint">
              Open Resonance on another device to sync playback
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
