/**
 * OllamaStep component for Resonance Setup Wizard
 *
 * Step to configure Ollama AI integration.
 */

import { useState, type FormEvent } from 'react'
import { Loader2, CheckCircle2, XCircle } from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'
import { Input } from '../ui/Input'
import { useUpdateSystemSetting, useTestServiceConnection } from '../../hooks/useSetup'

interface OllamaStepProps {
  /** Callback when step is complete */
  onNext: () => void
  /** Callback to go back to previous step */
  onBack: () => void
}

/**
 * Ollama configuration step of the setup wizard.
 */
export function OllamaStep({ onNext, onBack }: OllamaStepProps): JSX.Element {
  const [ollamaUrl, setOllamaUrl] = useState('http://ollama:11434')
  const [model, setModel] = useState('mistral')
  const [testResult, setTestResult] = useState<{
    success: boolean
    message: string
    version?: string | null
  } | null>(null)

  const updateSetting = useUpdateSystemSetting()
  const testConnection = useTestServiceConnection()

  async function handleTest(): Promise<void> {
    setTestResult(null)

    // First save the config
    try {
      await updateSetting.mutateAsync({
        service: 'OLLAMA',
        enabled: true,
        config: JSON.stringify({ url: ollamaUrl, model }),
      })

      // Then test the connection
      const result = await testConnection.mutateAsync('OLLAMA')

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

    try {
      await updateSetting.mutateAsync({
        service: 'OLLAMA',
        enabled: true,
        config: JSON.stringify({ url: ollamaUrl, model }),
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
            AI Configuration
          </h1>
          <p className="mt-2 text-text-secondary">
            Connect to Ollama for AI-powered recommendations and natural language search.
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

          {/* Ollama URL */}
          <div>
            <label
              htmlFor="ollamaUrl"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Ollama URL
            </label>
            <Input
              id="ollamaUrl"
              type="url"
              value={ollamaUrl}
              onChange={(e) => setOllamaUrl(e.target.value)}
              placeholder="http://ollama:11434"
              disabled={isLoading}
            />
            <p className="mt-1 text-xs text-text-muted">
              The URL where your Ollama instance is running
            </p>
          </div>

          {/* Model */}
          <div>
            <label
              htmlFor="model"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Model
            </label>
            <Input
              id="model"
              type="text"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="mistral"
              disabled={isLoading}
            />
            <p className="mt-1 text-xs text-text-muted">
              The Ollama model to use (e.g., mistral, llama2, codellama)
            </p>
          </div>

          {/* Test Connection */}
          <div>
            <Button
              type="button"
              variant="secondary"
              onClick={handleTest}
              disabled={isLoading}
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
                      Version: {testResult.version}
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
              disabled={isLoading}
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
            <strong className="text-text-secondary">Optional:</strong> Ollama enables AI features
            like smart recommendations and natural language search. You can configure this later
            in the admin settings.
          </p>
        </div>
      </div>
    </Card>
  )
}

export default OllamaStep
