import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../components/ui/Card'
import { Button } from '../components/ui/Button'
import { Switch } from '../components/ui/Switch'
import { Slider } from '../components/ui/Slider'
import { useSettingsStore } from '../stores/settingsStore'

// Note: Crossfade settings are synced to AudioEngine in AudioProvider.tsx
// This component only updates the store; AudioProvider handles the sync.

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

  const formatDuration = (seconds: number): string => {
    return `${seconds}s`
  }

  const handleCrossfadeDurationChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setCrossfadeDuration(Number(e.target.value))
  }

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
                  onClick={() => setAudioQuality(quality)}
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
                  onCheckedChange={setCrossfadeEnabled}
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
                onCheckedChange={setGaplessEnabled}
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
                onCheckedChange={setNormalizeVolume}
                aria-label="Enable volume normalization"
              />
            </div>
          </CardContent>
        </Card>

        {/* Account Settings */}
        <Card padding="lg">
          <CardHeader>
            <CardTitle>Account</CardTitle>
            <CardDescription>
              Manage your account settings
            </CardDescription>
          </CardHeader>
          <CardContent className="mt-4 space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-text-primary">Email</p>
                <p className="text-sm text-text-muted">user@example.com</p>
              </div>
              <Button variant="ghost" size="sm">Change</Button>
            </div>
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-text-primary">Password</p>
                <p className="text-sm text-text-muted">Last changed 30 days ago</p>
              </div>
              <Button variant="ghost" size="sm">Change</Button>
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
    </div>
  )
}
