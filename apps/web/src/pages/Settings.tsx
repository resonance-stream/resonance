import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../components/ui/Card'
import { Button } from '../components/ui/Button'

export default function Settings() {
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
            <div className="flex flex-wrap gap-2">
              {['Auto', 'Low', 'Normal', 'High', 'Lossless'].map((quality) => (
                <Button
                  key={quality}
                  variant={quality === 'High' ? 'secondary' : 'ghost'}
                  size="sm"
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
          <CardContent className="mt-4 space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-text-primary">Crossfade</p>
                <p className="text-sm text-text-muted">Smoothly transition between tracks</p>
              </div>
              <Button variant="ghost" size="sm">Off</Button>
            </div>
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-text-primary">Gapless Playback</p>
                <p className="text-sm text-text-muted">Play albums without gaps</p>
              </div>
              <Button variant="secondary" size="sm">On</Button>
            </div>
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-text-primary">Normalize Volume</p>
                <p className="text-sm text-text-muted">Set the same volume for all tracks</p>
              </div>
              <Button variant="secondary" size="sm">On</Button>
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
