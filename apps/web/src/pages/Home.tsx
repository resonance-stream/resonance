import { useCallback } from 'react'
import { MediaCard } from '../components/media'
import { SkeletonCard } from '../components/ui/Skeleton'
import { useRecentAlbums, useMyPlaylists, useTopTracks } from '../hooks/useLibrary'
import { usePlayerStore } from '../stores/playerStore'
import { mapGqlTrackToPlayerTrack, mapAlbumToPlayerTracks } from '../lib/mappers'
import type { GqlAlbum, GqlPlaylist, GqlTrack } from '../types/library'

/**
 * Get greeting based on time of day
 */
function getGreeting(): string {
  const hour = new Date().getHours()
  if (hour < 12) return 'Good morning'
  if (hour < 18) return 'Good afternoon'
  return 'Good evening'
}

export default function Home() {
  const { data: recentAlbums, isLoading: loadingAlbums, error: albumsError } = useRecentAlbums(6)
  const { data: playlists, isLoading: loadingPlaylists, error: playlistsError } = useMyPlaylists({ limit: 6 })
  const { data: topTracks, isLoading: loadingTopTracks, error: tracksError } = useTopTracks(6)

  const setTrack = usePlayerStore((s) => s.setTrack)
  const setQueue = usePlayerStore((s) => s.setQueue)

  // Handle playing an album (sets full queue for consistency with other pages)
  const handlePlayAlbum = useCallback((album: GqlAlbum) => {
    if (album.tracks?.length) {
      const tracks = mapAlbumToPlayerTracks(album)
      setQueue(tracks, 0)
    }
  }, [setQueue])

  // Handle playing a track directly
  const handlePlayTrack = useCallback((track: GqlTrack) => {
    const playerTrack = mapGqlTrackToPlayerTrack(track)
    setTrack(playerTrack)
  }, [setTrack])

  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Hero Section */}
      <div className="mb-8">
        <h1 className="font-display text-display text-text-primary">
          {getGreeting()}
        </h1>
        <p className="mt-2 text-text-secondary">
          Welcome back to Resonance
        </p>
      </div>

      {/* Recently Added Albums Section */}
      <section className="mb-8">
        <h2 className="text-xl font-semibold text-text-primary mb-4">
          Recently Added
        </h2>
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {loadingAlbums ? (
            Array.from({ length: 6 }).map((_, i) => (
              <SkeletonCard key={i} />
            ))
          ) : albumsError ? (
            <p role="alert" className="text-text-muted col-span-full">Failed to load albums</p>
          ) : recentAlbums?.length ? (
            recentAlbums.map((album) => (
              <MediaCard
                key={album.id}
                title={album.title}
                subtitle={album.artist?.name ?? 'Unknown Artist'}
                imageUrl={album.coverArtUrl}
                href={`/album/${album.id}`}
                onPlay={album.tracks?.length ? () => handlePlayAlbum(album) : undefined}
              />
            ))
          ) : (
            <p className="text-text-muted col-span-full">No albums yet</p>
          )}
        </div>
      </section>

      {/* Your Playlists Section */}
      <section className="mb-8">
        <h2 className="text-xl font-semibold text-text-primary mb-4">
          Your Playlists
        </h2>
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {loadingPlaylists ? (
            Array.from({ length: 6 }).map((_, i) => (
              <SkeletonCard key={i} />
            ))
          ) : playlistsError ? (
            <p role="alert" className="text-text-muted col-span-full">Failed to load playlists</p>
          ) : playlists?.length ? (
            playlists.map((playlist: GqlPlaylist) => (
              <MediaCard
                key={playlist.id}
                title={playlist.name}
                subtitle={playlist.description || `${playlist.trackCount} tracks`}
                imageUrl={playlist.imageUrl}
                href={`/playlist/${playlist.id}`}
              />
            ))
          ) : (
            <p className="text-text-muted col-span-full">No playlists yet</p>
          )}
        </div>
      </section>

      {/* Top Tracks Section */}
      <section>
        <h2 className="text-xl font-semibold text-text-primary mb-4">
          Top Tracks
        </h2>
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {loadingTopTracks ? (
            Array.from({ length: 6 }).map((_, i) => (
              <SkeletonCard key={i} />
            ))
          ) : tracksError ? (
            <p role="alert" className="text-text-muted col-span-full">Failed to load top tracks</p>
          ) : topTracks?.length ? (
            topTracks.map((track) => (
              <MediaCard
                key={track.id}
                title={track.title}
                subtitle={track.artist?.name ?? 'Unknown Artist'}
                imageUrl={track.album?.coverArtUrl}
                href={track.albumId ? `/album/${track.albumId}` : undefined}
                onPlay={() => handlePlayTrack(track)}
              />
            ))
          ) : (
            <p className="text-text-muted col-span-full">No top tracks yet</p>
          )}
        </div>
      </section>
    </div>
  )
}
