/**
 * ServiceConfigDialog - Dialog for configuring a service
 */

import { useState, useEffect } from 'react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '../ui/Dialog'
import { Button } from '../ui/Button'
import { Input } from '../ui/Input'
import { Switch } from '../ui/Switch'
import type { SystemSettingInfo, ServiceType, ServiceMetadata } from '../../types/systemSettings'
import { SERVICE_METADATA } from '../../types/systemSettings'

interface ServiceConfigDialogProps {
  setting: SystemSettingInfo | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onSave: (data: {
    service: ServiceType
    enabled: boolean
    config: string
    secret?: string
  }) => void
  isSaving: boolean
}

export function ServiceConfigDialog({
  setting,
  open,
  onOpenChange,
  onSave,
  isSaving,
}: ServiceConfigDialogProps) {
  const [enabled, setEnabled] = useState(false)
  const [configValues, setConfigValues] = useState<Record<string, string>>({})
  const [secret, setSecret] = useState('')

  // Reset form when dialog opens with new setting
  useEffect(() => {
    if (setting && open) {
      setEnabled(setting.enabled)
      const config = setting.config as Record<string, unknown>
      const values: Record<string, string> = {}
      Object.entries(config).forEach(([key, value]) => {
        values[key] = String(value ?? '')
      })
      setConfigValues(values)
      setSecret('') // Don't pre-fill secrets
    }
  }, [setting, open])

  if (!setting) return null

  const metadata: ServiceMetadata | undefined = SERVICE_METADATA[setting.service as ServiceType]
  const configFields = metadata?.configFields ?? []

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    onSave({
      service: setting.service as ServiceType,
      enabled,
      config: JSON.stringify(configValues),
      secret: secret || undefined,
    })
  }

  const handleConfigChange = (key: string, value: string) => {
    setConfigValues((prev) => ({ ...prev, [key]: value }))
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Configure {metadata?.name ?? setting.service}</DialogTitle>
          <DialogDescription>
            {metadata?.description ?? 'Configure this external service.'}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-6 mt-4">
          {/* Enabled Toggle */}
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-text-primary">Enabled</p>
              <p className="text-xs text-text-secondary">
                Enable or disable this service
              </p>
            </div>
            <Switch
              checked={enabled}
              onCheckedChange={setEnabled}
            />
          </div>

          {/* Config Fields */}
          {configFields.map((field) => (
            <div key={field.key}>
              <label
                htmlFor={`config-${field.key}`}
                className="block text-sm font-medium text-text-primary mb-1"
              >
                {field.label}
                {field.required && <span className="text-error ml-1">*</span>}
              </label>
              <Input
                id={`config-${field.key}`}
                type={field.type === 'url' ? 'url' : 'text'}
                value={configValues[field.key] ?? ''}
                onChange={(e) => handleConfigChange(field.key, e.target.value)}
                placeholder={field.placeholder}
                required={field.required}
              />
            </div>
          ))}

          {/* Secret Field */}
          {metadata?.hasSecret && (
            <div>
              <label
                htmlFor="secret"
                className="block text-sm font-medium text-text-primary mb-1"
              >
                {metadata.secretLabel ?? 'Secret'}
              </label>
              <Input
                id="secret"
                type="password"
                value={secret}
                onChange={(e) => setSecret(e.target.value)}
                placeholder={setting.hasSecret ? '(unchanged)' : 'Enter secret value'}
              />
              <p className="text-xs text-text-tertiary mt-1">
                {setting.hasSecret
                  ? 'Leave blank to keep the existing value'
                  : 'Required for this service to function'}
              </p>
            </div>
          )}

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
