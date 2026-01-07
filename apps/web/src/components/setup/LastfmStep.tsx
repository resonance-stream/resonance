/**
 * LastfmStep component for Resonance Setup Wizard
 *
 * Step to configure Last.fm integration.
 */

import { useState, type FormEvent } from 'react'
import { Loader2, CheckCircle2, XCircle } from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'
import { Input } from '../ui/Input'
import { useUpdateSystemSetting, useTestServiceConnection } from '../../hooks/useSetup'

interface LastfmStepProps {
  /** Callback when step is complete */
  onNext: () => void
  /** Callback to go back to previous step */
  onBack: () => void
}

/**
 * Last.fm configuration step of the setup wizard.
 */
export function LastfmStep({ onNext, onBack }: LastfmStepProps): JSX.Element {
  const [apiKey, setApiKey] = useState('')
  const [sharedSecret, setSharedSecret] = useState('')
  const [testResult, setTestResult] = useState<{
    success: boolean
    message: string
  } | null>(null)

  const updateSetting = useUpdateSystemSetting()
  const testConnection = useTestServiceConnection()

  async function handleTest(): Promise<void> {
    setTestResult(null)

    if (!apiKey) {
      setTestResult({
        success: false,
        message: 'Please enter an API key',
      })
      return
    }

    try {
      await updateSetting.mutateAsync({
        service: 'LASTFM',
        enabled: true,
        config: JSON.stringify({}),
        secret: JSON.stringify({ apiKey, sharedSecret: sharedSecret || undefined }),
      })

      const result = await testConnection.mutateAsync('LASTFM')

      if (result.success) {
        setTestResult({
          success: true,
          message: `Connected successfully${result.responseTimeMs ? ` (${result.responseTimeMs}ms)` : ''}`,
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

    if (!apiKey) {
      return
    }

    try {
      await updateSetting.mutateAsync({
        service: 'LASTFM',
        enabled: true,
        config: JSON.stringify({}),
        secret: JSON.stringify({ apiKey, sharedSecret: sharedSecret || undefined }),
      })
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
            Last.fm Integration
          </h1>
          <p className="mt-2 text-text-secondary">
            Connect to Last.fm for scrobbling and music discovery features.
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
              type="text"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="Your Last.fm API key"
              disabled={isLoading}
            />
            <p className="mt-1 text-xs text-text-muted">
              Get your API key at{' '}
              <a
                href="https://www.last.fm/api/account/create"
                target="_blank"
                rel="noopener noreferrer"
                className="text-accent-light hover:underline"
              >
                last.fm/api
              </a>
            </p>
          </div>

          {/* Shared Secret */}
          <div>
            <label
              htmlFor="sharedSecret"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Shared Secret <span className="text-text-muted">(optional)</span>
            </label>
            <Input
              id="sharedSecret"
              type="password"
              value={sharedSecret}
              onChange={(e) => setSharedSecret(e.target.value)}
              placeholder="Your Last.fm shared secret"
              disabled={isLoading}
            />
            <p className="mt-1 text-xs text-text-muted">
              Required for scrobbling. Found on your API account page.
            </p>
          </div>

          {/* Test Connection */}
          <div>
            <Button
              type="button"
              variant="secondary"
              onClick={handleTest}
              disabled={isLoading || !apiKey}
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
                <p className={`text-sm ${testResult.success ? 'text-accent-light' : 'text-error-text'}`}>
                  {testResult.message}
                </p>
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
              disabled={isLoading || !apiKey}
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
            <strong className="text-text-secondary">Optional:</strong> Last.fm enables scrobbling
            (play history tracking) and similar artist recommendations. You can configure this later
            in the admin settings. Users can also add their own Last.fm credentials in their profile.
          </p>
        </div>
      </div>
    </Card>
  )
}

export default LastfmStep
