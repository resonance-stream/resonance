/**
 * System Tray IPC types and utilities.
 *
 * This module provides TypeScript types for system tray integration
 * via Tauri's tray-icon feature. The system tray allows the app to
 * minimize to tray, show playback status, and provide quick actions.
 */

import { isTauri } from './environment.js';

/** Tray icon state */
export type TrayIconState = 'default' | 'playing' | 'paused' | 'loading';

/** Tray menu item types */
export type TrayMenuItemType = 'normal' | 'separator' | 'submenu' | 'checkbox';

/** Tray menu item definition */
export interface TrayMenuItem {
  /** Unique identifier for the menu item */
  id: string;
  /** Display label (not used for separators) */
  label?: string;
  /** Item type */
  type: TrayMenuItemType;
  /** Whether the item is enabled */
  enabled?: boolean;
  /** Whether the item is checked (for checkbox type) */
  checked?: boolean;
  /** Keyboard accelerator (e.g., 'CmdOrCtrl+Q') */
  accelerator?: string;
  /** Submenu items (for submenu type) */
  submenu?: TrayMenuItem[];
}

/** Playback information for tray tooltip/menu */
export interface TrayPlaybackInfo {
  /** Track title */
  title: string;
  /** Artist name */
  artist: string;
  /** Whether currently playing */
  isPlaying: boolean;
  /** Current position in seconds */
  position?: number;
  /** Total duration in seconds */
  duration?: number;
}

/** Tray menu click event */
export interface TrayMenuClickEvent {
  /** ID of the clicked menu item */
  id: string;
  /** Current checked state (for checkbox items) */
  checked?: boolean;
}

/** Tray icon click event */
export interface TrayIconClickEvent {
  /** Click type */
  type: 'left' | 'right' | 'double';
  /** Click position (screen coordinates) */
  position: { x: number; y: number };
}

/**
 * IPC command names for tray operations.
 */
export const TRAY_COMMANDS = {
  SET_ICON: 'tray_set_icon',
  SET_TOOLTIP: 'tray_set_tooltip',
  SET_MENU: 'tray_set_menu',
  UPDATE_PLAYBACK: 'tray_update_playback',
  SHOW: 'tray_show',
  HIDE: 'tray_hide',
} as const;

/**
 * Event names for tray events.
 */
export const TRAY_EVENTS = {
  MENU_CLICK: 'tray:menu-click',
  ICON_CLICK: 'tray:icon-click',
} as const;

/** Standard tray menu item IDs */
export const TRAY_MENU_IDS = {
  SHOW_WINDOW: 'show_window',
  PLAY_PAUSE: 'play_pause',
  NEXT_TRACK: 'next_track',
  PREVIOUS_TRACK: 'previous_track',
  QUIT: 'quit',
} as const;

/**
 * Sets the tray icon state.
 * Only works in Tauri context.
 *
 * @param state - The icon state to display
 */
export async function setTrayIcon(state: TrayIconState): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(TRAY_COMMANDS.SET_ICON, { state });
}

/**
 * Sets the tray icon tooltip text.
 * Only works in Tauri context.
 *
 * @param tooltip - Tooltip text to display on hover
 */
export async function setTrayTooltip(tooltip: string): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(TRAY_COMMANDS.SET_TOOLTIP, { tooltip });
}

/**
 * Sets the tray context menu.
 * Only works in Tauri context.
 *
 * @param items - Menu items to display
 */
export async function setTrayMenu(items: TrayMenuItem[]): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(TRAY_COMMANDS.SET_MENU, { items });
}

/**
 * Updates the tray with current playback information.
 * This updates the tooltip and menu to reflect playback state.
 * Only works in Tauri context.
 *
 * @param info - Current playback information
 */
export async function updateTrayPlayback(
  info: TrayPlaybackInfo
): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(TRAY_COMMANDS.UPDATE_PLAYBACK, { info });
}

/**
 * Shows the tray icon.
 * Only works in Tauri context.
 */
export async function showTray(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(TRAY_COMMANDS.SHOW);
}

/**
 * Hides the tray icon.
 * Only works in Tauri context.
 */
export async function hideTray(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(TRAY_COMMANDS.HIDE);
}

/**
 * Subscribes to tray menu click events.
 * Only works in Tauri context.
 *
 * @param callback - Function to call when a menu item is clicked
 * @returns Unsubscribe function
 */
export async function onTrayMenuClick(
  callback: (event: TrayMenuClickEvent) => void
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }

  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<TrayMenuClickEvent>(
    TRAY_EVENTS.MENU_CLICK,
    (event) => callback(event.payload)
  );
  return unlisten;
}

/**
 * Subscribes to tray icon click events.
 * Only works in Tauri context.
 *
 * @param callback - Function to call when the tray icon is clicked
 * @returns Unsubscribe function
 */
export async function onTrayIconClick(
  callback: (event: TrayIconClickEvent) => void
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }

  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<TrayIconClickEvent>(
    TRAY_EVENTS.ICON_CLICK,
    (event) => callback(event.payload)
  );
  return unlisten;
}

/**
 * Creates a standard playback menu for the tray.
 *
 * @param isPlaying - Whether music is currently playing
 * @param hasTrack - Whether a track is loaded
 * @returns Array of menu items
 */
export function createPlaybackMenu(
  isPlaying: boolean,
  hasTrack: boolean
): TrayMenuItem[] {
  return [
    {
      id: TRAY_MENU_IDS.SHOW_WINDOW,
      label: 'Show Resonance',
      type: 'normal',
      accelerator: 'CmdOrCtrl+Shift+R',
    },
    { id: 'sep1', type: 'separator' },
    {
      id: TRAY_MENU_IDS.PREVIOUS_TRACK,
      label: 'Previous Track',
      type: 'normal',
      enabled: hasTrack,
    },
    {
      id: TRAY_MENU_IDS.PLAY_PAUSE,
      label: isPlaying ? 'Pause' : 'Play',
      type: 'normal',
      enabled: hasTrack,
    },
    {
      id: TRAY_MENU_IDS.NEXT_TRACK,
      label: 'Next Track',
      type: 'normal',
      enabled: hasTrack,
    },
    { id: 'sep2', type: 'separator' },
    {
      id: TRAY_MENU_IDS.QUIT,
      label: 'Quit Resonance',
      type: 'normal',
      accelerator: 'CmdOrCtrl+Q',
    },
  ];
}

/**
 * Creates a tooltip string from playback information.
 *
 * @param info - Playback information or null if nothing playing
 * @returns Tooltip string
 */
export function createPlaybackTooltip(
  info: TrayPlaybackInfo | null
): string {
  if (!info) {
    return 'Resonance';
  }

  const status = info.isPlaying ? 'Playing' : 'Paused';
  return `${info.title} - ${info.artist}\n${status}`;
}
