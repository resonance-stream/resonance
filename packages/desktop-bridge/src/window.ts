/**
 * Window Controls IPC types and utilities.
 *
 * This module provides TypeScript types for window management
 * via Tauri's window API. It handles minimize, maximize, close,
 * fullscreen, and other window operations.
 *
 * On macOS, this also handles traffic light button behavior
 * and frameless window interactions.
 */

import { isTauri } from './environment.js';

/** Window state */
export interface WindowState {
  /** Whether the window is maximized */
  isMaximized: boolean;
  /** Whether the window is minimized */
  isMinimized: boolean;
  /** Whether the window is fullscreen */
  isFullscreen: boolean;
  /** Whether the window is focused */
  isFocused: boolean;
  /** Whether the window is visible */
  isVisible: boolean;
  /** Whether the window is decorated (has title bar) */
  isDecorated: boolean;
}

/** Window position */
export interface WindowPosition {
  x: number;
  y: number;
}

/** Window size */
export interface WindowSize {
  width: number;
  height: number;
}

/** Window bounds (position + size) */
export interface WindowBounds {
  position: WindowPosition;
  size: WindowSize;
}

/** Window event types */
export type WindowEventType =
  | 'focus'
  | 'blur'
  | 'resize'
  | 'move'
  | 'close-requested'
  | 'destroyed'
  | 'maximized'
  | 'minimized'
  | 'restored';

/** Window event payload */
export interface WindowEvent {
  type: WindowEventType;
  /** Window label (identifier) */
  label: string;
}

/**
 * IPC command names for window operations.
 */
export const WINDOW_COMMANDS = {
  MINIMIZE: 'window_minimize',
  MAXIMIZE: 'window_maximize',
  UNMAXIMIZE: 'window_unmaximize',
  TOGGLE_MAXIMIZE: 'window_toggle_maximize',
  CLOSE: 'window_close',
  HIDE: 'window_hide',
  SHOW: 'window_show',
  SET_FULLSCREEN: 'window_set_fullscreen',
  GET_STATE: 'window_get_state',
  GET_BOUNDS: 'window_get_bounds',
  SET_BOUNDS: 'window_set_bounds',
  CENTER: 'window_center',
  SET_ALWAYS_ON_TOP: 'window_set_always_on_top',
  START_DRAGGING: 'window_start_dragging',
} as const;

/**
 * Event names for window events.
 */
export const WINDOW_EVENTS = {
  STATE_CHANGED: 'window:state-changed',
  CLOSE_REQUESTED: 'window:close-requested',
  FOCUS: 'window:focus',
  BLUR: 'window:blur',
} as const;

/**
 * Minimizes the main window.
 * Only works in Tauri context.
 */
export async function minimizeWindow(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().minimize();
}

/**
 * Maximizes the main window.
 * Only works in Tauri context.
 */
export async function maximizeWindow(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().maximize();
}

/**
 * Unmaximizes the main window.
 * Only works in Tauri context.
 */
export async function unmaximizeWindow(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().unmaximize();
}

/**
 * Toggles the maximize state of the main window.
 * Only works in Tauri context.
 */
export async function toggleMaximize(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().toggleMaximize();
}

/**
 * Closes the main window.
 * Only works in Tauri context.
 */
export async function closeWindow(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().close();
}

/**
 * Hides the main window (minimize to tray).
 * Only works in Tauri context.
 */
export async function hideWindow(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().hide();
}

/**
 * Shows the main window.
 * Only works in Tauri context.
 */
export async function showWindow(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const win = getCurrentWindow();
  await win.show();
  await win.setFocus();
}

/**
 * Sets the fullscreen state.
 * Only works in Tauri context.
 *
 * @param fullscreen - Whether to enter fullscreen
 */
export async function setFullscreen(fullscreen: boolean): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().setFullscreen(fullscreen);
}

/**
 * Toggles fullscreen mode.
 * Only works in Tauri context.
 */
export async function toggleFullscreen(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const win = getCurrentWindow();
  const isFullscreen = await win.isFullscreen();
  await win.setFullscreen(!isFullscreen);
}

/**
 * Gets the current window state.
 * Only works in Tauri context.
 *
 * @returns Window state or null if not in Tauri
 */
export async function getWindowState(): Promise<WindowState | null> {
  if (!isTauri()) {
    return null;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const win = getCurrentWindow();

  const [isMaximized, isMinimized, isFullscreen, isFocused, isVisible, isDecorated] =
    await Promise.all([
      win.isMaximized(),
      win.isMinimized(),
      win.isFullscreen(),
      win.isFocused(),
      win.isVisible(),
      win.isDecorated(),
    ]);

  return {
    isMaximized,
    isMinimized,
    isFullscreen,
    isFocused,
    isVisible,
    isDecorated,
  };
}

/**
 * Gets the current window bounds.
 * Only works in Tauri context.
 *
 * @returns Window bounds or null if not in Tauri
 */
export async function getWindowBounds(): Promise<WindowBounds | null> {
  if (!isTauri()) {
    return null;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const win = getCurrentWindow();

  const [position, size] = await Promise.all([
    win.outerPosition(),
    win.outerSize(),
  ]);

  return {
    position: { x: position.x, y: position.y },
    size: { width: size.width, height: size.height },
  };
}

/**
 * Sets the window bounds.
 * Only works in Tauri context.
 *
 * @param bounds - New window bounds
 */
export async function setWindowBounds(bounds: Partial<WindowBounds>): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow, LogicalPosition, LogicalSize } = await import(
    '@tauri-apps/api/window'
  );
  const win = getCurrentWindow();

  if (bounds.position) {
    await win.setPosition(
      new LogicalPosition(bounds.position.x, bounds.position.y)
    );
  }
  if (bounds.size) {
    await win.setSize(new LogicalSize(bounds.size.width, bounds.size.height));
  }
}

/**
 * Centers the window on the screen.
 * Only works in Tauri context.
 */
export async function centerWindow(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().center();
}

/**
 * Sets whether the window should always be on top.
 * Only works in Tauri context.
 *
 * @param alwaysOnTop - Whether to keep window on top
 */
export async function setAlwaysOnTop(alwaysOnTop: boolean): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().setAlwaysOnTop(alwaysOnTop);
}

/**
 * Starts dragging the window.
 * Call this on mousedown of a draggable region.
 * Only works in Tauri context.
 */
export async function startDragging(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  await getCurrentWindow().startDragging();
}

/**
 * Subscribes to window close requested events.
 * This allows intercepting the close action (e.g., to minimize to tray).
 * Only works in Tauri context.
 *
 * @param callback - Function to call when close is requested
 * @returns Unsubscribe function
 */
export async function onCloseRequested(
  callback: () => void
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const unlisten = await getCurrentWindow().onCloseRequested((event) => {
    event.preventDefault();
    callback();
  });
  return unlisten;
}

/**
 * Subscribes to window focus events.
 * Only works in Tauri context.
 *
 * @param callback - Function to call when window gains focus
 * @returns Unsubscribe function
 */
export async function onWindowFocus(
  callback: () => void
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const unlisten = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
    if (focused) {
      callback();
    }
  });
  return unlisten;
}

/**
 * Subscribes to window blur events.
 * Only works in Tauri context.
 *
 * @param callback - Function to call when window loses focus
 * @returns Unsubscribe function
 */
export async function onWindowBlur(
  callback: () => void
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const unlisten = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
    if (!focused) {
      callback();
    }
  });
  return unlisten;
}

/**
 * Subscribes to window resize events.
 * Only works in Tauri context.
 *
 * @param callback - Function to call with new size
 * @returns Unsubscribe function
 */
export async function onWindowResize(
  callback: (size: WindowSize) => void
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }

  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const unlisten = await getCurrentWindow().onResized(({ payload: size }) => {
    callback({ width: size.width, height: size.height });
  });
  return unlisten;
}
