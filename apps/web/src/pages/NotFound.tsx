import { Link } from 'react-router-dom'

export default function NotFound() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center p-6">
      <h1 className="text-6xl font-bold text-primary">404</h1>
      <p className="mt-4 text-xl text-text-secondary">Page not found</p>
      <Link to="/" className="btn-primary mt-6">
        Go Home
      </Link>
    </div>
  )
}
