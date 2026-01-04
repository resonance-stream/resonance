import { useState, useCallback, useEffect, useRef } from 'react'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../components/ui/Card'
import { Button } from '../components/ui/Button'
import { Switch } from '../components/ui/Switch'
import { Slider } from '../components/ui/Slider'
import {
  IntegrationsSettings,
  ChangePasswordModal,
  ChangeEmailModal,
  EditProfileModal,
} from '../components/settings'
import { useSettingsStore } from '../stores/settingsStore'
import { useAuthStore } from '../stores/authStore'
import { useUserPreferences, useUpdatePreferences } from '../hooks/usePreferences'

// Note: Crossfade settings are synced to AudioEngine in AudioProvider.tsx
// This component updates both local store (for immediate UI) and server (for persistence).
// The useUpdatePreferences hook handles server sync with optimistic updates.

/**
 * Calculate relative time string from a date
 */
function formatRelativeTime(dateString: string | undefined): string {
  if (!dateString) {
    return 'Never'
  }

  const date = new Date(dateString)

  // Handle invalid date strings
  if (isNaN(date.getTime())) {
    return 'Never'
  }

  const now = new Date()
  const diffMs = now.getTime() - date.getTime()

  // Handle future dates (clock skew protection)
  if (diffMs < 0) {
    return 'Just now'
  }

  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))

  if (diffDays === 0) {
    return 'Today'
  } else if (diffDays === 1) {
    return 'Yesterday'
  } else if (diffDays < 30) {
    return `${diffDays} days ago`
  } else if (diffDays < 365) {
    const months = Math.floor(diffDays / 30)
    return `${months} month${months === 1 ? '' : 's'} ago`
  } else {
    const years = Math.floor(diffDays / 365)
    return `${years} year${years === 1 ? '' : 's'} ago`
  }
}

export default function Settings() {
  const {
    playback,
    audioQuality,
    setCrossfadeEnabled,
    setCrossfadeDuration,
    setGaplessEnabled,
    setNormalizeVolume,
    setAudioQuality,
  } = useSettingsStore()

  const user = useAuthStore((s) => s.user)

  // Server preferences sync
  const { data: serverPreferences } = useUserPreferences()
  const updatePreferences = useUpdatePreferences()

  // Track whether initial sync has been performed to prevent re-syncing
  // on every render. This allows us to properly specify dependencies
  // without causing infinite loops.
  const hasInitialSyncRef = useRef(false)

  // Sync server preferences to local store on initial load only
  useEffect(() => {
    // Skip if no server preferences or already synced
    if (!serverPreferences || hasInitialSyncRef.current) {
      return
    }

    // Mark as synced before applying to prevent re-entry
    hasInitialSyncRef.current = true

    // Apply server preferences to local store
    const crossfadeEnabled = serverPreferences.crossfadeDurationMs > 0
    const crossfadeDuration = Math.round(serverPreferences.crossfadeDurationMs / 1000)

    setCrossfadeEnabled(crossfadeEnabled)
    if (crossfadeEnabled) {
      setCrossfadeDuration(crossfadeDuration)
    }
    setGaplessEnabled(serverPreferences.gaplessPlayback)
    setNormalizeVolume(serverPreferences.normalizeVolume)

    // Map server quality to local quality
    const qualityMap: Record<string, 'auto' | 'low' | 'normal' | 'high' | 'lossless'> = {
      low: 'low',
      medium: 'normal',
      high: 'high',
      lossless: 'lossless',
    }
    const localQuality = qualityMap[serverPreferences.quality] ?? 'high'
    setAudioQuality(localQuality)
  }, [
    serverPreferences,
    setCrossfadeEnabled,
    setCrossfadeDuration,
    setGaplessEnabled,
    setNormalizeVolume,
    setAudioQuality,
  ])

  // Modal state
  const [isPasswordModalOpen, setIsPasswordModalOpen] = useState(false)
  const [isEmailModalOpen, setIsEmailModalOpen] = useState(false)
  const [isProfileModalOpen, setIsProfileModalOpen] = useState(false)

  const formatDuration = (seconds: number): string => {
    return `${seconds}s`
  }

  // Handlers that update both local store and server
  const handleCrossfadeEnabledChange = useCallback((enabled: boolean) => {
    setCrossfadeEnabled(enabled)
    // When disabling, set crossfade to 0ms; when enabling, use current duration
    const durationMs = enabled ? playback.crossfadeDuration * 1000 : 0
    updatePreferences.mutate({ crossfadeDurationMs: durationMs })
  }, [setCrossfadeEnabled, playback.crossfadeDuration, updatePreferences])

  const handleCrossfadeDurationChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const duration = Number(e.target.value)
    setCrossfadeDuration(duration)
    // Convert seconds to milliseconds for server
    updatePreferences.mutate({ crossfadeDurationMs: duration * 1000 })
  }, [setCrossfadeDuration, updatePreferences])

  const handleGaplessChange = useCallback((enabled: boolean) => {
    setGaplessEnabled(enabled)
    updatePreferences.mutate({ gaplessPlayback: enabled })
  }, [setGaplessEnabled, updatePreferences])

  const handleNormalizeVolumeChange = useCallback((enabled: boolean) => {
    setNormalizeVolume(enabled)
    updatePreferences.mutate({ normalizeVolume: enabled })
  }, [setNormalizeVolume, updatePreferences])

  const handleAudioQualityChange = useCallback((quality: 'auto' | 'low' | 'normal' | 'high' | 'lossless') => {
    setAudioQuality(quality)
    // Map local quality to server quality
    // Local uses: auto, low, normal, high, lossless
    // Server uses: low, medium, high, lossless (no auto)
    const serverQualityMap: Record<string, string> = {
      auto: 'high', // Default to high for auto
      low: 'low',
      normal: 'medium',
      high: 'high',
      lossless: 'lossless',
    }
    updatePreferences.mutate({ quality: serverQualityMap[quality] })
  }, [setAudioQuality, updatePreferences])

  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Header */}
      <div className="mb-8">
        <h1 className="font-display text-display text-text-primary">
          Settings
        </h1>
        <p className="mt-2 text-text-secondary">
          Configure your Resonance experience
        </p>
      </div>

      {/* Settings Sections */}
      <div className="max-w-2xl space-y-6">
        {/* Audio Settings */}
        <Card padding="lg">
          <CardHeader>
            <CardTitle>Audio Quality</CardTitle>
            <CardDescription>
              Choose your preferred streaming quality
            </CardDescription>
          </CardHeader>
          <CardContent className="mt-4">
            <div className="flex flex-wrap gap-2" role="group" aria-label="Audio quality selection">
              {(['auto', 'low', 'normal', 'high', 'lossless'] as const).map((quality) => (
                <Button
                  key={quality}
                  variant={audioQuality.quality === quality ? 'secondary' : 'ghost'}
                  size="sm"
                  onClick={() => handleAudioQualityChange(quality)}
                  className="capitalize"
                  aria-pressed={audioQuality.quality === quality}
                >
                  {quality}
                </Button>
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Playback Settings */}
        <Card padding="lg">
          <CardHeader>
            <CardTitle>Playback</CardTitle>
            <CardDescription>
              Customize your listening experience
            </CardDescription>
          </CardHeader>
          <CardContent className="mt-4 space-y-6">
            {/* Crossfade */}
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium text-text-primary">Crossfade</p>
                  <p className="text-sm text-text-muted">Smoothly transition between tracks</p>
                </div>
                <Switch
                  id="crossfade-toggle"
                  checked={playback.crossfadeEnabled}
                  onCheckedChange={handleCrossfadeEnabledChange}
                  aria-label="Enable crossfade"
                />
              </div>
              {playback.crossfadeEnabled && (
                <div className="pl-4 border-l-2 border-background-tertiary">
                  <Slider
                    min={1}
                    max={12}
                    step={1}
                    value={playback.crossfadeDuration}
                    onChange={handleCrossfadeDurationChange}
                    valueFormatter={formatDuration}
                    aria-label="Crossfade duration"
                  />
                </div>
              )}
            </div>

            {/* Gapless Playback */}
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-text-primary">Gapless Playback</p>
                <p className="text-sm text-text-muted">Play albums without gaps</p>
              </div>
              <Switch
                id="gapless-toggle"
                checked={playback.gaplessEnabled}
                onCheckedChange={handleGaplessChange}
                aria-label="Enable gapless playback"
              />
            </div>

            {/* Volume Normalization */}
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-text-primary">Normalize Volume</p>
                <p className="text-sm text-text-muted">Set the same volume for all tracks</p>
              </div>
              <Switch
                id="normalize-volume-toggle"
                checked={playback.normalizeVolume}
                onCheckedChange={handleNormalizeVolumeChange}
                aria-label="Enable volume normalization"
              />
            </div>
          </CardContent>
        </Card>

        {/* Integrations */}
        <IntegrationsSettings />

        {/* Account Settings */}
        <Card padding="lg">
          <CardHeader>
            <CardTitle>Account</CardTitle>
            <CardDescription>
              Manage your account settings
            </CardDescription>
          </CardHeader>
          <CardContent className="mt-4 space-y-4">
            {/* Profile Section */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                {user?.avatarUrl ? (
                  <img
                    src={user.avatarUrl}
                    alt="Avatar"
                    className="w-10 h-10 rounded-full object-cover bg-background-tertiary"
                  />
                ) : (
                  <div className="w-10 h-10 rounded-full bg-background-tertiary flex items-center justify-center">
                    <span className="text-text-muted text-lg">
                      {user?.displayName?.[0]?.toUpperCase() || user?.email?.[0]?.toUpperCase() || '?'}
                    </span>
                  </div>
                )}
                <div>
                  <p className="font-medium text-text-primary">Profile</p>
                  <p className="text-sm text-text-muted">
                    {user?.displayName || 'No display name set'}
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="sm" onClick={() => setIsProfileModalOpen(true)}>
                Edit
              </Button>
            </div>

            {/* Email Section */}
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-text-primary">Email</p>
                <p className="text-sm text-text-muted">{user?.email || 'Not set'}</p>
              </div>
              <Button variant="ghost" size="sm" onClick={() => setIsEmailModalOpen(true)}>
                Change
              </Button>
            </div>

            {/* Password Section */}
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-text-primary">Password</p>
                <p className="text-sm text-text-muted">
                  Last changed {formatRelativeTime(user?.passwordUpdatedAt)}
                </p>
              </div>
              <Button variant="ghost" size="sm" onClick={() => setIsPasswordModalOpen(true)}>
                Change
              </Button>
            </div>
          </CardContent>
        </Card>

        {/* Danger Zone */}
        <Card padding="lg" className="border-error/30">
          <CardHeader>
            <CardTitle className="text-error-text">Danger Zone</CardTitle>
            <CardDescription>
              Irreversible account actions
            </CardDescription>
          </CardHeader>
          <CardContent className="mt-4">
            <Button variant="ghost" size="sm" className="text-error-text hover:bg-error/20">
              Delete Account
            </Button>
          </CardContent>
        </Card>
      </div>

      {/* Account Settings Modals */}
      <ChangePasswordModal
        open={isPasswordModalOpen}
        onOpenChange={setIsPasswordModalOpen}
      />
      <ChangeEmailModal
        open={isEmailModalOpen}
        onOpenChange={setIsEmailModalOpen}
        currentEmail={user?.email || ''}
      />
      <EditProfileModal
        open={isProfileModalOpen}
        onOpenChange={setIsProfileModalOpen}
      />
    </div>
  )
}
