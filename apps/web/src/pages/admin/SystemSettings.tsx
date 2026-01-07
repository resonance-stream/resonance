/**
 * SystemSettings Admin Page
 *
 * Admin-only page for configuring external services (Ollama, Lidarr, LastFM, etc.)
 */

import { useState } from 'react'
import { ArrowLeft, Server } from 'lucide-react'
import { Link } from 'react-router-dom'
import { Button } from '../../components/ui/Button'
import {
  ServiceHealthDashboard,
  ServiceConfigCard,
  ServiceConfigDialog,
} from '../../components/admin'
import {
  useSystemSettings,
  useUpdateSystemSetting,
  useTestServiceConnection,
  useTestAllConnections,
} from '../../hooks/useSystemSettings'
import type { SystemSettingInfo, ServiceType } from '../../types/systemSettings'
import { SERVICE_CATEGORIES } from '../../types/systemSettings'

export default function SystemSettings() {
  const { data: settings, isLoading, error } = useSystemSettings()
  const updateSetting = useUpdateSystemSetting()
  const testConnection = useTestServiceConnection()
  const testAllConnections = useTestAllConnections()

  const [selectedSetting, setSelectedSetting] = useState<SystemSettingInfo | null>(null)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [testingService, setTestingService] = useState<ServiceType | null>(null)

  const handleTestConnection = async (service: ServiceType) => {
    setTestingService(service)
    try {
      await testConnection.mutateAsync(service)
    } finally {
      setTestingService(null)
    }
  }

  const handleTestAll = async () => {
    if (!settings) return
    const enabledServices = settings
      .filter((s) => s.enabled)
      .map((s) => s.service as ServiceType)
    await testAllConnections.mutateAsync(enabledServices)
  }

  const handleConfigure = (setting: SystemSettingInfo) => {
    setSelectedSetting(setting)
    setDialogOpen(true)
  }

  const handleSave = async (data: {
    service: ServiceType
    enabled: boolean
    config: string
    secret?: string
  }) => {
    await updateSetting.mutateAsync(data)
    setDialogOpen(false)
    setSelectedSetting(null)
  }

  const getSettingsForCategory = (services: ServiceType[]): SystemSettingInfo[] => {
    if (!settings) return []
    return services
      .map((service) => settings.find((s) => s.service === service))
      .filter((s): s is SystemSettingInfo => s !== undefined)
  }

  if (error) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="rounded-lg bg-error/10 border border-error/20 p-6">
          <h2 className="text-lg font-semibold text-error mb-2">Error Loading Settings</h2>
          <p className="text-text-secondary">{error.message}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="container mx-auto space-y-8 px-4 py-8">
      {/* Header */}
      <div className="flex items-start gap-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to="/admin">
            <ArrowLeft className="h-5 w-5" />
          </Link>
        </Button>
        <div>
          <h1 className="text-3xl font-bold text-text-primary flex items-center gap-3">
            <Server className="h-8 w-8" />
            System Settings
          </h1>
          <p className="mt-1 text-text-secondary">
            Configure external services and integrations
          </p>
        </div>
      </div>

      {isLoading ? (
        <div className="space-y-4">
          {/* Loading skeleton */}
          <div className="h-32 rounded-lg bg-background-secondary animate-pulse" />
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="h-48 rounded-lg bg-background-secondary animate-pulse" />
            ))}
          </div>
        </div>
      ) : settings ? (
        <>
          {/* Service Health Dashboard */}
          <ServiceHealthDashboard
            settings={settings}
            isTestingAll={testAllConnections.isPending}
            onTestAll={handleTestAll}
          />

          {/* Service Categories */}
          {Object.entries(SERVICE_CATEGORIES).map(([category, { label, services }]) => {
            const categorySettings = getSettingsForCategory(services)
            if (categorySettings.length === 0) return null

            return (
              <section key={category}>
                <h2 className="text-xl font-semibold text-text-primary mb-4">{label}</h2>
                <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {categorySettings.map((setting) => (
                    <ServiceConfigCard
                      key={setting.service}
                      setting={setting}
                      isTestingConnection={testingService === setting.service}
                      onTestConnection={() => handleTestConnection(setting.service as ServiceType)}
                      onConfigure={() => handleConfigure(setting)}
                    />
                  ))}
                </div>
              </section>
            )
          })}
        </>
      ) : null}

      {/* Configure Dialog */}
      <ServiceConfigDialog
        setting={selectedSetting}
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onSave={handleSave}
        isSaving={updateSetting.isPending}
      />
    </div>
  )
}
