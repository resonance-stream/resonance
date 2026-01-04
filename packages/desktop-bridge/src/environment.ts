/**
 * Environment detection utilities for Tauri vs Web contexts.
 *
 * This module provides runtime detection of whether the application is running
 * in a Tauri desktop context or a regular web browser. This is essential for
 * conditionally enabling desktop-specific features like Discord RPC, media keys,
 * system tray integration, and window controls.
 */

/** Platform information when running in Tauri */
export interface TauriPlatformInfo {
  /** Operating system: 'windows' | 'macos' | 'linux' | 'android' | 'ios' */
  os: string;
  /** OS version string */
  version: string;
  /** Architecture: 'x86_64' | 'aarch64' | 'arm' | etc. */
  arch: string;
  /** Tauri version */
  tauriVersion: string;
}

/** Environment context type */
export type EnvironmentContext = 'tauri' | 'web';

/** Cache for environment detection results */
let cachedIsTauri: boolean | null = null;
let cachedPlatformInfo: TauriPlatformInfo | null = null;

/**
 * Checks if the Tauri API is available in the current environment.
 * This is a lightweight check that doesn't require loading the Tauri API.
 *
 * @returns True if running in Tauri desktop context
 */
export function isTauri(): boolean {
  if (cachedIsTauri !== null) {
    return cachedIsTauri;
  }

  // Check for Tauri 2.x API presence
  // The __TAURI_INTERNALS__ object is injected by Tauri into the webview
  cachedIsTauri =
    typeof window !== 'undefined' &&
    '__TAURI_INTERNALS__' in window;

  return cachedIsTauri;
}

/**
 * Checks if running in a standard web browser context.
 *
 * @returns True if running in a web browser (not Tauri)
 */
export function isWeb(): boolean {
  return !isTauri();
}

/**
 * Gets the current environment context.
 *
 * @returns 'tauri' or 'web'
 */
export function getEnvironment(): EnvironmentContext {
  return isTauri() ? 'tauri' : 'web';
}

/**
 * Gets platform information when running in Tauri.
 * Returns null if running in a web browser.
 *
 * This function is async because it may need to invoke Tauri APIs.
 * Note: Full OS info requires the OS plugin to be installed on the Rust side.
 *
 * @returns Platform information or null if not in Tauri
 */
export async function getPlatformInfo(): Promise<TauriPlatformInfo | null> {
  if (!isTauri()) {
    return null;
  }

  if (cachedPlatformInfo !== null) {
    return cachedPlatformInfo;
  }

  try {
    const { getTauriVersion } = await import('@tauri-apps/api/app');
    const { invoke } = await import('@tauri-apps/api/core');
    const tauriVer = await getTauriVersion();

    let osName = 'unknown';
    let osVersion = 'unknown';
    let osArch = 'unknown';

    try {
      // Try to get OS info via plugin commands if available
      // These commands are provided by @tauri-apps/plugin-os on the Rust side
      const [platform, version, arch] = await Promise.all([
        invoke<string>('plugin:os|platform').catch(() => detectPlatformFromNavigator()),
        invoke<string>('plugin:os|version').catch(() => 'unknown'),
        invoke<string>('plugin:os|arch').catch(() => 'unknown'),
      ]);
      osName = platform;
      osVersion = version;
      osArch = arch;
    } catch {
      // OS plugin not installed, use basic detection
      osName = detectPlatformFromNavigator();
    }

    cachedPlatformInfo = {
      os: osName,
      version: osVersion,
      arch: osArch,
      tauriVersion: tauriVer,
    };

    return cachedPlatformInfo;
  } catch {
    // If Tauri API calls fail, return null
    return null;
  }
}

/**
 * Fallback platform detection using navigator.
 * Used when @tauri-apps/plugin-os is not available.
 */
function detectPlatformFromNavigator(): string {
  if (typeof navigator === 'undefined') {
    return 'unknown';
  }

  const platform = navigator.platform.toLowerCase();
  if (platform.includes('mac')) return 'darwin';
  if (platform.includes('win')) return 'windows';
  if (platform.includes('linux')) return 'linux';
  return 'unknown';
}

/**
 * Checks if the current platform is macOS.
 * Returns false if not in Tauri context.
 */
export async function isMacOS(): Promise<boolean> {
  const info = await getPlatformInfo();
  return info?.os === 'darwin';
}

/**
 * Checks if the current platform is Windows.
 * Returns false if not in Tauri context.
 */
export async function isWindows(): Promise<boolean> {
  const info = await getPlatformInfo();
  return info?.os === 'windows';
}

/**
 * Checks if the current platform is Linux.
 * Returns false if not in Tauri context.
 */
export async function isLinux(): Promise<boolean> {
  const info = await getPlatformInfo();
  return info?.os === 'linux';
}

/**
 * Executes a callback only when running in Tauri context.
 * Useful for conditionally enabling desktop features.
 *
 * @param callback Function to execute in Tauri context
 * @returns Result of callback or undefined if not in Tauri
 */
export function whenTauri<T>(callback: () => T): T | undefined {
  if (isTauri()) {
    return callback();
  }
  return undefined;
}

/**
 * Executes an async callback only when running in Tauri context.
 *
 * @param callback Async function to execute in Tauri context
 * @returns Result of callback or undefined if not in Tauri
 */
export async function whenTauriAsync<T>(
  callback: () => Promise<T>
): Promise<T | undefined> {
  if (isTauri()) {
    return callback();
  }
  return undefined;
}

/**
 * Clears cached environment detection results.
 * Useful for testing or when environment might change (rare).
 */
export function clearEnvironmentCache(): void {
  cachedIsTauri = null;
  cachedPlatformInfo = null;
}
