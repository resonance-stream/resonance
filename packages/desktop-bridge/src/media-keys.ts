/**
 * Media Keys IPC types and utilities.
 *
 * This module provides TypeScript types for global media key handling
 * via Tauri's global-shortcut plugin. Media keys include play/pause,
 * next/previous track, stop, and volume controls.
 *
 * Global shortcuts are registered at the OS level, so they work even
 * when the application is not focused.
 */

import { isTauri } from './environment.js';

/** Standard media key actions */
export type MediaKeyAction =
  | 'play'
  | 'pause'
  | 'play_pause'
  | 'stop'
  | 'next'
  | 'previous'
  | 'volume_up'
  | 'volume_down'
  | 'mute';

/** Media key event payload */
export interface MediaKeyEvent {
  /** The media key action that was triggered */
  action: MediaKeyAction;
  /** Timestamp when the key was pressed */
  timestamp: number;
}

/** Shortcut key identifiers for different platforms */
export interface MediaKeyShortcuts {
  playPause: string;
  stop: string;
  nextTrack: string;
  previousTrack: string;
  volumeUp: string;
  volumeDown: string;
  mute: string;
}

/**
 * Default media key shortcuts.
 * These are the standard media key identifiers across platforms.
 */
export const DEFAULT_MEDIA_SHORTCUTS: MediaKeyShortcuts = {
  playPause: 'MediaPlayPause',
  stop: 'MediaStop',
  nextTrack: 'MediaTrackNext',
  previousTrack: 'MediaTrackPrevious',
  volumeUp: 'AudioVolumeUp',
  volumeDown: 'AudioVolumeDown',
  mute: 'AudioVolumeMute',
};

/**
 * IPC command names for media key operations.
 */
export const MEDIA_KEY_COMMANDS = {
  REGISTER_ALL: 'media_keys_register_all',
  UNREGISTER_ALL: 'media_keys_unregister_all',
  IS_REGISTERED: 'media_keys_is_registered',
} as const;

/**
 * Event names for media key events.
 */
export const MEDIA_KEY_EVENTS = {
  PRESSED: 'media-key:pressed',
} as const;

/** Callback type for media key handlers */
export type MediaKeyHandler = (action: MediaKeyAction) => void;

/** Registered handler storage */
let mediaKeyHandlers: Set<MediaKeyHandler> = new Set();
let isListenerRegistered = false;

/**
 * Registers global media key shortcuts.
 * Only works in Tauri context with the global-shortcut plugin.
 *
 * @returns True if registration was successful
 */
export async function registerMediaKeys(): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke(MEDIA_KEY_COMMANDS.REGISTER_ALL);
    await setupEventListener();
    return true;
  } catch {
    return false;
  }
}

/**
 * Unregisters all global media key shortcuts.
 * Only works in Tauri context.
 */
export async function unregisterMediaKeys(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke(MEDIA_KEY_COMMANDS.UNREGISTER_ALL);
  } catch {
    // Ignore errors during cleanup
  }
}

/**
 * Checks if media keys are currently registered.
 * Only works in Tauri context.
 *
 * @returns True if media keys are registered
 */
export async function isMediaKeysRegistered(): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<boolean>(MEDIA_KEY_COMMANDS.IS_REGISTERED);
  } catch {
    return false;
  }
}

/**
 * Sets up the Tauri event listener for media key events.
 */
async function setupEventListener(): Promise<void> {
  if (isListenerRegistered || !isTauri()) {
    return;
  }

  const { listen } = await import('@tauri-apps/api/event');
  await listen<MediaKeyEvent>(MEDIA_KEY_EVENTS.PRESSED, (event) => {
    const { action } = event.payload;
    mediaKeyHandlers.forEach((handler) => handler(action));
  });

  isListenerRegistered = true;
}

/**
 * Adds a handler for media key events.
 *
 * @param handler - Function to call when a media key is pressed
 * @returns Function to remove the handler
 */
export function onMediaKey(handler: MediaKeyHandler): () => void {
  mediaKeyHandlers.add(handler);
  return () => {
    mediaKeyHandlers.delete(handler);
  };
}

/**
 * Removes all media key handlers.
 */
export function clearMediaKeyHandlers(): void {
  mediaKeyHandlers.clear();
}

/**
 * Creates a media key handler that dispatches to specific callbacks.
 * Convenience function for routing media key actions.
 *
 * @param handlers - Object mapping actions to callbacks
 * @returns Unified handler function
 */
export function createMediaKeyDispatcher(handlers: {
  onPlay?: () => void;
  onPause?: () => void;
  onPlayPause?: () => void;
  onStop?: () => void;
  onNext?: () => void;
  onPrevious?: () => void;
  onVolumeUp?: () => void;
  onVolumeDown?: () => void;
  onMute?: () => void;
}): MediaKeyHandler {
  return (action: MediaKeyAction) => {
    switch (action) {
      case 'play':
        handlers.onPlay?.();
        break;
      case 'pause':
        handlers.onPause?.();
        break;
      case 'play_pause':
        handlers.onPlayPause?.();
        break;
      case 'stop':
        handlers.onStop?.();
        break;
      case 'next':
        handlers.onNext?.();
        break;
      case 'previous':
        handlers.onPrevious?.();
        break;
      case 'volume_up':
        handlers.onVolumeUp?.();
        break;
      case 'volume_down':
        handlers.onVolumeDown?.();
        break;
      case 'mute':
        handlers.onMute?.();
        break;
    }
  };
}
