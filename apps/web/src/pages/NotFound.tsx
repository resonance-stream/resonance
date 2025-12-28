import { Link } from 'react-router-dom'
import { Home } from 'lucide-react'
import { Button } from '../components/ui/Button'

export default function NotFound() {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-background p-6 animate-fade-in">
      <h1 className="font-display text-display-xl text-accent-light">404</h1>
      <p className="mt-4 text-xl text-text-secondary">Page not found</p>
      <p className="mt-2 text-text-muted text-center max-w-md">
        The page you're looking for doesn't exist or has been moved.
      </p>
      <Button asChild className="mt-8 gap-2">
        <Link to="/">
          <Home size={18} />
          Go Home
        </Link>
      </Button>
    </div>
  )
}
