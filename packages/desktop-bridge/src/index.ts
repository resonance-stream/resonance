/**
 * @resonance/desktop-bridge
 *
 * TypeScript IPC types and utilities for Tauri desktop integration.
 * This package provides type-safe communication between the React frontend
 * and the Tauri Rust backend for desktop-specific features.
 *
 * Features:
 * - Environment detection (Tauri vs web browser)
 * - Discord Rich Presence integration
 * - Global media key handling
 * - System tray integration
 * - Window controls (minimize, maximize, close, etc.)
 *
 * @example
 * ```typescript
 * import { isTauri, onMediaKey, createMediaKeyDispatcher } from '@resonance/desktop-bridge';
 *
 * if (isTauri()) {
 *   // Register media key handlers
 *   const dispatcher = createMediaKeyDispatcher({
 *     onPlayPause: () => playerStore.togglePlayback(),
 *     onNext: () => playerStore.nextTrack(),
 *     onPrevious: () => playerStore.previousTrack(),
 *   });
 *   onMediaKey(dispatcher);
 * }
 * ```
 */

// Environment detection
export {
  isTauri,
  isWeb,
  getEnvironment,
  getPlatformInfo,
  isMacOS,
  isWindows,
  isLinux,
  whenTauri,
  whenTauriAsync,
  clearEnvironmentCache,
  type EnvironmentContext,
  type TauriPlatformInfo,
} from './environment.js';

// Discord RPC
export {
  setDiscordActivity,
  clearDiscordActivity,
  connectDiscord,
  disconnectDiscord,
  getDiscordStatus,
  onDiscordStatusChange,
  createActivityFromTrack,
  DISCORD_COMMANDS,
  DISCORD_EVENTS,
  type DiscordActivity,
  type DiscordActivityType,
  type DiscordAssets,
  type DiscordButton,
  type DiscordConnectionStatus,
  type DiscordIpcCommand,
  type DiscordStatusEvent,
  type DiscordTimestamps,
  type DiscordTrackInfo,
} from './discord.js';

// Media keys
export {
  registerMediaKeys,
  unregisterMediaKeys,
  isMediaKeysRegistered,
  onMediaKey,
  clearMediaKeyHandlers,
  createMediaKeyDispatcher,
  DEFAULT_MEDIA_SHORTCUTS,
  MEDIA_KEY_COMMANDS,
  MEDIA_KEY_EVENTS,
  type MediaKeyAction,
  type MediaKeyEvent,
  type MediaKeyHandler,
  type MediaKeyShortcuts,
} from './media-keys.js';

// System tray
export {
  setTrayIcon,
  setTrayTooltip,
  setTrayMenu,
  updateTrayPlayback,
  showTray,
  hideTray,
  onTrayMenuClick,
  onTrayIconClick,
  createPlaybackMenu,
  createPlaybackTooltip,
  TRAY_COMMANDS,
  TRAY_EVENTS,
  TRAY_MENU_IDS,
  type TrayIconState,
  type TrayMenuItem,
  type TrayMenuItemType,
  type TrayMenuClickEvent,
  type TrayIconClickEvent,
  type TrayPlaybackInfo,
} from './tray.js';

// Window controls
export {
  minimizeWindow,
  maximizeWindow,
  unmaximizeWindow,
  toggleMaximize,
  closeWindow,
  hideWindow,
  showWindow,
  setFullscreen,
  toggleFullscreen,
  getWindowState,
  getWindowBounds,
  setWindowBounds,
  centerWindow,
  setAlwaysOnTop,
  startDragging,
  onCloseRequested,
  onWindowFocus,
  onWindowBlur,
  onWindowResize,
  WINDOW_COMMANDS,
  WINDOW_EVENTS,
  type WindowState,
  type WindowPosition,
  type WindowSize,
  type WindowBounds,
  type WindowEvent,
  type WindowEventType,
} from './window.js';
