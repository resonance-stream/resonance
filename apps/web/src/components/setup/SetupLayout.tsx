/**
 * SetupLayout component for Resonance
 *
 * Provides a centered layout for the setup wizard with branding and progress indicator.
 */

import { cn } from '../../lib/utils'
import type { ReactNode } from 'react'

/**
 * Setup wizard step identifiers
 */
export type SetupStep =
  | 'welcome'
  | 'admin'
  | 'ollama'
  | 'lidarr'
  | 'library'
  | 'lastfm'
  | 'complete'

/**
 * Step metadata for the progress indicator
 */
interface StepInfo {
  id: SetupStep
  label: string
  shortLabel: string
}

const SETUP_STEPS: StepInfo[] = [
  { id: 'welcome', label: 'Welcome', shortLabel: 'Welcome' },
  { id: 'admin', label: 'Admin Account', shortLabel: 'Admin' },
  { id: 'ollama', label: 'AI Configuration', shortLabel: 'AI' },
  { id: 'lidarr', label: 'Lidarr Integration', shortLabel: 'Lidarr' },
  { id: 'library', label: 'Music Library', shortLabel: 'Library' },
  { id: 'lastfm', label: 'Last.fm', shortLabel: 'Last.fm' },
  { id: 'complete', label: 'Complete', shortLabel: 'Done' },
]

interface SetupLayoutProps {
  /** Current step in the wizard */
  currentStep: SetupStep
  /** Child components (step content) */
  children: ReactNode
}

/**
 * Progress indicator showing all steps and current position
 */
function ProgressIndicator({ currentStep }: { currentStep: SetupStep }): JSX.Element {
  const currentIndex = SETUP_STEPS.findIndex((s) => s.id === currentStep)

  return (
    <div className="w-full">
      {/* Desktop: full labels */}
      <div className="hidden sm:flex items-center justify-center gap-2">
        {SETUP_STEPS.map((step, index) => {
          const isCompleted = index < currentIndex
          const isCurrent = index === currentIndex
          const isUpcoming = index > currentIndex

          return (
            <div key={step.id} className="flex items-center">
              {/* Step indicator */}
              <div className="flex flex-col items-center">
                <div
                  className={cn(
                    'w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-all duration-200',
                    isCompleted && 'bg-accent-light text-white',
                    isCurrent && 'bg-primary text-white ring-2 ring-primary ring-offset-2 ring-offset-background',
                    isUpcoming && 'bg-background-tertiary text-text-muted'
                  )}
                >
                  {isCompleted ? (
                    <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                    </svg>
                  ) : (
                    index + 1
                  )}
                </div>
                <span
                  className={cn(
                    'mt-2 text-xs font-medium transition-colors',
                    isCurrent && 'text-text-primary',
                    !isCurrent && 'text-text-muted'
                  )}
                >
                  {step.shortLabel}
                </span>
              </div>

              {/* Connector line */}
              {index < SETUP_STEPS.length - 1 && (
                <div
                  className={cn(
                    'w-8 h-0.5 mx-1 transition-colors',
                    index < currentIndex ? 'bg-accent-light' : 'bg-background-tertiary'
                  )}
                />
              )}
            </div>
          )
        })}
      </div>

      {/* Mobile: simplified progress */}
      <div className="sm:hidden">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm text-text-secondary">
            Step {currentIndex + 1} of {SETUP_STEPS.length}
          </span>
          <span className="text-sm font-medium text-text-primary">
            {SETUP_STEPS[currentIndex]?.label}
          </span>
        </div>
        <div className="h-2 bg-background-tertiary rounded-full overflow-hidden">
          <div
            className="h-full bg-primary transition-all duration-300"
            style={{ width: `${((currentIndex + 1) / SETUP_STEPS.length) * 100}%` }}
          />
        </div>
      </div>
    </div>
  )
}

/**
 * Layout component for the setup wizard.
 *
 * Provides:
 * - Centered content area
 * - Resonance branding at the top
 * - Step progress indicator
 * - Consistent styling
 */
export function SetupLayout({ currentStep, children }: SetupLayoutProps): JSX.Element {
  return (
    <div className="min-h-screen bg-background flex flex-col">
      {/* Header with branding */}
      <header className="pt-8 pb-4 px-4">
        <div className="flex flex-col items-center gap-4">
          <img
            src="/logo.png"
            alt="resonance logo"
            className="h-14 w-14 rounded-xl shadow-[0_0_30px_rgba(90,106,125,0.3)]"
          />
          <img
            src="/wordmark.png"
            alt="resonance"
            className="h-6 brightness-0 invert opacity-90"
          />
        </div>
      </header>

      {/* Progress indicator */}
      <div className="px-4 py-6 max-w-3xl mx-auto w-full">
        <ProgressIndicator currentStep={currentStep} />
      </div>

      {/* Main content area */}
      <main className="flex-1 flex items-start justify-center px-4 pb-8">
        <div className="w-full max-w-xl animate-fade-in">
          {children}
        </div>
      </main>
    </div>
  )
}

export { SETUP_STEPS }
export default SetupLayout
