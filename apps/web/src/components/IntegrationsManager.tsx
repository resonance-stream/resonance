/**
 * Integrations Manager component
 *
 * Activates and manages external service integrations:
 * - ListenBrainz scrobbling (monitors playback, submits scrobbles)
 * - Discord Rich Presence (shows "Now Playing" in Discord)
 *
 * This component is a render-less manager that runs integration hooks.
 * It should be mounted once inside the app, after AudioProvider.
 * Integrations only run when user is authenticated.
 */

import { useScrobble } from '../hooks/useScrobble'
import { useDiscordRpc } from '../hooks/useDiscordRpc'
import { useAuthStore } from '../stores/authStore'

// Enable debug logging in development
const DEBUG = import.meta.env.DEV

/**
 * Inner component that actually runs the integration hooks.
 * Only mounted when user is authenticated.
 */
function ActiveIntegrations(): null {
  // ListenBrainz scrobbling - tracks playback and submits listens
  useScrobble({ debug: DEBUG })

  // Discord Rich Presence - updates Discord status with current track
  useDiscordRpc({ debug: DEBUG })

  return null
}

/**
 * Wrapper that checks authentication before activating integrations.
 * Prevents API calls and unnecessary subscriptions on public routes.
 */
export function IntegrationsManager(): JSX.Element | null {
  const status = useAuthStore((s) => s.status)
  const user = useAuthStore((s) => s.user)

  // Only run integrations when user is authenticated
  const isAuthenticated = status === 'authenticated' && user !== null

  if (!isAuthenticated) {
    return null
  }

  return <ActiveIntegrations />
}
