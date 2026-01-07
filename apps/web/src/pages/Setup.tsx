/**
 * Setup Page for Resonance
 *
 * First-run setup wizard that guides users through configuring
 * the essential services for Resonance.
 */

import { useState, useCallback } from 'react'
import { useSetupStatus } from '../hooks/useSetup'
import {
  SetupLayout,
  WelcomeStep,
  AdminStep,
  OllamaStep,
  LidarrStep,
  LibraryStep,
  LastfmStep,
  CompleteStep,
  type SetupStep,
} from '../components/setup'

/**
 * Order of steps in the wizard
 */
const STEP_ORDER: SetupStep[] = [
  'welcome',
  'admin',
  'ollama',
  'lidarr',
  'library',
  'lastfm',
  'complete',
]

/**
 * Setup wizard page component.
 *
 * Manages the step state and renders the appropriate step component.
 */
export default function Setup(): JSX.Element {
  const { data: setupStatus, isLoading } = useSetupStatus()
  const [currentStep, setCurrentStep] = useState<SetupStep>('welcome')

  /**
   * Navigate to the next step
   */
  const handleNext = useCallback(() => {
    const currentIndex = STEP_ORDER.indexOf(currentStep)
    const nextStep = STEP_ORDER[currentIndex + 1]
    if (currentIndex < STEP_ORDER.length - 1 && nextStep) {
      setCurrentStep(nextStep)
    }
  }, [currentStep])

  /**
   * Navigate to the previous step
   */
  const handleBack = useCallback(() => {
    const currentIndex = STEP_ORDER.indexOf(currentStep)
    const prevStep = STEP_ORDER[currentIndex - 1]
    if (currentIndex > 0 && prevStep) {
      setCurrentStep(prevStep)
    }
  }, [currentStep])

  // Show loading while fetching setup status
  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <div className="h-12 w-12 animate-spin rounded-full border-4 border-primary border-t-transparent" />
          <p className="text-text-secondary">Loading setup wizard...</p>
        </div>
      </div>
    )
  }

  /**
   * Render the current step component
   */
  function renderStep(): JSX.Element {
    switch (currentStep) {
      case 'welcome':
        return <WelcomeStep onNext={handleNext} />

      case 'admin':
        return (
          <AdminStep
            onNext={handleNext}
            onBack={handleBack}
            hasAdmin={setupStatus?.hasAdmin ?? false}
          />
        )

      case 'ollama':
        return <OllamaStep onNext={handleNext} onBack={handleBack} />

      case 'lidarr':
        return <LidarrStep onNext={handleNext} onBack={handleBack} />

      case 'library':
        return <LibraryStep onNext={handleNext} onBack={handleBack} />

      case 'lastfm':
        return <LastfmStep onNext={handleNext} onBack={handleBack} />

      case 'complete':
        return <CompleteStep />

      default:
        return <WelcomeStep onNext={handleNext} />
    }
  }

  return (
    <SetupLayout currentStep={currentStep}>
      {renderStep()}
    </SetupLayout>
  )
}
