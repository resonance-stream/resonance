/**
 * ServiceConfigCard - Card showing service name, status, and configuration options
 */

import {
  Brain,
  Music,
  Radio,
  Search,
  Folder,
  CheckCircle,
  XCircle,
  Circle,
  Settings,
  RefreshCw,
  Key,
} from 'lucide-react'
import { Card } from '../ui/Card'
import { Button } from '../ui/Button'
import { Badge } from '../ui/Badge'
import type { SystemSettingInfo, ServiceType } from '../../types/systemSettings'
import { SERVICE_METADATA } from '../../types/systemSettings'

interface ServiceConfigCardProps {
  setting: SystemSettingInfo
  isTestingConnection: boolean
  onTestConnection: () => void
  onConfigure: () => void
}

// Map service types to icons
const SERVICE_ICONS: Record<ServiceType, React.ComponentType<{ className?: string }>> = {
  OLLAMA: Brain,
  LIDARR: Music,
  LASTFM: Radio,
  MEILISEARCH: Search,
  MUSIC_LIBRARY: Folder,
}

function getStatusBadge(setting: SystemSettingInfo) {
  if (!setting.enabled) {
    return <Badge variant="default" size="sm">Disabled</Badge>
  }
  if (setting.connectionHealthy === null) {
    return <Badge variant="info" size="sm">Not Tested</Badge>
  }
  if (setting.connectionHealthy) {
    return <Badge variant="success" size="sm">Connected</Badge>
  }
  return <Badge variant="error" size="sm">Error</Badge>
}

function getStatusIcon(setting: SystemSettingInfo) {
  if (!setting.enabled) {
    return <Circle className="h-5 w-5 text-text-muted" />
  }
  if (setting.connectionHealthy === null) {
    return <Circle className="h-5 w-5 text-text-muted" />
  }
  if (setting.connectionHealthy) {
    return <CheckCircle className="h-5 w-5 text-mint" />
  }
  return <XCircle className="h-5 w-5 text-error" />
}

function getConfigSummary(setting: SystemSettingInfo): string {
  const config = setting.config as Record<string, unknown>

  // For MUSIC_LIBRARY, show the path
  if (setting.service === 'MUSIC_LIBRARY' && config.path) {
    return String(config.path)
  }

  // For services with URL, show the URL
  if (config.url) {
    return String(config.url)
  }

  // For OLLAMA with model
  if (setting.service === 'OLLAMA' && config.model) {
    return `Model: ${config.model}`
  }

  return 'Not configured'
}

export function ServiceConfigCard({
  setting,
  isTestingConnection,
  onTestConnection,
  onConfigure,
}: ServiceConfigCardProps) {
  const metadata = SERVICE_METADATA[setting.service as ServiceType]
  const Icon = SERVICE_ICONS[setting.service as ServiceType] ?? Circle

  return (
    <Card variant="glass" padding="lg" className="flex flex-col">
      {/* Header */}
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className="rounded-lg bg-accent-dark/20 p-2">
            <Icon className="h-5 w-5 text-accent" />
          </div>
          <div>
            <h3 className="text-base font-semibold text-text-primary">
              {metadata?.name ?? setting.service}
            </h3>
            <p className="text-xs text-text-secondary">
              {metadata?.description ?? 'External service'}
            </p>
          </div>
        </div>
        {getStatusIcon(setting)}
      </div>

      {/* Status Badge */}
      <div className="mb-3">
        {getStatusBadge(setting)}
      </div>

      {/* Config Summary */}
      <div className="flex-1 mb-4">
        <p className="text-sm text-text-secondary mb-1">Configuration</p>
        <p className="text-sm text-text-primary font-mono truncate">
          {getConfigSummary(setting)}
        </p>
        {metadata?.hasSecret && (
          <div className="flex items-center gap-1 mt-2">
            <Key className="h-3 w-3 text-text-tertiary" />
            <span className="text-xs text-text-tertiary">
              {setting.hasSecret ? `${metadata.secretLabel ?? 'Secret'} configured` : `No ${metadata.secretLabel?.toLowerCase() ?? 'secret'} set`}
            </span>
          </div>
        )}
      </div>

      {/* Error Display */}
      {setting.connectionError && (
        <div className="mb-4 rounded-lg bg-error/10 border border-error/20 p-3">
          <p className="text-xs text-error">{setting.connectionError}</p>
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-2">
        <Button
          variant="secondary"
          size="sm"
          onClick={onTestConnection}
          disabled={!setting.enabled || isTestingConnection}
          className="flex-1"
        >
          <RefreshCw className={`h-4 w-4 mr-2 ${isTestingConnection ? 'animate-spin' : ''}`} />
          Test
        </Button>
        <Button variant="secondary" size="sm" onClick={onConfigure} className="flex-1">
          <Settings className="h-4 w-4 mr-2" />
          Configure
        </Button>
      </div>
    </Card>
  )
}
