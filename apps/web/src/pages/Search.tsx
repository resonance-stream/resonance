import { useState } from 'react'
import { Search as SearchIcon } from 'lucide-react'
import { Input } from '../components/ui/Input'
import { MediaCard } from '../components/media'

export default function Search() {
  const [query, setQuery] = useState('')

  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Header */}
      <div className="mb-8">
        <h1 className="font-display text-display text-text-primary">
          Search
        </h1>
        <p className="mt-2 text-text-secondary">
          Find your favorite music
        </p>
      </div>

      {/* Search Input */}
      <div className="max-w-2xl mb-8">
        <Input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search for songs, albums, or artists..."
          icon={<SearchIcon size={20} />}
        />
      </div>

      {/* Browse Categories (when no search query) */}
      {!query && (
        <section>
          <h2 className="text-xl font-semibold text-text-primary mb-4">
            Browse All
          </h2>
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
            {[
              { name: 'Rock', color: 'from-red-500/30 to-red-700/30' },
              { name: 'Pop', color: 'from-pink-500/30 to-pink-700/30' },
              { name: 'Hip Hop', color: 'from-amber-500/30 to-amber-700/30' },
              { name: 'Electronic', color: 'from-cyan-500/30 to-cyan-700/30' },
              { name: 'Jazz', color: 'from-indigo-500/30 to-indigo-700/30' },
              { name: 'Classical', color: 'from-purple-500/30 to-purple-700/30' },
              { name: 'R&B', color: 'from-teal-500/30 to-teal-700/30' },
              { name: 'Country', color: 'from-orange-500/30 to-orange-700/30' },
              { name: 'Metal', color: 'from-slate-500/30 to-slate-700/30' },
              { name: 'Folk', color: 'from-emerald-500/30 to-emerald-700/30' },
            ].map((genre) => (
              <div
                key={genre.name}
                className={`aspect-square rounded-lg bg-gradient-to-br ${genre.color} flex items-end p-4 cursor-pointer hover:scale-[1.02] transition-transform`}
              >
                <span className="font-semibold text-text-primary">
                  {genre.name}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* Search Results (placeholder) */}
      {query && (
        <section>
          <h2 className="text-xl font-semibold text-text-primary mb-4">
            Results for "{query}"
          </h2>
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
            {Array.from({ length: 6 }).map((_, i) => (
              <MediaCard
                key={i}
                title={`Result ${i + 1}`}
                subtitle="Artist Name"
                href={`/album/${i + 1}`}
                onPlay={() => console.log(`Play result ${i + 1}`)}
              />
            ))}
          </div>
        </section>
      )}
    </div>
  )
}
