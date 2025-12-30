/**
 * Discord Rich Presence service
 *
 * Displays "Now Playing" information in Discord.
 * Only functional in desktop app environments (Electron/Tauri).
 * Web/PWA builds gracefully degrade to no-op.
 *
 * Discord RPC requires IPC communication with the local Discord client,
 * which is only possible in desktop applications.
 */

import { detectDeviceType } from '../sync/types'

/** Track information for Discord presence */
export interface PresenceTrack {
  title: string
  artist: string
  album?: string
  duration?: number // in seconds
  coverUrl?: string
}

/** Current playback state for presence */
export interface PresenceState {
  track: PresenceTrack | null
  isPlaying: boolean
  position?: number // in seconds
}

/** Discord RPC availability status */
export type RpcAvailability =
  | { available: true }
  | { available: false; reason: 'not_desktop' | 'not_installed' | 'not_supported' }

/**
 * Discord Rich Presence service interface
 */
export interface DiscordRpcService {
  /** Check if Discord RPC is available in this environment */
  checkAvailability(): RpcAvailability

  /** Update Discord presence with current track */
  setPresence(state: PresenceState): Promise<void>

  /** Clear Discord presence */
  clearPresence(): Promise<void>

  /** Whether the service is enabled */
  isEnabled(): boolean

  /** Enable/disable the service */
  setEnabled(enabled: boolean): void
}

/**
 * Check if running in a desktop environment
 */
function isDesktopEnvironment(): boolean {
  const deviceType = detectDeviceType()
  return deviceType === 'desktop'
}

/**
 * Web/PWA stub implementation of Discord RPC
 *
 * This is a no-op implementation used when Discord RPC is not available.
 * Provides the same interface but logs warnings instead of connecting.
 */
class WebDiscordRpc implements DiscordRpcService {
  private enabled = false
  private hasLoggedWarning = false

  checkAvailability(): RpcAvailability {
    return { available: false, reason: 'not_desktop' }
  }

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  async setPresence(_state: PresenceState): Promise<void> {
    if (!this.hasLoggedWarning && this.enabled) {
      console.debug('[DiscordRpc] Discord Rich Presence is only available in the desktop app')
      this.hasLoggedWarning = true
    }
  }

  async clearPresence(): Promise<void> {
    // No-op in web environment
  }

  isEnabled(): boolean {
    return this.enabled
  }

  setEnabled(enabled: boolean): void {
    this.enabled = enabled
    if (enabled && !this.hasLoggedWarning) {
      console.debug('[DiscordRpc] Discord Rich Presence is only available in the desktop app')
      this.hasLoggedWarning = true
    }
  }
}

/**
 * Desktop implementation of Discord RPC
 *
 * Uses the Electron/Tauri IPC bridge to communicate with the Discord client.
 * This implementation is a placeholder - actual IPC integration will be added
 * when the desktop app shell is implemented.
 */
class DesktopDiscordRpc implements DiscordRpcService {
  private enabled = false
  private currentState: PresenceState | null = null

  // Discord Application ID - to be set when registering with Discord
  // private static readonly CLIENT_ID = 'YOUR_DISCORD_CLIENT_ID'

  checkAvailability(): RpcAvailability {
    // In the future, check if Discord IPC is actually available
    // For now, assume it's available if we're in desktop mode
    return { available: true }
  }

  async setPresence(state: PresenceState): Promise<void> {
    if (!this.enabled) return

    this.currentState = state

    if (!state.track || !state.isPlaying) {
      await this.clearPresence()
      return
    }

    // Build presence payload (track is guaranteed non-null by guard above)
    const presence = this.buildPresence(state.track, state.isPlaying, state.position)

    // In the future, send via Electron/Tauri IPC bridge
    // For now, log the intent
    console.debug('[DiscordRpc] Would set presence:', presence)
  }

  async clearPresence(): Promise<void> {
    if (!this.enabled) return

    this.currentState = null

    // In the future, clear via Electron/Tauri IPC bridge
    console.debug('[DiscordRpc] Would clear presence')
  }

  isEnabled(): boolean {
    return this.enabled
  }

  setEnabled(enabled: boolean): void {
    const wasEnabled = this.enabled
    this.enabled = enabled

    // Clear currentState when disabling to free memory
    if (!enabled) {
      this.currentState = null
    }

    if (wasEnabled && !enabled) {
      // Clear presence when disabling
      this.clearPresence().catch(console.error)
    } else if (!wasEnabled && enabled && this.currentState) {
      // Re-apply presence when enabling
      this.setPresence(this.currentState).catch(console.error)
    }
  }

  /**
   * Build Discord presence payload from track information
   */
  private buildPresence(
    track: PresenceTrack,
    isPlaying: boolean,
    position?: number
  ): DiscordPresencePayload {
    const now = Date.now()

    // Calculate timestamps for progress bar
    let timestamps: { start?: number; end?: number } | undefined
    if (position !== undefined && track.duration !== undefined) {
      const startTime = now - position * 1000
      const endTime = startTime + track.duration * 1000
      timestamps = { start: startTime, end: endTime }
    }

    return {
      details: track.title,
      state: `by ${track.artist}${track.album ? ` • ${track.album}` : ''}`,
      timestamps,
      largeImageKey: track.coverUrl || 'resonance_logo',
      largeImageText: track.album || 'Resonance',
      smallImageKey: isPlaying ? 'play' : 'pause',
      smallImageText: isPlaying ? 'Playing' : 'Paused',
      instance: true,
    }
  }
}

/** Discord presence payload structure */
interface DiscordPresencePayload {
  details: string
  state: string
  timestamps?: {
    start?: number
    end?: number
  }
  largeImageKey?: string
  largeImageText?: string
  smallImageKey?: string
  smallImageText?: string
  instance?: boolean
}

/**
 * Create a Discord RPC service instance
 *
 * Returns the appropriate implementation based on the runtime environment.
 */
export function createDiscordRpcService(): DiscordRpcService {
  if (isDesktopEnvironment()) {
    return new DesktopDiscordRpc()
  }
  return new WebDiscordRpc()
}

// Export singleton instance
let serviceInstance: DiscordRpcService | null = null

/**
 * Get the Discord RPC service singleton
 */
export function getDiscordRpcService(): DiscordRpcService {
  if (!serviceInstance) {
    serviceInstance = createDiscordRpcService()
  }
  return serviceInstance
}
