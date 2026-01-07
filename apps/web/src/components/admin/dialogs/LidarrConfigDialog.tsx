/**
 * LidarrConfigDialog - Dialog for configuring Lidarr integration
 */

import { useState, useEffect, useCallback } from 'react'
import { Music, CheckCircle, XCircle, Loader2 } from 'lucide-react'
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

interface LidarrConfig {
  url: string
}

interface LidarrConfigDialogProps {
  setting: SystemSettingInfo | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onSave: (data: {
    service: 'LIDARR'
    enabled: boolean
    config: string
    secret?: string
  }) => void
  onTestConnection: () => Promise<{ success: boolean; error?: string | null; version?: string | null }>
  isSaving: boolean
}

export function LidarrConfigDialog({
  setting,
  open,
  onOpenChange,
  onSave,
  onTestConnection,
  isSaving,
}: LidarrConfigDialogProps): JSX.Element | null {
  const [enabled, setEnabled] = useState(false)
  const [url, setUrl] = useState('')
  const [updateApiKey, setUpdateApiKey] = useState(false)
  const [apiKey, setApiKey] = useState('')
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
      const config = setting.config as Partial<LidarrConfig>
      setUrl(config.url ?? 'http://localhost:8686')
      setUpdateApiKey(false)
      setApiKey('')
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

    const config: LidarrConfig = {
      url: url.trim(),
    }

    onSave({
      service: 'LIDARR',
      enabled,
      config: JSON.stringify(config),
      // Only send the API key if the user opted to update it
      secret: updateApiKey ? apiKey : undefined,
    })
  }

  if (!setting) return null

  const hasExistingSecret = setting.hasSecret

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="rounded-lg bg-accent-dark/20 p-2">
              <Music className="h-5 w-5 text-accent" />
            </div>
            <div>
              <DialogTitle>Configure Lidarr</DialogTitle>
              <DialogDescription>
                Music collection manager for automatic library management
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
                Enable Lidarr integration
              </p>
            </div>
            <Switch checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {/* URL Field */}
          <div>
            <label
              htmlFor="lidarr-url"
              className="block text-sm font-medium text-text-primary mb-1"
            >
              URL <span className="text-error">*</span>
            </label>
            <Input
              id="lidarr-url"
              type="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="http://localhost:8686"
              required
            />
            <p className="text-xs text-text-tertiary mt-1">
              The base URL for your Lidarr instance
            </p>
          </div>

          {/* API Key Section */}
          <div className="space-y-3">
            {hasExistingSecret && (
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={updateApiKey}
                  onChange={(e) => {
                    setUpdateApiKey(e.target.checked)
                    if (!e.target.checked) {
                      setApiKey('')
                    }
                  }}
                  className="rounded border-background-tertiary bg-background-secondary text-accent focus:ring-accent-glow"
                />
                <span className="text-sm text-text-secondary">
                  Update API Key
                </span>
              </label>
            )}

            {(!hasExistingSecret || updateApiKey) && (
              <div>
                <label
                  htmlFor="lidarr-api-key"
                  className="block text-sm font-medium text-text-primary mb-1"
                >
                  API Key {!hasExistingSecret && <span className="text-error">*</span>}
                </label>
                <Input
                  id="lidarr-api-key"
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder="Enter API key"
                  required={!hasExistingSecret}
                />
                <p className="text-xs text-text-tertiary mt-1">
                  Find this in Lidarr under Settings → General → API Key
                </p>
              </div>
            )}

            {hasExistingSecret && !updateApiKey && (
              <p className="text-xs text-text-tertiary flex items-center gap-1">
                <CheckCircle className="h-3 w-3 text-mint" />
                API Key is configured
              </p>
            )}
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
