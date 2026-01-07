/**
 * OllamaConfigDialog - Dialog for configuring Ollama AI service
 */

import { useState, useEffect, useCallback } from 'react'
import { Brain, CheckCircle, XCircle, Loader2 } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../../ui/Dialog'
import { Button } from '../../ui/Button'
import { Input } from '../../ui/Input'
import { Switch } from '../../ui/Switch'
import type { SystemSettingInfo } from '../../../types/systemSettings'

interface OllamaConfig {
  url: string
  model: string
  embeddingModel: string
}

interface OllamaConfigDialogProps {
  setting: SystemSettingInfo | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onSave: (data: {
    service: 'OLLAMA'
    enabled: boolean
    config: string
  }) => void
  onTestConnection: () => Promise<{ success: boolean; error?: string | null; version?: string | null }>
  isSaving: boolean
}

export function OllamaConfigDialog({
  setting,
  open,
  onOpenChange,
  onSave,
  onTestConnection,
  isSaving,
}: OllamaConfigDialogProps): JSX.Element | null {
  const [enabled, setEnabled] = useState(false)
  const [url, setUrl] = useState('')
  const [model, setModel] = useState('')
  const [embeddingModel, setEmbeddingModel] = useState('')
  const [isTesting, setIsTesting] = useState(false)
  const [testResult, setTestResult] = useState<{
    success: boolean
    error?: string | null
    version?: string | null
  } | null>(null)

  // Reset form when dialog opens with new setting
  useEffect(() => {
    if (setting && open) {
      setEnabled(setting.enabled)
      const config = setting.config as Partial<OllamaConfig>
      setUrl(config.url ?? 'http://localhost:11434')
      setModel(config.model ?? 'mistral')
      setEmbeddingModel(config.embeddingModel ?? 'nomic-embed-text')
      setTestResult(null)
    }
  }, [setting, open])

  const handleTestConnection = useCallback(async () => {
    setIsTesting(true)
    setTestResult(null)
    try {
      const result = await onTestConnection()
      setTestResult(result)
    } catch (error) {
      setTestResult({
        success: false,
        error: error instanceof Error ? error.message : 'Connection test failed',
      })
    } finally {
      setIsTesting(false)
    }
  }, [onTestConnection])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    const config: OllamaConfig = {
      url: url.trim(),
      model: model.trim() || 'mistral',
      embeddingModel: embeddingModel.trim() || 'nomic-embed-text',
    }

    onSave({
      service: 'OLLAMA',
      enabled,
      config: JSON.stringify(config),
    })
  }

  if (!setting) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="rounded-lg bg-accent-dark/20 p-2">
              <Brain className="h-5 w-5 text-accent" />
            </div>
            <div>
              <DialogTitle>Configure Ollama</DialogTitle>
              <DialogDescription>
                Local AI inference for recommendations and natural language search
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-6 mt-4">
          {/* Enabled Toggle */}
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-text-primary">Enabled</p>
              <p className="text-xs text-text-secondary">
                Enable AI-powered features
              </p>
            </div>
            <Switch checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {/* URL Field */}
          <div>
            <label
              htmlFor="ollama-url"
              className="block text-sm font-medium text-text-primary mb-1"
            >
              URL <span className="text-error">*</span>
            </label>
            <Input
              id="ollama-url"
              type="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="http://localhost:11434"
              required
            />
            <p className="text-xs text-text-tertiary mt-1">
              The base URL for your Ollama instance
            </p>
          </div>

          {/* Model Field */}
          <div>
            <label
              htmlFor="ollama-model"
              className="block text-sm font-medium text-text-primary mb-1"
            >
              Chat Model
            </label>
            <Input
              id="ollama-model"
              type="text"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="mistral"
            />
            <p className="text-xs text-text-tertiary mt-1">
              Model for natural language search (e.g., mistral, llama2, phi)
            </p>
          </div>

          {/* Embedding Model Field */}
          <div>
            <label
              htmlFor="ollama-embedding-model"
              className="block text-sm font-medium text-text-primary mb-1"
            >
              Embedding Model
            </label>
            <Input
              id="ollama-embedding-model"
              type="text"
              value={embeddingModel}
              onChange={(e) => setEmbeddingModel(e.target.value)}
              placeholder="nomic-embed-text"
            />
            <p className="text-xs text-text-tertiary mt-1">
              Model for generating embeddings (e.g., nomic-embed-text, mxbai-embed-large)
            </p>
          </div>

          {/* Test Connection */}
          <div className="rounded-lg bg-background-tertiary/50 p-4">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-medium text-text-primary">Connection Test</span>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                onClick={handleTestConnection}
                disabled={isTesting || !url.trim()}
              >
                {isTesting ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    Testing...
                  </>
                ) : (
                  'Test Connection'
                )}
              </Button>
            </div>
            {testResult && (
              <div
                className={`flex items-center gap-2 mt-2 ${
                  testResult.success ? 'text-mint' : 'text-error'
                }`}
              >
                {testResult.success ? (
                  <>
                    <CheckCircle className="h-4 w-4" />
                    <span className="text-sm">
                      Connected{testResult.version ? ` (v${testResult.version})` : ''}
                    </span>
                  </>
                ) : (
                  <>
                    <XCircle className="h-4 w-4" />
                    <span className="text-sm">{testResult.error || 'Connection failed'}</span>
                  </>
                )}
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={() => onOpenChange(false)}
              disabled={isSaving}
            >
              Cancel
            </Button>
            <Button type="submit" variant="accent" disabled={isSaving}>
              {isSaving ? 'Saving...' : 'Save Changes'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
