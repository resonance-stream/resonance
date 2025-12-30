/**
 * Integrations settings component
 *
 * Provides UI for configuring external service integrations:
 * - ListenBrainz scrobbling
 * - Discord Rich Presence
 */

import { useState, useCallback } from 'react'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../ui/Card'
import { Button } from '../ui/Button'
import { Switch } from '../ui/Switch'
import { Input } from '../ui/Input'
import {
  useIntegrations,
  useUpdateIntegrations,
  useTestListenbrainzConnection,
} from '../../hooks/useIntegrations'

export function IntegrationsSettings() {
  // Token input state (local state, not synced until save)
  const [tokenInput, setTokenInput] = useState('')
  const [showTokenInput, setShowTokenInput] = useState(false)
  const [mutationError, setMutationError] = useState<string | null>(null)

  // Fetch current integration settings from backend (single source of truth)
  const { data: serverSettings, isLoading } = useIntegrations()

  // Mutations
  const updateIntegrations = useUpdateIntegrations({
    onSuccess: () => {
      // TanStack Query will automatically refetch and update serverSettings
      setTokenInput('')
      setShowTokenInput(false)
      setMutationError(null)
    },
    onError: (error) => {
      setMutationError(error.message || 'Failed to update settings')
    },
  })

  const testConnection = useTestListenbrainzConnection()

  // Extract stable mutate references for useCallback
  const { mutate: updateMutate } = updateIntegrations
  const { mutate: testMutate, reset: resetTest } = testConnection

  // Clear test result and error when token input changes
  const handleTokenChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setTokenInput(e.target.value)
      resetTest()
      setMutationError(null)
    },
    [resetTest]
  )

  // Handlers
  const handleTestConnection = useCallback(() => {
    if (tokenInput.trim()) {
      testMutate(tokenInput.trim())
    }
  }, [tokenInput, testMutate])

  const handleSaveToken = useCallback(() => {
    const token = tokenInput.trim()
    updateMutate({
      listenbrainzToken: token, // Send empty string to remove, or the new token
      listenbrainzEnabled: !!token, // Enable if token exists, disable if it's empty
    })
  }, [tokenInput, updateMutate])

  const handleRemoveToken = useCallback(() => {
    updateMutate({
      listenbrainzToken: '', // Empty string removes token
      listenbrainzEnabled: false,
    })
  }, [updateMutate])

  const handleListenbrainzToggle = useCallback(
    (enabled: boolean) => {
      setMutationError(null)
      // TanStack Query handles optimistic updates via invalidation
      updateMutate({ listenbrainzEnabled: enabled })
    },
    [updateMutate]
  )

  const handleDiscordToggle = useCallback(
    (enabled: boolean) => {
      setMutationError(null)
      // Sync Discord RPC preference to backend
      updateMutate({ discordRpcEnabled: enabled })
    },
    [updateMutate]
  )

  const hasToken = serverSettings?.hasListenbrainzToken ?? false
  const username = serverSettings?.listenbrainzUsername
  // Use server state as single source of truth
  const listenbrainzEnabled = serverSettings?.listenbrainzEnabled ?? false
  const discordRpcEnabled = serverSettings?.discordRpcEnabled ?? false

  return (
    <Card padding="lg">
      <CardHeader>
        <CardTitle>Integrations</CardTitle>
        <CardDescription>
          Connect to external services for enhanced features
        </CardDescription>
      </CardHeader>
      <CardContent className="mt-4 space-y-6">
        {/* Loading state */}
        {isLoading && (
          <div className="flex items-center gap-2 text-sm text-text-muted" role="status">
            <span className="animate-pulse">Loading integration settings...</span>
          </div>
        )}

        {/* Mutation error */}
        {mutationError && (
          <div className="text-sm text-error-text" role="alert">
            {mutationError}
          </div>
        )}

        {/* ListenBrainz */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium text-text-primary">ListenBrainz</p>
              <p className="text-sm text-text-muted">
                Scrobble your listening history to ListenBrainz
              </p>
            </div>
            <Switch
              id="listenbrainz-toggle"
              checked={listenbrainzEnabled}
              onCheckedChange={handleListenbrainzToggle}
              disabled={!hasToken || isLoading || updateIntegrations.isPending}
              aria-label="Enable ListenBrainz scrobbling"
            />
          </div>

          {/* Connection status */}
          {hasToken && username && (
            <div className="flex items-center gap-2 text-sm">
              <span className="h-2 w-2 rounded-full bg-success" aria-hidden="true" />
              <span className="text-text-secondary">
                Connected as <span className="font-medium text-text-primary">{username}</span>
              </span>
            </div>
          )}

          {/* Token management */}
          <div className="pl-4 border-l-2 border-background-tertiary space-y-3">
            {!hasToken && !showTokenInput && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowTokenInput(true)}
                disabled={isLoading}
              >
                Add ListenBrainz Token
              </Button>
            )}

            {(showTokenInput || (!hasToken && tokenInput)) && (
              <div className="space-y-3">
                <div className="flex gap-2">
                  <Input
                    id="listenbrainz-token-input"
                    type="password"
                    placeholder="Enter your ListenBrainz token"
                    value={tokenInput}
                    onChange={handleTokenChange}
                    className="flex-1"
                    aria-label="ListenBrainz token"
                  />
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={handleTestConnection}
                    disabled={!tokenInput.trim() || testConnection.isPending}
                  >
                    {testConnection.isPending ? 'Testing...' : 'Test Connection'}
                  </Button>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={handleSaveToken}
                    disabled={!tokenInput.trim() || updateIntegrations.isPending}
                  >
                    {updateIntegrations.isPending ? 'Saving...' : 'Save Token'}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      setShowTokenInput(false)
                      setTokenInput('')
                      resetTest()
                      setMutationError(null)
                    }}
                  >
                    Cancel
                  </Button>
                </div>
                {/* Test result */}
                {testConnection.data && (
                  <div
                    className={`text-sm ${
                      testConnection.data.valid ? 'text-success' : 'text-error-text'
                    }`}
                    role="status"
                  >
                    {testConnection.data.valid
                      ? `Valid token for user: ${testConnection.data.username}`
                      : testConnection.data.error || 'Invalid token'}
                  </div>
                )}
                {testConnection.isError && (
                  <div className="text-sm text-error-text" role="alert">
                    Connection test failed
                  </div>
                )}
                <p className="text-xs text-text-muted">
                  Get your token at{' '}
                  <a
                    href="https://listenbrainz.org/settings/"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-accent hover:underline"
                  >
                    listenbrainz.org/settings
                  </a>
                </p>
              </div>
            )}

            {hasToken && !showTokenInput && (
              <div className="flex gap-2">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setShowTokenInput(true)}
                >
                  Update Token
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-error-text hover:bg-error/20"
                  onClick={handleRemoveToken}
                  disabled={updateIntegrations.isPending}
                >
                  Remove Token
                </Button>
              </div>
            )}
          </div>
        </div>

        {/* Discord Rich Presence */}
        <div className="flex items-center justify-between pt-4 border-t border-background-tertiary">
          <div>
            <p className="font-medium text-text-primary">Discord Rich Presence</p>
            <p className="text-sm text-text-muted">
              Show what you're listening to on Discord
            </p>
            <p className="text-xs text-text-muted mt-1">
              Note: Only available in desktop app
            </p>
          </div>
          <Switch
            id="discord-rpc-toggle"
            checked={discordRpcEnabled}
            onCheckedChange={handleDiscordToggle}
            disabled={isLoading || updateIntegrations.isPending}
            aria-label="Enable Discord Rich Presence"
          />
        </div>
      </CardContent>
    </Card>
  )
}
