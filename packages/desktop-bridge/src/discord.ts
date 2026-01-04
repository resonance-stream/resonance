/**
 * Discord Rich Presence IPC types and utilities.
 *
 * This module provides TypeScript types for Discord Rich Presence integration
 * via Tauri IPC. The actual Discord RPC implementation lives in the Rust backend;
 * this module defines the message types for communication.
 *
 * Discord Rich Presence shows "Now Playing" information in the user's Discord
 * profile, including track name, artist, album art, and playback progress.
 */

import { isTauri } from './environment.js';

/** Discord Rich Presence activity type */
export type DiscordActivityType = 'listening' | 'playing' | 'watching';

/** Timestamps for Discord Rich Presence */
export interface DiscordTimestamps {
  /** Unix timestamp (ms) when the activity started */
  start?: number;
  /** Unix timestamp (ms) when the activity will end (for progress bar) */
  end?: number;
}

/** Asset configuration for Discord Rich Presence */
export interface DiscordAssets {
  /** Large image key (album art) */
  largeImage?: string;
  /** Tooltip text for large image */
  largeText?: string;
  /** Small image key (app icon overlay) */
  smallImage?: string;
  /** Tooltip text for small image */
  smallText?: string;
}

/** Button configuration for Discord Rich Presence */
export interface DiscordButton {
  /** Button label (max 32 characters) */
  label: string;
  /** URL to open when button is clicked */
  url: string;
}

/** Full Discord Rich Presence activity payload */
export interface DiscordActivity {
  /** Activity type */
  type: DiscordActivityType;
  /** First line of the activity (track name) */
  details?: string;
  /** Second line of the activity (artist name) */
  state?: string;
  /** Timestamps for progress display */
  timestamps?: DiscordTimestamps;
  /** Image assets */
  assets?: DiscordAssets;
  /** Buttons (max 2) */
  buttons?: [DiscordButton] | [DiscordButton, DiscordButton];
}

/** IPC command types for Discord RPC */
export type DiscordIpcCommand =
  | { type: 'set_activity'; activity: DiscordActivity }
  | { type: 'clear_activity' }
  | { type: 'connect' }
  | { type: 'disconnect' };

/** Discord RPC connection status */
export type DiscordConnectionStatus =
  | 'connected'
  | 'disconnected'
  | 'connecting'
  | 'error';

/** Event payload from Discord RPC */
export interface DiscordStatusEvent {
  status: DiscordConnectionStatus;
  error?: string;
}

/** Track information for updating Discord presence */
export interface DiscordTrackInfo {
  /** Track title */
  title: string;
  /** Artist name */
  artist: string;
  /** Album name */
  album?: string;
  /** Album art URL or data URL */
  albumArt?: string;
  /** Track duration in milliseconds */
  durationMs: number;
  /** Current position in milliseconds */
  positionMs: number;
  /** Whether the track is currently playing */
  isPlaying: boolean;
}

/**
 * IPC command names for Tauri invoke.
 * These correspond to #[tauri::command] functions in Rust.
 */
export const DISCORD_COMMANDS = {
  SET_ACTIVITY: 'discord_set_activity',
  CLEAR_ACTIVITY: 'discord_clear_activity',
  CONNECT: 'discord_connect',
  DISCONNECT: 'discord_disconnect',
  GET_STATUS: 'discord_get_status',
} as const;

/**
 * Event names for Discord RPC events.
 * These are emitted from Rust via Tauri event system.
 */
export const DISCORD_EVENTS = {
  STATUS_CHANGE: 'discord:status-change',
  ERROR: 'discord:error',
} as const;

/**
 * Sets Discord Rich Presence activity.
 * Only works in Tauri context.
 *
 * @param activity - The activity to display
 * @throws If not in Tauri context or if IPC fails
 */
export async function setDiscordActivity(
  activity: DiscordActivity
): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(DISCORD_COMMANDS.SET_ACTIVITY, { activity });
}

/**
 * Clears Discord Rich Presence activity.
 * Only works in Tauri context.
 */
export async function clearDiscordActivity(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(DISCORD_COMMANDS.CLEAR_ACTIVITY);
}

/**
 * Connects to Discord RPC.
 * Only works in Tauri context.
 */
export async function connectDiscord(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(DISCORD_COMMANDS.CONNECT);
}

/**
 * Disconnects from Discord RPC.
 * Only works in Tauri context.
 */
export async function disconnectDiscord(): Promise<void> {
  if (!isTauri()) {
    return;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(DISCORD_COMMANDS.DISCONNECT);
}

/**
 * Gets the current Discord RPC connection status.
 * Only works in Tauri context.
 *
 * @returns Connection status or 'disconnected' if not in Tauri
 */
export async function getDiscordStatus(): Promise<DiscordConnectionStatus> {
  if (!isTauri()) {
    return 'disconnected';
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<DiscordConnectionStatus>(DISCORD_COMMANDS.GET_STATUS);
}

/**
 * Subscribes to Discord RPC status change events.
 * Only works in Tauri context.
 *
 * @param callback - Function to call when status changes
 * @returns Unsubscribe function
 */
export async function onDiscordStatusChange(
  callback: (event: DiscordStatusEvent) => void
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }

  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<DiscordStatusEvent>(
    DISCORD_EVENTS.STATUS_CHANGE,
    (event) => callback(event.payload)
  );
  return unlisten;
}

/**
 * Creates a Discord activity from track information.
 * Helper function to convert playback state to Discord activity format.
 *
 * @param track - Current track information
 * @returns Discord activity ready for display
 */
export function createActivityFromTrack(
  track: DiscordTrackInfo
): DiscordActivity {
  const activity: DiscordActivity = {
    type: 'listening',
    details: track.title,
    state: track.artist,
    assets: {
      largeImage: track.albumArt ?? 'resonance_icon',
      largeText: track.album ?? track.title,
      smallImage: track.isPlaying ? 'playing' : 'paused',
      smallText: track.isPlaying ? 'Playing' : 'Paused',
    },
  };

  // Add timestamps for progress bar (only when playing)
  if (track.isPlaying && track.durationMs > 0) {
    const now = Date.now();
    activity.timestamps = {
      start: now - track.positionMs,
      end: now + (track.durationMs - track.positionMs),
    };
  }

  return activity;
}
