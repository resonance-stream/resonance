/**
 * CompleteStep component for Resonance Setup Wizard
 *
 * Final step showing setup completion and next steps.
 */

import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Loader2, CheckCircle2, Sparkles } from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'
import { useCompleteSetup } from '../../hooks/useSetup'

interface CompleteStepProps {
  /** Callback to go back to previous step */
  onBack: () => void
}

/**
 * Completion step of the setup wizard.
 */
export function CompleteStep({ onBack }: CompleteStepProps): JSX.Element {
  const navigate = useNavigate()
  const completeSetup = useCompleteSetup()
  const [error, setError] = useState<string | null>(null)

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
        {/* Success icon */}
        <div className="flex justify-center">
          <div className="relative">
            <div className="w-20 h-20 rounded-full bg-accent-light/20 flex items-center justify-center">
              <CheckCircle2 className="w-10 h-10 text-accent-light" />
            </div>
            <Sparkles className="absolute -top-1 -right-1 w-6 h-6 text-accent-glow animate-pulse" />
          </div>
        </div>

        {/* Header */}
        <div className="text-center">
          <h1 className="font-display text-2xl text-text-primary">
            Setup Complete!
          </h1>
          <p className="mt-2 text-text-secondary">
            Resonance is ready to use. Let's start exploring your music collection.
          </p>
        </div>

        {/* What's next */}
        <div className="space-y-3">
          <h2 className="text-sm font-medium text-text-muted uppercase tracking-wider">
            What's next
          </h2>
          <ul className="space-y-3 text-sm text-text-secondary">
            <li className="flex items-start gap-3">
              <div className="w-5 h-5 rounded bg-primary/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                <span className="text-xs font-medium text-accent-light">1</span>
              </div>
              <span>
                Your music library will be scanned automatically. This may take a few minutes
                depending on your collection size.
              </span>
            </li>
            <li className="flex items-start gap-3">
              <div className="w-5 h-5 rounded bg-primary/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                <span className="text-xs font-medium text-accent-light">2</span>
              </div>
              <span>
                Invite other users from the Admin panel, or let them register if you've enabled
                open registration.
              </span>
            </li>
            <li className="flex items-start gap-3">
              <div className="w-5 h-5 rounded bg-primary/20 flex items-center justify-center flex-shrink-0 mt-0.5">
                <span className="text-xs font-medium text-accent-light">3</span>
              </div>
              <span>
                Customize your experience in Settings - equalizer, crossfade, theme, and more!
              </span>
            </li>
          </ul>
        </div>

        {/* Error message */}
        {error && (
          <div className="rounded-lg bg-error/20 border border-error/30 p-4 text-error-text">
            <p className="text-sm">{error}</p>
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-3 pt-2">
          <Button
            variant="secondary"
            onClick={onBack}
            disabled={completeSetup.isPending}
            className="flex-1"
          >
            Back
          </Button>
          <Button
            onClick={handleComplete}
            disabled={completeSetup.isPending}
            className="flex-1"
          >
            {completeSetup.isPending ? (
              <span className="flex items-center justify-center gap-2">
                <Loader2 className="h-5 w-5 animate-spin" />
                Finishing...
              </span>
            ) : (
              'Start Listening'
            )}
          </Button>
        </div>

        {/* Admin reminder */}
        <div className="rounded-lg bg-background-tertiary/50 border border-white/5 p-4">
          <p className="text-sm text-text-muted">
            <strong className="text-text-secondary">Tip:</strong> You can always modify service
            configurations from the{' '}
            <span className="text-accent-light">Admin → Settings</span> page.
          </p>
        </div>
      </div>
    </Card>
  )
}

export default CompleteStep
