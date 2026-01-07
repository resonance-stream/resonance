/**
 * LastFmConfigDialog - Dialog for configuring Last.fm integration
 */

import { useState, useEffect } from 'react'
import { Radio, CheckCircle } from 'lucide-react'
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

interface LastFmConfigDialogProps {
  setting: SystemSettingInfo | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onSave: (data: {
    service: 'LASTFM'
    enabled: boolean
    config: string
    secret?: string
  }) => void
  isSaving: boolean
}

export function LastFmConfigDialog({
  setting,
  open,
  onOpenChange,
  onSave,
  isSaving,
}: LastFmConfigDialogProps): JSX.Element | null {
  const [enabled, setEnabled] = useState(false)
  const [updateApiKey, setUpdateApiKey] = useState(false)
  const [apiKey, setApiKey] = useState('')
  const [sharedSecret, setSharedSecret] = useState('')

  // Reset form when dialog opens with new setting
  useEffect(() => {
    if (setting && open) {
      setEnabled(setting.enabled)
      setUpdateApiKey(false)
      setApiKey('')
      setSharedSecret('')
    }
  }, [setting, open])

  if (!setting) return null

  const hasExistingSecret = setting.hasSecret

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    // Build secret object with apiKey and optional sharedSecret
    let secretValue: string | undefined
    if (updateApiKey || !hasExistingSecret) {
      secretValue = JSON.stringify({
        apiKey,
        ...(sharedSecret && { sharedSecret }),
      })
    }

    onSave({
      service: 'LASTFM',
      enabled,
      config: JSON.stringify({}),
      secret: secretValue,
    })
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="rounded-lg bg-accent-dark/20 p-2">
              <Radio className="h-5 w-5 text-accent" />
            </div>
            <div>
              <DialogTitle>Configure Last.fm</DialogTitle>
              <DialogDescription>
                Scrobbling and music discovery integration
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
                Enable Last.fm integration for scrobbling
              </p>
            </div>
            <Switch checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {/* Info about Last.fm */}
          <div className="rounded-lg bg-background-tertiary/50 p-4">
            <p className="text-sm text-text-secondary">
              Last.fm integration enables:
            </p>
            <ul className="mt-2 space-y-1 text-sm text-text-tertiary">
              <li className="flex items-center gap-2">
                <span className="h-1 w-1 rounded-full bg-accent" />
                Track scrobbling to your Last.fm profile
              </li>
              <li className="flex items-center gap-2">
                <span className="h-1 w-1 rounded-full bg-accent" />
                Artist and album information enrichment
              </li>
              <li className="flex items-center gap-2">
                <span className="h-1 w-1 rounded-full bg-accent" />
                Similar artist recommendations
              </li>
            </ul>
          </div>

          {/* Credentials Section */}
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
                      setSharedSecret('')
                    }
                  }}
                  className="rounded border-background-tertiary bg-background-secondary text-accent focus:ring-accent-glow"
                />
                <span className="text-sm text-text-secondary">
                  Update Credentials
                </span>
              </label>
            )}

            {(!hasExistingSecret || updateApiKey) && (
              <>
                <div>
                  <label
                    htmlFor="lastfm-api-key"
                    className="block text-sm font-medium text-text-primary mb-1"
                  >
                    API Key {!hasExistingSecret && <span className="text-error">*</span>}
                  </label>
                  <Input
                    id="lastfm-api-key"
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    placeholder="Enter API key"
                    required={!hasExistingSecret}
                  />
                  <p className="text-xs text-text-tertiary mt-1">
                    Create an API account at{' '}
                    <a
                      href="https://www.last.fm/api/account/create"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-accent hover:underline"
                    >
                      last.fm/api
                    </a>
                  </p>
                </div>

                <div>
                  <label
                    htmlFor="lastfm-shared-secret"
                    className="block text-sm font-medium text-text-primary mb-1"
                  >
                    Shared Secret <span className="text-text-tertiary">(optional)</span>
                  </label>
                  <Input
                    id="lastfm-shared-secret"
                    type="password"
                    value={sharedSecret}
                    onChange={(e) => setSharedSecret(e.target.value)}
                    placeholder="Enter shared secret"
                  />
                  <p className="text-xs text-text-tertiary mt-1">
                    Required for scrobbling. Found on your API account page.
                  </p>
                </div>
              </>
            )}

            {hasExistingSecret && !updateApiKey && (
              <p className="text-xs text-text-tertiary flex items-center gap-1">
                <CheckCircle className="h-3 w-3 text-mint" />
                Credentials are configured
              </p>
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
