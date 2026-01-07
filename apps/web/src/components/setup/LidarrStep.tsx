/**
 * LidarrStep component for Resonance Setup Wizard
 *
 * Step to configure Lidarr integration.
 */

import { useState, type FormEvent } from 'react'
import { Loader2, CheckCircle2, XCircle } from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'
import { Input } from '../ui/Input'
import { useUpdateSystemSetting, useTestServiceConnection } from '../../hooks/useSetup'

interface LidarrStepProps {
  /** Callback when step is complete */
  onNext: () => void
  /** Callback to go back to previous step */
  onBack: () => void
}

/**
 * Lidarr configuration step of the setup wizard.
 */
export function LidarrStep({ onNext, onBack }: LidarrStepProps): JSX.Element {
  const [lidarrUrl, setLidarrUrl] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [testResult, setTestResult] = useState<{
    success: boolean
    message: string
    version?: string | null
  } | null>(null)
  // Track saved values to avoid duplicate save on submit after test
  const [savedConfig, setSavedConfig] = useState<{ url: string; apiKey: string } | null>(null)

  const updateSetting = useUpdateSystemSetting()
  const testConnection = useTestServiceConnection()

  async function handleTest(): Promise<void> {
    setTestResult(null)

    if (!lidarrUrl || !apiKey) {
      setTestResult({
        success: false,
        message: 'Please enter both URL and API key',
      })
      return
    }

    try {
      await updateSetting.mutateAsync({
        service: 'LIDARR',
        enabled: true,
        config: JSON.stringify({ url: lidarrUrl }),
        secret: apiKey,
      })
      // Track saved config to avoid duplicate save on submit
      setSavedConfig({ url: lidarrUrl, apiKey })

      const result = await testConnection.mutateAsync('LIDARR')

      if (result.success) {
        setTestResult({
          success: true,
          message: `Connected successfully${result.responseTimeMs ? ` (${result.responseTimeMs}ms)` : ''}`,
          version: result.version,
        })
      } else {
        setTestResult({
          success: false,
          message: result.error || 'Connection failed',
        })
      }
    } catch (err) {
      setTestResult({
        success: false,
        message: (err as Error)?.message || 'Failed to test connection',
      })
    }
  }

  async function handleSubmit(e: FormEvent<HTMLFormElement>): Promise<void> {
    e.preventDefault()

    if (!lidarrUrl || !apiKey) {
      return
    }

    try {
      // Skip save if config hasn't changed since last test (avoid duplicate API call)
      const configChanged = !savedConfig ||
        savedConfig.url !== lidarrUrl ||
        savedConfig.apiKey !== apiKey

      if (configChanged) {
        await updateSetting.mutateAsync({
          service: 'LIDARR',
          enabled: true,
          config: JSON.stringify({ url: lidarrUrl }),
          secret: apiKey,
        })
      }
      onNext()
    } catch {
      // Error handled by mutation
    }
  }

  function handleSkip(): void {
    onNext()
  }

  const isLoading = updateSetting.isPending || testConnection.isPending

  return (
    <Card variant="glass" padding="lg">
      <div className="space-y-6">
        {/* Header */}
        <div className="text-center">
          <h1 className="font-display text-2xl text-text-primary">
            Lidarr Integration
          </h1>
          <p className="mt-2 text-text-secondary">
            Connect to Lidarr for automatic music library management and downloads.
          </p>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Error message */}
          {updateSetting.error && (
            <div className="rounded-lg bg-error/20 border border-error/30 p-4 text-error-text">
              <p className="text-sm">{(updateSetting.error as Error)?.message || 'An error occurred'}</p>
            </div>
          )}

          {/* Lidarr URL */}
          <div>
            <label
              htmlFor="lidarrUrl"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Lidarr URL
            </label>
            <Input
              id="lidarrUrl"
              type="url"
              value={lidarrUrl}
              onChange={(e) => setLidarrUrl(e.target.value)}
              placeholder="http://lidarr:8686"
              disabled={isLoading}
            />
            <p className="mt-1 text-xs text-text-muted">
              The URL where your Lidarr instance is running
            </p>
          </div>

          {/* API Key */}
          <div>
            <label
              htmlFor="apiKey"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              API Key
            </label>
            <Input
              id="apiKey"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="Your Lidarr API key"
              disabled={isLoading}
            />
            <p className="mt-1 text-xs text-text-muted">
              Found in Lidarr under Settings → General → Security
            </p>
          </div>

          {/* Test Connection */}
          <div>
            <Button
              type="button"
              variant="secondary"
              onClick={handleTest}
              disabled={isLoading || !lidarrUrl || !apiKey}
              className="w-full"
            >
              {testConnection.isPending ? (
                <span className="flex items-center justify-center gap-2">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Testing...
                </span>
              ) : (
                'Test Connection'
              )}
            </Button>

            {/* Test Result */}
            {testResult && (
              <div
                className={`mt-3 rounded-lg p-3 flex items-start gap-3 ${
                  testResult.success
                    ? 'bg-accent-light/10 border border-accent-light/20'
                    : 'bg-error/10 border border-error/20'
                }`}
              >
                {testResult.success ? (
                  <CheckCircle2 className="w-5 h-5 text-accent-light flex-shrink-0 mt-0.5" />
                ) : (
                  <XCircle className="w-5 h-5 text-error flex-shrink-0 mt-0.5" />
                )}
                <div>
                  <p className={`text-sm ${testResult.success ? 'text-accent-light' : 'text-error-text'}`}>
                    {testResult.message}
                  </p>
                  {testResult.version && (
                    <p className="text-xs text-text-muted mt-1">
                      Lidarr v{testResult.version}
                    </p>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* Actions */}
          <div className="flex gap-3 pt-2">
            <Button
              type="button"
              variant="secondary"
              onClick={onBack}
              disabled={isLoading}
              className="flex-1"
            >
              Back
            </Button>
            <Button
              type="button"
              variant="ghost"
              onClick={handleSkip}
              disabled={isLoading}
            >
              Skip
            </Button>
            <Button
              type="submit"
              disabled={isLoading || !lidarrUrl || !apiKey}
              className="flex-1"
            >
              {updateSetting.isPending ? (
                <span className="flex items-center justify-center gap-2">
                  <Loader2 className="h-5 w-5 animate-spin" />
                  Saving...
                </span>
              ) : (
                'Continue'
              )}
            </Button>
          </div>
        </form>

        {/* Info box */}
        <div className="rounded-lg bg-background-tertiary/50 border border-white/5 p-4">
          <p className="text-sm text-text-muted">
            <strong className="text-text-secondary">Optional:</strong> Lidarr integration enables
            automatic library management, missing album detection, and music downloads. You can
            configure this later in the admin settings.
          </p>
        </div>
      </div>
    </Card>
  )
}

export default LidarrStep
