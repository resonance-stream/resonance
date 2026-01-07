/**
 * OllamaStep component for Resonance Setup Wizard
 *
 * Step to configure Ollama AI integration with URL input,
 * model selector, embedding model selector, and connection testing.
 */

import { useState, useEffect, useCallback, type FormEvent } from 'react'
import { Loader2, CheckCircle2, XCircle, RefreshCw, ChevronDown } from 'lucide-react'
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

/** Model information returned from Ollama API */
interface OllamaModel {
  name: string
  modified_at: string
  size: number
}

/** Response from Ollama /api/tags endpoint */
interface OllamaTagsResponse {
  models: OllamaModel[]
}

/** Default Ollama configuration values */
const DEFAULT_URL = 'http://localhost:11434'
const DEFAULT_MODEL = 'mistral'
const DEFAULT_EMBEDDING_MODEL = 'nomic-embed-text'

/** Timeout for fetching models from Ollama (5 seconds) */
const FETCH_TIMEOUT_MS = 5000

/**
 * Fetches available models from an Ollama instance
 */
async function fetchOllamaModels(url: string): Promise<string[]> {
  const tagsUrl = `${url.replace(/\/$/, '')}/api/tags`

  // Create AbortController with timeout
  const controller = new AbortController()
  const timeoutId = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS)

  try {
    const response = await fetch(tagsUrl, {
      method: 'GET',
      headers: { 'Accept': 'application/json' },
      signal: controller.signal,
    })

    if (!response.ok) {
      throw new Error(`Failed to fetch models: ${response.status}`)
    }

    const data: OllamaTagsResponse = await response.json()
    return data.models.map((m) => m.name)
  } catch (err) {
    if (err instanceof Error && err.name === 'AbortError') {
      throw new Error(`Request timed out after ${FETCH_TIMEOUT_MS / 1000} seconds`)
    }
    throw err
  } finally {
    clearTimeout(timeoutId)
  }
}

/**
 * Ollama configuration step of the setup wizard.
 */
export function OllamaStep({ onNext, onBack }: OllamaStepProps): JSX.Element {
  const [ollamaUrl, setOllamaUrl] = useState(DEFAULT_URL)
  const [model, setModel] = useState(DEFAULT_MODEL)
  const [embeddingModel, setEmbeddingModel] = useState(DEFAULT_EMBEDDING_MODEL)
  const [availableModels, setAvailableModels] = useState<string[]>([])
  const [isFetchingModels, setIsFetchingModels] = useState(false)
  const [modelsFetchError, setModelsFetchError] = useState<string | null>(null)
  const [testResult, setTestResult] = useState<{
    success: boolean
    message: string
    version?: string | null
  } | null>(null)

  const updateSetting = useUpdateSystemSetting()
  const testConnection = useTestServiceConnection()

  /**
   * Attempts to fetch models from the Ollama instance
   */
  const handleFetchModels = useCallback(async () => {
    if (!ollamaUrl) return

    setIsFetchingModels(true)
    setModelsFetchError(null)

    try {
      const models = await fetchOllamaModels(ollamaUrl)
      setAvailableModels(models)

      // If current model isn't in the list and we have models, suggest first one
      if (models.length > 0 && !models.includes(model)) {
        // Keep current model but show it's not found
      }
    } catch (err) {
      setModelsFetchError((err as Error)?.message || 'Failed to fetch models')
      setAvailableModels([])
    } finally {
      setIsFetchingModels(false)
    }
  }, [ollamaUrl, model])

  // Fetch models when URL changes (debounced)
  useEffect(() => {
    const timer = setTimeout(() => {
      if (ollamaUrl) {
        handleFetchModels()
      }
    }, 500)

    return () => clearTimeout(timer)
  }, [ollamaUrl, handleFetchModels])

  async function handleTest(): Promise<void> {
    setTestResult(null)

    // First save the config
    try {
      await updateSetting.mutateAsync({
        service: 'OLLAMA',
        enabled: true,
        config: JSON.stringify({
          url: ollamaUrl,
          model,
          embedding_model: embeddingModel,
        }),
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
        config: JSON.stringify({
          url: ollamaUrl,
          model,
          embedding_model: embeddingModel,
        }),
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

  // Check if selected models are available
  const modelNotAvailable = availableModels.length > 0 && !availableModels.includes(model)
  const embeddingModelNotAvailable = availableModels.length > 0 && !availableModels.includes(embeddingModel)

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
              placeholder={DEFAULT_URL}
              disabled={isLoading}
            />
            <p className="mt-1 text-xs text-text-muted">
              The URL where your Ollama instance is running
            </p>
          </div>

          {/* Model Selector */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <label
                htmlFor="model"
                className="block text-sm font-medium text-text-secondary"
              >
                Chat Model
              </label>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={handleFetchModels}
                disabled={isFetchingModels || !ollamaUrl}
                className="h-6 px-2 text-xs"
              >
                {isFetchingModels ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <RefreshCw className="h-3 w-3" />
                )}
                <span className="ml-1">Refresh</span>
              </Button>
            </div>

            {availableModels.length > 0 ? (
              <div className="relative">
                <select
                  id="model"
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  disabled={isLoading}
                  className="w-full appearance-none rounded-lg bg-background-secondary border border-white/10 px-4 py-2.5 text-text-primary focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50"
                >
                  {!availableModels.includes(model) && (
                    <option value={model}>{model} (not installed)</option>
                  )}
                  {availableModels.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </select>
                <ChevronDown className="absolute right-3 top-1/2 -translate-y-1/2 h-4 w-4 text-text-muted pointer-events-none" />
              </div>
            ) : (
              <Input
                id="model"
                type="text"
                value={model}
                onChange={(e) => setModel(e.target.value)}
                placeholder={DEFAULT_MODEL}
                disabled={isLoading}
              />
            )}

            <p className="mt-1 text-xs text-text-muted">
              The LLM model for chat and recommendations (e.g., mistral, llama2, codellama)
            </p>

            {modelNotAvailable && (
              <p className="mt-1 text-xs text-warning">
                Model "{model}" is not installed. Pull it with: ollama pull {model}
              </p>
            )}

            {modelsFetchError && (
              <p className="mt-1 text-xs text-text-muted">
                Could not fetch models: {modelsFetchError}
              </p>
            )}
          </div>

          {/* Embedding Model Selector */}
          <div>
            <label
              htmlFor="embeddingModel"
              className="mb-2 block text-sm font-medium text-text-secondary"
            >
              Embedding Model
            </label>

            {availableModels.length > 0 ? (
              <div className="relative">
                <select
                  id="embeddingModel"
                  value={embeddingModel}
                  onChange={(e) => setEmbeddingModel(e.target.value)}
                  disabled={isLoading}
                  className="w-full appearance-none rounded-lg bg-background-secondary border border-white/10 px-4 py-2.5 text-text-primary focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50"
                >
                  {!availableModels.includes(embeddingModel) && (
                    <option value={embeddingModel}>{embeddingModel} (not installed)</option>
                  )}
                  {availableModels.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </select>
                <ChevronDown className="absolute right-3 top-1/2 -translate-y-1/2 h-4 w-4 text-text-muted pointer-events-none" />
              </div>
            ) : (
              <Input
                id="embeddingModel"
                type="text"
                value={embeddingModel}
                onChange={(e) => setEmbeddingModel(e.target.value)}
                placeholder={DEFAULT_EMBEDDING_MODEL}
                disabled={isLoading}
              />
            )}

            <p className="mt-1 text-xs text-text-muted">
              The embedding model for vector search (e.g., nomic-embed-text, mxbai-embed-large)
            </p>

            {embeddingModelNotAvailable && (
              <p className="mt-1 text-xs text-warning">
                Model "{embeddingModel}" is not installed. Pull it with: ollama pull {embeddingModel}
              </p>
            )}
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
