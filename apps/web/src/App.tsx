import { Routes, Route } from 'react-router-dom'
import { Suspense, lazy } from 'react'

// Lazy load pages for code splitting
const Home = lazy(() => import('./pages/Home'))
const Library = lazy(() => import('./pages/Library'))
const Search = lazy(() => import('./pages/Search'))
const Playlist = lazy(() => import('./pages/Playlist'))
const Album = lazy(() => import('./pages/Album'))
const Artist = lazy(() => import('./pages/Artist'))
const Settings = lazy(() => import('./pages/Settings'))
const Login = lazy(() => import('./pages/Login'))
const Register = lazy(() => import('./pages/Register'))
const NotFound = lazy(() => import('./pages/NotFound'))

function LoadingFallback() {
  return (
    <div className="flex h-screen items-center justify-center bg-background">
      <div className="flex flex-col items-center gap-4">
        <div className="h-12 w-12 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        <p className="text-text-secondary">Loading...</p>
      </div>
    </div>
  )
}

function App() {
  return (
    <div className="flex h-screen flex-col bg-background">
      <Suspense fallback={<LoadingFallback />}>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/library" element={<Library />} />
          <Route path="/search" element={<Search />} />
          <Route path="/playlist/:id" element={<Playlist />} />
          <Route path="/album/:id" element={<Album />} />
          <Route path="/artist/:id" element={<Artist />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="/login" element={<Login />} />
          <Route path="/register" element={<Register />} />
          <Route path="*" element={<NotFound />} />
        </Routes>
      </Suspense>
    </div>
  )
}

export default App
