import { useParams } from 'react-router-dom'
import { Play, Heart, MoreHorizontal, Clock } from 'lucide-react'
import { Button } from '../components/ui/Button'
import { AlbumArt } from '../components/media/AlbumArt'
import { QualityBadge } from '../components/ui/Badge'

export default function Album() {
  const { id } = useParams<{ id: string }>()

  // Placeholder data
  const album = {
    title: 'Album Title',
    artist: 'Artist Name',
    year: 2024,
    trackCount: 12,
    duration: '48 min',
    tracks: Array.from({ length: 12 }, (_, i) => ({
      id: i + 1,
      title: `Track ${i + 1}`,
      duration: `${3 + Math.floor(Math.random() * 2)}:${String(Math.floor(Math.random() * 60)).padStart(2, '0')}`,
      plays: Math.floor(Math.random() * 1000000),
    })),
  }

  return (
    <div className="flex flex-1 flex-col p-6 animate-fade-in">
      {/* Album Header */}
      <div className="flex flex-col md:flex-row gap-8 mb-8">
        <AlbumArt
          alt={album.title}
          size="xl"
          showPlayButton={false}
          className="flex-shrink-0 self-center md:self-start"
        />

        <div className="flex flex-col justify-end">
          <span className="text-overline text-text-muted uppercase tracking-wider">
            Album
          </span>
          <h1 className="font-display text-display-xl text-text-primary mt-2">
            {album.title}
          </h1>
          <p className="text-lg text-text-secondary mt-2">
            {album.artist}
          </p>
          <p className="text-sm text-text-muted mt-1">
            {album.year} - {album.trackCount} songs, {album.duration}
          </p>

          {/* Action Buttons */}
          <div className="flex items-center gap-4 mt-6">
            <Button variant="accent" className="gap-2 rounded-full px-8">
              <Play size={20} fill="currentColor" className="ml-0.5" />
              Play
            </Button>
            <Button variant="ghost" size="icon" aria-label="Like album" className="hover:text-mint">
              <Heart size={24} />
            </Button>
            <Button variant="ghost" size="icon" aria-label="More options">
              <MoreHorizontal size={24} />
            </Button>
          </div>
        </div>
      </div>

      {/* Track List */}
      <div className="mt-4">
        {/* Header */}
        <div className="grid grid-cols-[auto_1fr_auto_auto] gap-4 px-4 py-2 text-sm text-text-muted border-b border-white/5">
          <span className="w-8 text-center">#</span>
          <span>Title</span>
          <span className="w-20 text-center">Quality</span>
          <span className="w-16 text-right flex items-center justify-end">
            <Clock size={16} />
          </span>
        </div>

        {/* Tracks */}
        <div className="divide-y divide-white/5">
          {album.tracks.map((track) => (
            <div
              key={track.id}
              className="grid grid-cols-[auto_1fr_auto_auto] gap-4 px-4 py-3 text-sm hover:bg-background-tertiary/50 rounded-lg cursor-pointer group transition-colors"
            >
              <span className="w-8 text-center text-text-muted group-hover:hidden">
                {track.id}
              </span>
              <span className="w-8 text-center hidden group-hover:flex items-center justify-center">
                <Play size={14} className="text-navy" fill="currentColor" />
              </span>
              <span className="text-text-primary truncate">{track.title}</span>
              <span className="w-20 flex items-center justify-center">
                <QualityBadge format="flac" />
              </span>
              <span className="w-16 text-right text-text-muted">{track.duration}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Album ID (debug) */}
      <p className="mt-8 text-xs text-text-disabled">Album ID: {id}</p>
    </div>
  )
}
