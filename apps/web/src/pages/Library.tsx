import { useState } from 'react'
import { LayoutGrid, List } from 'lucide-react'
import { Button } from '../components/ui/Button'
import { MediaCard } from '../components/media'
import { cn } from '../lib/utils'

type ViewMode = 'grid' | 'list'
type FilterType = 'all' | 'albums' | 'playlists' | 'artists'

export default function Library() {
  const [viewMode, setViewMode] = useState<ViewMode>('grid')
  const [filter, setFilter] = useState<FilterType>('all')

  const filters: { key: FilterType; label: string }[] = [
    { key: 'all', label: 'All' },
    { key: 'albums', label: 'Albums' },
    { key: 'playlists', label: 'Playlists' },
    { key: 'artists', label: 'Artists' },
  ]

  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="font-display text-display text-text-primary">
            Your Library
          </h1>
          <p className="mt-1 text-text-secondary">
            Your saved albums, playlists, and artists
          </p>
        </div>

        {/* View Toggle */}
        <div className="flex items-center gap-2">
          <Button
            variant={viewMode === 'grid' ? 'secondary' : 'ghost'}
            size="icon"
            onClick={() => setViewMode('grid')}
            aria-label="Grid view"
          >
            <LayoutGrid size={18} />
          </Button>
          <Button
            variant={viewMode === 'list' ? 'secondary' : 'ghost'}
            size="icon"
            onClick={() => setViewMode('list')}
            aria-label="List view"
          >
            <List size={18} />
          </Button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-2 mb-6">
        {filters.map((f) => (
          <Button
            key={f.key}
            variant={filter === f.key ? 'secondary' : 'ghost'}
            size="sm"
            onClick={() => setFilter(f.key)}
          >
            {f.label}
          </Button>
        ))}
      </div>

      {/* Content */}
      <div
        className={cn(
          'grid gap-4',
          viewMode === 'grid'
            ? 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'
            : 'grid-cols-1'
        )}
      >
        {/* Placeholder items */}
        {Array.from({ length: 12 }).map((_, i) => (
          <MediaCard
            key={i}
            title={`Saved Item ${i + 1}`}
            subtitle={i % 3 === 0 ? 'Album' : i % 3 === 1 ? 'Playlist' : 'Artist'}
            href={i % 3 === 0 ? `/album/${i}` : i % 3 === 1 ? `/playlist/${i}` : `/artist/${i}`}
            onPlay={() => console.log(`Play item ${i + 1}`)}
          />
        ))}
      </div>
    </div>
  )
}
