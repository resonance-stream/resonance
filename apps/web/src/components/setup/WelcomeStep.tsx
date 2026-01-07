/**
 * WelcomeStep component for Resonance Setup Wizard
 *
 * First step of the setup wizard. Introduces the user to the setup process
 * and explains what will be configured.
 */

import { Music, Brain, Download, Radio, Headphones } from 'lucide-react'
import { Button } from '../ui/Button'
import { Card } from '../ui/Card'

interface WelcomeStepProps {
  /** Callback when user clicks to start setup */
  onNext: () => void
}

/**
 * Feature item displayed in the welcome screen
 */
interface FeatureItemProps {
  icon: React.ReactNode
  title: string
  description: string
}

function FeatureItem({ icon, title, description }: FeatureItemProps): JSX.Element {
  return (
    <div className="flex gap-4 items-start">
      <div className="flex-shrink-0 w-10 h-10 rounded-lg bg-primary/20 flex items-center justify-center text-accent-light">
        {icon}
      </div>
      <div>
        <h3 className="font-medium text-text-primary">{title}</h3>
        <p className="text-sm text-text-secondary mt-1">{description}</p>
      </div>
    </div>
  )
}

/**
 * Welcome step of the setup wizard.
 *
 * Displays:
 * - Brief introduction to Resonance
 * - List of what will be configured
 * - Get Started button to proceed
 */
export function WelcomeStep({ onNext }: WelcomeStepProps): JSX.Element {
  return (
    <Card variant="glass" padding="lg">
      <div className="space-y-6">
        {/* Header */}
        <div className="text-center">
          <h1 className="font-display text-2xl text-text-primary">
            Welcome to Resonance
          </h1>
          <p className="mt-2 text-text-secondary">
            Let's get your self-hosted music streaming platform set up.
            This wizard will guide you through configuring the essential services.
          </p>
        </div>

        {/* Divider */}
        <div className="h-px bg-white/10" />

        {/* What will be configured */}
        <div>
          <h2 className="text-sm font-medium text-text-muted uppercase tracking-wider mb-4">
            What we'll configure
          </h2>
          <div className="space-y-4">
            <FeatureItem
              icon={<Headphones className="w-5 h-5" />}
              title="Admin Account"
              description="Create your administrator account to manage the platform"
            />
            <FeatureItem
              icon={<Brain className="w-5 h-5" />}
              title="AI Integration (Ollama)"
              description="Connect to Ollama for AI-powered recommendations and natural language search"
            />
            <FeatureItem
              icon={<Download className="w-5 h-5" />}
              title="Lidarr Integration"
              description="Link to Lidarr for automatic music library management"
            />
            <FeatureItem
              icon={<Music className="w-5 h-5" />}
              title="Music Library"
              description="Set up the path to your music collection"
            />
            <FeatureItem
              icon={<Radio className="w-5 h-5" />}
              title="Last.fm"
              description="Optional scrobbling and music discovery features"
            />
          </div>
        </div>

        {/* Info box */}
        <div className="rounded-lg bg-primary/10 border border-primary/20 p-4">
          <p className="text-sm text-text-secondary">
            <strong className="text-text-primary">Note:</strong> All services are optional except
            the admin account and music library path. You can skip any service and configure it
            later from the admin settings.
          </p>
        </div>

        {/* Action button */}
        <Button
          onClick={onNext}
          className="w-full"
          size="lg"
        >
          Get Started
        </Button>
      </div>
    </Card>
  )
}

export default WelcomeStep
