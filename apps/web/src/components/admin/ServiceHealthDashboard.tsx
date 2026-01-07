/**
 * ServiceHealthDashboard - Grid of health status indicators for all services
 */

import { RefreshCw, CheckCircle, XCircle, Circle } from 'lucide-react'
import { Card } from '../ui/Card'
import { Button } from '../ui/Button'
import { Badge } from '../ui/Badge'
import type { SystemSettingInfo, ServiceType } from '../../types/systemSettings'
import { SERVICE_METADATA } from '../../types/systemSettings'

interface ServiceHealthDashboardProps {
  settings: SystemSettingInfo[]
  isTestingAll: boolean
  onTestAll: () => void
}

function getStatusIcon(setting: SystemSettingInfo) {
  if (!setting.enabled) {
    return <Circle className="h-4 w-4 text-text-muted" />
  }
  if (setting.connectionHealthy === null) {
    return <Circle className="h-4 w-4 text-text-muted" />
  }
  if (setting.connectionHealthy) {
    return <CheckCircle className="h-4 w-4 text-mint" />
  }
  return <XCircle className="h-4 w-4 text-error" />
}

function getStatusBadge(setting: SystemSettingInfo) {
  if (!setting.enabled) {
    return <Badge variant="default" size="sm">Disabled</Badge>
  }
  if (setting.connectionHealthy === null) {
    return <Badge variant="default" size="sm">Not Tested</Badge>
  }
  if (setting.connectionHealthy) {
    return <Badge variant="success" size="sm">Healthy</Badge>
  }
  return <Badge variant="error" size="sm">Error</Badge>
}

function formatLastCheck(timestamp: string | null): string {
  if (!timestamp) return 'Never'
  const date = new Date(timestamp)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)

  if (diffMins < 1) return 'Just now'
  if (diffMins < 60) return `${diffMins}m ago`
  const diffHours = Math.floor(diffMins / 60)
  if (diffHours < 24) return `${diffHours}h ago`
  const diffDays = Math.floor(diffHours / 24)
  return `${diffDays}d ago`
}

function ServiceHealthItem({ setting }: { setting: SystemSettingInfo }) {
  const metadata = SERVICE_METADATA[setting.service as ServiceType]

  return (
    <div className="flex items-center justify-between gap-4 rounded-lg bg-background-tertiary/30 p-3">
      <div className="flex items-center gap-3">
        {getStatusIcon(setting)}
        <div>
          <p className="text-sm font-medium text-text-primary">
            {metadata?.name ?? setting.service}
          </p>
          <p className="text-xs text-text-tertiary">
            Last check: {formatLastCheck(setting.lastConnectionTest)}
          </p>
        </div>
      </div>
      <div className="flex items-center gap-2">
        {getStatusBadge(setting)}
      </div>
    </div>
  )
}

export function ServiceHealthDashboard({
  settings,
  isTestingAll,
  onTestAll,
}: ServiceHealthDashboardProps) {
  // Sort settings by service name
  const sortedSettings = [...settings].sort((a, b) => {
    const nameA = SERVICE_METADATA[a.service as ServiceType]?.name ?? a.service
    const nameB = SERVICE_METADATA[b.service as ServiceType]?.name ?? b.service
    return nameA.localeCompare(nameB)
  })

  const enabledCount = settings.filter((s) => s.enabled).length
  const healthyCount = settings.filter((s) => s.enabled && s.connectionHealthy === true).length
  const errorCount = settings.filter((s) => s.enabled && s.connectionHealthy === false).length

  return (
    <Card variant="glass" padding="lg">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h2 className="text-lg font-semibold text-text-primary">Service Health</h2>
          <p className="text-sm text-text-secondary">
            {healthyCount} of {enabledCount} services healthy
            {errorCount > 0 && (
              <span className="text-error"> ({errorCount} with errors)</span>
            )}
          </p>
        </div>
        <Button
          variant="secondary"
          size="sm"
          onClick={onTestAll}
          disabled={isTestingAll || enabledCount === 0}
        >
          <RefreshCw className={`h-4 w-4 mr-2 ${isTestingAll ? 'animate-spin' : ''}`} />
          Test All
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {sortedSettings.map((setting) => (
          <ServiceHealthItem key={setting.service} setting={setting} />
        ))}
      </div>
    </Card>
  )
}
