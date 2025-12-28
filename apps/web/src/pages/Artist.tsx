import { useParams } from 'react-router-dom'
import { Play, UserPlus, MoreHorizontal } from 'lucide-react'
import { Button } from '../components/ui/Button'
import { MediaCard } from '../components/media'

export default function Artist() {
  const { id } = useParams<{ id: string }>()

  // Placeholder data
  const artist = {
    name: 'Artist Name',
    monthlyListeners: '1.2M',
    albums: Array.from({ length: 6 }, (_, i) => ({
      id: i + 1,
      title: `Album ${i + 1}`,
      year: 2024 - i,
    })),
    topTracks: Array.from({ length: 5 }, (_, i) => ({
      id: i + 1,
      title: `Popular Track ${i + 1}`,
      album: `Album ${(i % 3) + 1}`,
      plays: `${Math.floor(Math.random() * 100)}M`,
    })),
  }

  return (
    <div className="flex flex-1 flex-col animate-fade-in">
      {/* Hero Section */}
      <div className="relative h-80 bg-gradient-to-b from-accent-dark/50 to-background flex items-end p-6">
        <div>
          <span className="text-overline text-text-muted uppercase tracking-wider">
            Artist
          </span>
          <h1 className="font-display text-display-xl text-text-primary mt-2">
            {artist.name}
          </h1>
          <p className="text-text-secondary mt-2">
            {artist.monthlyListeners} monthly listeners
          </p>
        </div>
      </div>

      {/* Content */}
      <div className="p-6">
        {/* Action Buttons */}
        <div className="flex items-center gap-4 mb-8">
          <Button variant="accent" className="gap-2 rounded-full px-8">
            <Play size={20} fill="currentColor" className="ml-0.5" />
            Play
          </Button>
          <Button variant="secondary" className="gap-2">
            <UserPlus size={18} />
            Follow
          </Button>
          <Button variant="ghost" size="icon" aria-label="More options">
            <MoreHorizontal size={24} />
          </Button>
        </div>

        {/* Popular Tracks */}
        <section className="mb-8">
          <h2 className="text-xl font-semibold text-text-primary mb-4">
            Popular
          </h2>
          <div className="space-y-1">
            {artist.topTracks.map((track, index) => (
              <div
                key={track.id}
                className="flex items-center gap-4 px-4 py-3 hover:bg-background-tertiary/50 rounded-lg cursor-pointer group transition-colors"
              >
                <span className="w-6 text-center text-text-muted group-hover:hidden">
                  {index + 1}
                </span>
                <span className="w-6 hidden group-hover:flex items-center justify-center">
                  <Play size={14} className="text-navy" fill="currentColor" />
                </span>
                <div className="w-10 h-10 rounded bg-background-tertiary flex-shrink-0" />
                <div className="flex-1 min-w-0">
                  <p className="text-text-primary truncate">{track.title}</p>
                  <p className="text-sm text-text-muted truncate">{track.album}</p>
                </div>
                <span className="text-text-muted text-sm">{track.plays}</span>
              </div>
            ))}
          </div>
        </section>

        {/* Discography */}
        <section>
          <h2 className="text-xl font-semibold text-text-primary mb-4">
            Discography
          </h2>
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
            {artist.albums.map((album) => (
              <MediaCard
                key={album.id}
                title={album.title}
                subtitle={String(album.year)}
                href={`/album/${album.id}`}
                onPlay={() => console.log(`Play album ${album.id}`)}
              />
            ))}
          </div>
        </section>

        {/* Artist ID (debug) */}
        <p className="mt-8 text-xs text-text-disabled">Artist ID: {id}</p>
      </div>
    </div>
  )
}
