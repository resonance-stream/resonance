/**
 * CompleteStep component for Resonance Setup Wizard
 *
 * Final step showing setup completion summary with configured services
 * and a button to complete setup and start using Resonance.
 */

import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  Loader2,
  CheckCircle2,
  Sparkles,
  SkipForward,
  Bot,
  Radio,
  Music2,
  Headphones,
} from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'
import { useCompleteSetup, useSetupStatus } from '../../hooks/useSetup'
import type { ServiceType } from '@resonance/shared-types'

/**
 * Service display configuration
 */
interface ServiceDisplayConfig {
  id: ServiceType
  label: string
  description: string
  icon: typeof Bot
}

const SERVICE_DISPLAY: ServiceDisplayConfig[] = [
  {
    id: 'OLLAMA',
    label: 'Ollama AI',
    description: 'AI-powered recommendations and search',
    icon: Bot,
  },
  {
    id: 'LIDARR',
    label: 'Lidarr',
    description: 'Automatic music library management',
    icon: Radio,
  },
  {
    id: 'MUSIC_LIBRARY',
    label: 'Music Library',
    description: 'Your music collection path',
    icon: Music2,
  },
  {
    id: 'LASTFM',
    label: 'Last.fm',
    description: 'Scrobbling and social features',
    icon: Headphones,
  },
]

/**
 * Configuration summary item component
 */
function ConfigSummaryItem({
  config,
  isConfigured,
}: {
  config: ServiceDisplayConfig
  isConfigured: boolean
}): JSX.Element {
  const Icon = config.icon

  return (
    <div className="flex items-center gap-3 py-2">
      <div
        className={`w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 ${
          isConfigured
            ? 'bg-accent-light/20 text-accent-light'
            : 'bg-background-tertiary text-text-muted'
        }`}
      >
        <Icon className="w-4 h-4" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-text-primary">{config.label}</p>
        <p className="text-xs text-text-muted truncate">{config.description}</p>
      </div>
      <div className="flex-shrink-0">
        {isConfigured ? (
          <span className="inline-flex items-center gap-1 text-xs font-medium text-accent-light">
            <CheckCircle2 className="w-3.5 h-3.5" />
            Configured
          </span>
        ) : (
          <span className="inline-flex items-center gap-1 text-xs font-medium text-text-muted">
            <SkipForward className="w-3.5 h-3.5" />
            Skipped
          </span>
        )}
      </div>
    </div>
  )
}

/**
 * Completion step of the setup wizard.
 *
 * Shows a summary of what was configured during setup and provides
 * a "Get Started" button to complete setup and redirect to home.
 *
 * Note: This step intentionally has no Back button as all configuration
 * has been completed and users should proceed forward.
 */
export function CompleteStep(): JSX.Element {
  const navigate = useNavigate()
  const completeSetup = useCompleteSetup()
  const { data: setupStatus } = useSetupStatus()
  const [error, setError] = useState<string | null>(null)

  const configuredServices = setupStatus?.configuredServices ?? []

  async function handleComplete(): Promise<void> {
    setError(null)

    try {
      await completeSetup.mutateAsync()
      navigate('/')
    } catch (err) {
      setError((err as Error)?.message || 'Failed to complete setup')
    }
  }

  return (
    <Card variant="glass" padding="lg">
      <div className="space-y-6">
        {/* Success icon with animation */}
        <div className="flex justify-center">
          <div className="relative">
            <div className="w-20 h-20 rounded-full bg-accent-light/20 flex items-center justify-center animate-pulse-slow">
              <CheckCircle2 className="w-10 h-10 text-accent-light" />
            </div>
            <Sparkles className="absolute -top-1 -right-1 w-6 h-6 text-accent-glow animate-bounce-slow" />
            <Sparkles className="absolute -bottom-1 -left-1 w-5 h-5 text-primary animate-bounce-slow animation-delay-150" />
          </div>
        </div>

        {/* Header */}
        <div className="text-center">
          <h1 className="font-display text-2xl text-text-primary">
            You're All Set!
          </h1>
          <p className="mt-2 text-text-secondary">
            Resonance has been configured and is ready to go.
          </p>
        </div>

        {/* Configuration Summary */}
        <div className="space-y-3">
          <h2 className="text-sm font-medium text-text-muted uppercase tracking-wider">
            Configuration Summary
          </h2>

          <div className="rounded-lg bg-background-secondary/50 border border-white/5 divide-y divide-white/5">
            {/* Admin Account - always configured */}
            <div className="flex items-center gap-3 p-3">
              <div className="w-8 h-8 rounded-lg bg-accent-light/20 flex items-center justify-center flex-shrink-0">
                <CheckCircle2 className="w-4 h-4 text-accent-light" />
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-text-primary">
                  Admin Account
                </p>
                <p className="text-xs text-text-muted">
                  Your administrator account is ready
                </p>
              </div>
              <span className="inline-flex items-center gap-1 text-xs font-medium text-accent-light">
                <CheckCircle2 className="w-3.5 h-3.5" />
                Created
              </span>
            </div>

            {/* Service configurations */}
            <div className="p-3 space-y-1">
              {SERVICE_DISPLAY.map((service) => (
                <ConfigSummaryItem
                  key={service.id}
                  config={service}
                  isConfigured={configuredServices.includes(service.id)}
                />
              ))}
            </div>
          </div>
        </div>

        {/* Error message */}
        {error && (
          <div className="rounded-lg bg-error/20 border border-error/30 p-4 text-error-text">
            <p className="text-sm">{error}</p>
          </div>
        )}

        {/* Get Started button - full width, no Back button */}
        <div className="pt-2">
          <Button
            onClick={handleComplete}
            disabled={completeSetup.isPending}
            className="w-full"
            size="lg"
          >
            {completeSetup.isPending ? (
              <span className="flex items-center justify-center gap-2">
                <Loader2 className="h-5 w-5 animate-spin" />
                Starting Resonance...
              </span>
            ) : (
              <span className="flex items-center justify-center gap-2">
                <Sparkles className="h-5 w-5" />
                Get Started
              </span>
            )}
          </Button>
        </div>

        {/* Admin reminder */}
        <div className="rounded-lg bg-background-tertiary/50 border border-white/5 p-4">
          <p className="text-sm text-text-muted">
            <strong className="text-text-secondary">Tip:</strong> You can always
            modify service configurations from the{' '}
            <span className="text-accent-light">Admin → Settings</span> page.
          </p>
        </div>
      </div>
    </Card>
  )
}

export default CompleteStep
