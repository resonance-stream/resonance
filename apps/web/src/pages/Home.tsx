export default function Home() {
  return (
    <div className="flex flex-1 flex-col p-6">
      <h1 className="text-3xl font-bold">Welcome to Resonance</h1>
      <p className="mt-2 text-text-secondary">
        Your self-hosted music streaming platform
      </p>

      <section className="mt-8">
        <h2 className="text-xl font-semibold">Recently Played</h2>
        <div className="mt-4 grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {/* Placeholder cards */}
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="card group cursor-pointer transition-all hover:bg-background-tertiary"
            >
              <div className="aspect-square rounded-lg bg-background-tertiary" />
              <h3 className="mt-2 truncate font-medium">Album Title</h3>
              <p className="truncate text-sm text-text-secondary">Artist Name</p>
            </div>
          ))}
        </div>
      </section>

      <section className="mt-8">
        <h2 className="text-xl font-semibold">Made For You</h2>
        <div className="mt-4 grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="card group cursor-pointer transition-all hover:bg-background-tertiary"
            >
              <div className="aspect-square rounded-lg bg-gradient-to-br from-primary to-accent" />
              <h3 className="mt-2 truncate font-medium">Discover Weekly</h3>
              <p className="truncate text-sm text-text-secondary">
                Based on your listening
              </p>
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}
