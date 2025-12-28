import { MediaCard } from '../components/media'

export default function Home() {
  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Hero Section */}
      <div className="mb-8">
        <h1 className="font-display text-display text-text-primary">
          Good evening
        </h1>
        <p className="mt-2 text-text-secondary">
          Welcome back to Resonance
        </p>
      </div>

      {/* Recently Played Section */}
      <section className="mb-8">
        <h2 className="text-xl font-semibold text-text-primary mb-4">
          Recently Played
        </h2>
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {Array.from({ length: 6 }).map((_, i) => (
            <MediaCard
              key={i}
              title={`Album ${i + 1}`}
              subtitle="Artist Name"
              href={`/album/${i + 1}`}
              onPlay={() => console.log(`Play album ${i + 1}`)}
            />
          ))}
        </div>
      </section>

      {/* Made For You Section */}
      <section className="mb-8">
        <h2 className="text-xl font-semibold text-text-primary mb-4">
          Made For You
        </h2>
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {[
            { title: 'Discover Weekly', subtitle: 'Your personal mixtape' },
            { title: 'Release Radar', subtitle: 'New music from artists you follow' },
            { title: 'Daily Mix 1', subtitle: 'Based on your recent listening' },
            { title: 'Daily Mix 2', subtitle: 'A mix of your favorites' },
            { title: 'Chill Vibes', subtitle: 'Relaxing tunes for any mood' },
            { title: 'Focus Flow', subtitle: 'Concentration enhancement' },
          ].map((playlist, i) => (
            <MediaCard
              key={i}
              title={playlist.title}
              subtitle={playlist.subtitle}
              href={`/playlist/${i + 1}`}
              onPlay={() => console.log(`Play playlist ${i + 1}`)}
            />
          ))}
        </div>
      </section>

      {/* New Releases Section */}
      <section>
        <h2 className="text-xl font-semibold text-text-primary mb-4">
          New Releases
        </h2>
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {Array.from({ length: 6 }).map((_, i) => (
            <MediaCard
              key={i}
              title={`New Album ${i + 1}`}
              subtitle="Various Artists"
              href={`/album/new-${i + 1}`}
              onPlay={() => console.log(`Play new album ${i + 1}`)}
            />
          ))}
        </div>
      </section>
    </div>
  )
}
