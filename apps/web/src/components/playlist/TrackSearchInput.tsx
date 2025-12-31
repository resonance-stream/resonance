/**
 * Track Search Input Component
 *
 * An autocomplete input for searching and selecting seed tracks
 * for the similar_to rule type. Features:
 * - Debounced search
 * - Dropdown with search results
 * - Keyboard navigation (Arrow keys, Enter, Escape)
 * - Selected tracks display with remove
 * - Proper ARIA combobox pattern
 */

import { memo, useState, useCallback, useRef, useEffect, useId } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Search, X, Music, Loader2, AlertCircle } from 'lucide-react'
import { Input } from '../ui/Input'
import { graphqlClient } from '../../lib/api'
import { SEARCH_TRACKS_FOR_SEEDS_QUERY } from '../../lib/graphql/playlist'
import { libraryKeys } from '../../lib/queryKeys'
import { useDebouncedValue } from '../../hooks/useDebouncedValue'
import { cn } from '../../lib/utils'
import { VALIDATION_LIMITS } from '../../types/playlist'

// ============================================================================
// Types
// ============================================================================

interface TrackResult {
  id: string
  title: string
  durationMs: number
  formattedDuration: string
  genres: string[]
  album: {
    id: string
    title: string
    coverArtUrl: string | null
  }
  artist: {
    id: string
    name: string
  }
}

interface SearchTracksResponse {
  searchTracks: TrackResult[]
}

interface TrackSearchInputProps {
  /** Currently selected track IDs */
  selectedTrackIds: string[]
  /** Callback when selected tracks change */
  onChange: (trackIds: string[]) => void
  /** Whether the input is disabled */
  disabled?: boolean
}

// ============================================================================
// Hooks
// ============================================================================

/**
 * Hook for searching tracks
 */
function useTrackSearch(query: string, enabled: boolean) {
  const limit = 10
  return useQuery({
    queryKey: libraryKeys.tracks.seedSearch(query, limit),
    queryFn: async () => {
      const response = await graphqlClient.request<SearchTracksResponse>(
        SEARCH_TRACKS_FOR_SEEDS_QUERY,
        { query, limit }
      )
      return response.searchTracks
    },
    enabled: enabled && query.length >= 2,
    staleTime: 30000, // Cache for 30 seconds
  })
}

// ============================================================================
// Track Result Item
// ============================================================================

interface TrackResultItemProps {
  track: TrackResult
  isSelected: boolean
  isFocused: boolean
  onSelect: (track: TrackResult) => void
  id: string
}

const TrackResultItem = memo(function TrackResultItem({
  track,
  isSelected,
  isFocused,
  onSelect,
  id,
}: TrackResultItemProps): JSX.Element {
  const itemRef = useRef<HTMLButtonElement>(null)

  // Scroll into view when focused
  useEffect(() => {
    if (isFocused && itemRef.current) {
      itemRef.current.scrollIntoView({ block: 'nearest' })
    }
  }, [isFocused])

  return (
    <button
      ref={itemRef}
      id={id}
      type="button"
      role="option"
      aria-selected={isSelected}
      aria-disabled={isSelected}
      data-focused={isFocused || undefined}
      onClick={() => !isSelected && onSelect(track)}
      className={cn(
        'w-full flex items-center gap-3 px-3 py-2 text-left',
        'transition-colors duration-150',
        isFocused && 'bg-accent/20',
        !isFocused && 'hover:bg-accent/10',
        isSelected && 'opacity-50 cursor-not-allowed'
      )}
    >
      {/* Album art or placeholder */}
      {track.album.coverArtUrl ? (
        <img
          src={track.album.coverArtUrl}
          alt=""
          className="w-10 h-10 rounded object-cover"
        />
      ) : (
        <div className="w-10 h-10 rounded bg-background-tertiary flex items-center justify-center">
          <Music size={16} className="text-text-muted" aria-hidden="true" />
        </div>
      )}

      {/* Track info */}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium text-text-primary truncate">
          {track.title}
        </div>
        <div className="text-xs text-text-muted truncate">
          {track.artist.name} - {track.album.title}
        </div>
      </div>

      {/* Duration */}
      <span className="text-xs text-text-muted flex-shrink-0">
        {track.formattedDuration}
      </span>
    </button>
  )
})

// ============================================================================
// Selected Track Chip
// ============================================================================

interface SelectedTrackChipProps {
  track: TrackResult
  onRemove: (id: string) => void
  disabled: boolean
}

const SelectedTrackChip = memo(function SelectedTrackChip({
  track,
  onRemove,
  disabled,
}: SelectedTrackChipProps): JSX.Element {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-2 px-2 py-1 rounded-lg',
        'bg-accent/20 text-text-primary text-sm',
        disabled && 'opacity-50'
      )}
    >
      {/* Small album art */}
      {track.album.coverArtUrl ? (
        <img
          src={track.album.coverArtUrl}
          alt=""
          className="w-5 h-5 rounded object-cover"
        />
      ) : (
        <Music size={12} className="text-text-muted" aria-hidden="true" />
      )}

      {/* Track title */}
      <span className="truncate max-w-[150px]" title={`${track.title} - ${track.artist.name}`}>
        {track.title}
      </span>

      {/* Remove button */}
      <button
        type="button"
        onClick={() => onRemove(track.id)}
        disabled={disabled}
        className={cn(
          'text-text-muted hover:text-text-primary',
          'focus:outline-none focus-visible:ring-1 focus-visible:ring-accent',
          'disabled:opacity-50 disabled:cursor-not-allowed'
        )}
        aria-label={`Remove ${track.title}`}
      >
        <X size={14} aria-hidden="true" />
      </button>
    </span>
  )
})

// ============================================================================
// Main Component
// ============================================================================

export const TrackSearchInput = memo(function TrackSearchInput({
  selectedTrackIds,
  onChange,
  disabled = false,
}: TrackSearchInputProps): JSX.Element {
  const [searchQuery, setSearchQuery] = useState('')
  const [isOpen, setIsOpen] = useState(false)
  const [focusedIndex, setFocusedIndex] = useState(-1)
  // Track cache for selected tracks (keyed by ID for quick lookup)
  const [trackCache, setTrackCache] = useState<Map<string, TrackResult>>(new Map())
  const containerRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  // Generate unique IDs for ARIA
  const listboxId = useId()
  const getOptionId = (trackId: string) => `${listboxId}-option-${trackId}`

  // Debounce search query
  const debouncedQuery = useDebouncedValue(searchQuery, 300)

  // Search for tracks
  const { data: searchResults, isLoading, isError } = useTrackSearch(
    debouncedQuery,
    isOpen && !disabled
  )

  // Get selected tracks from cache
  const selectedTracks = selectedTrackIds
    .map((id) => trackCache.get(id))
    .filter((track): track is TrackResult => track !== undefined)

  // Check if max seed tracks reached
  const isMaxReached = selectedTrackIds.length >= VALIDATION_LIMITS.MAX_SEED_TRACKS

  // Get current focused track ID for ARIA
  const focusedTrackId = focusedIndex >= 0 && searchResults?.[focusedIndex]
    ? searchResults[focusedIndex].id
    : undefined

  // Handle track selection
  const handleSelect = useCallback(
    (track: TrackResult) => {
      if (isMaxReached || selectedTrackIds.includes(track.id)) return

      // Add track to cache
      setTrackCache((prev) => {
        const next = new Map(prev)
        next.set(track.id, track)
        return next
      })

      // Update selected IDs
      const newIds = [...selectedTrackIds, track.id]
      onChange(newIds)

      // Reset search
      setSearchQuery('')
      setFocusedIndex(-1)
      inputRef.current?.focus()
    },
    [selectedTrackIds, onChange, isMaxReached]
  )

  // Handle track removal
  const handleRemove = useCallback(
    (trackId: string) => {
      const newIds = selectedTrackIds.filter((id) => id !== trackId)
      onChange(newIds)
      // Keep track in cache in case user re-adds it
    },
    [selectedTrackIds, onChange]
  )

  // Reset focused index when results change
  useEffect(() => {
    setFocusedIndex(-1)
  }, [searchResults])

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false)
        setFocusedIndex(-1)
      }
    }

    document.addEventListener('mousedown', handleClickOutside)
    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [])

  // Handle keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const results = searchResults ?? []

      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault()
          if (!isOpen) {
            setIsOpen(true)
          } else if (results.length > 0) {
            setFocusedIndex((prev) =>
              prev < results.length - 1 ? prev + 1 : prev
            )
          }
          break

        case 'ArrowUp':
          e.preventDefault()
          if (results.length > 0) {
            setFocusedIndex((prev) => (prev > 0 ? prev - 1 : 0))
          }
          break

        case 'Enter':
          e.preventDefault()
          if (focusedIndex >= 0 && focusedIndex < results.length) {
            const track = results[focusedIndex]
            if (track && !selectedTrackIds.includes(track.id)) {
              handleSelect(track)
            }
          }
          break

        case 'Escape':
          e.preventDefault()
          setIsOpen(false)
          setSearchQuery('')
          setFocusedIndex(-1)
          break

        case 'Backspace':
          // Remove last selected track if input is empty
          if (searchQuery === '' && selectedTrackIds.length > 0) {
            const lastId = selectedTrackIds[selectedTrackIds.length - 1]
            if (lastId) {
              handleRemove(lastId)
            }
          }
          break

        case 'Home':
          e.preventDefault()
          if (results.length > 0) {
            setFocusedIndex(0)
          }
          break

        case 'End':
          e.preventDefault()
          if (results.length > 0) {
            setFocusedIndex(results.length - 1)
          }
          break

        case 'Tab':
          // Close dropdown on Tab, allow natural focus movement
          setIsOpen(false)
          setFocusedIndex(-1)
          break
      }
    },
    [searchResults, isOpen, focusedIndex, selectedTrackIds, searchQuery, handleSelect, handleRemove]
  )

  const showDropdown = isOpen && debouncedQuery.length >= 2

  return (
    <div ref={containerRef} className="flex flex-col gap-2 flex-1 min-w-[200px]">
      {/* Selected tracks */}
      {selectedTracks.length > 0 && (
        <div className="flex flex-wrap gap-1.5" role="list" aria-label="Selected seed tracks">
          {selectedTracks.map((track) => (
            <SelectedTrackChip
              key={track.id}
              track={track}
              onRemove={handleRemove}
              disabled={disabled}
            />
          ))}
        </div>
      )}

      {/* Search input with dropdown */}
      <div className="relative">
        <div className="relative">
          <Search
            size={16}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted"
            aria-hidden="true"
          />
          <Input
            ref={inputRef}
            type="text"
            role="combobox"
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value)
              setIsOpen(true)
            }}
            onFocus={() => setIsOpen(true)}
            onKeyDown={handleKeyDown}
            placeholder={
              isMaxReached
                ? `Max ${VALIDATION_LIMITS.MAX_SEED_TRACKS} tracks reached`
                : 'Search for tracks...'
            }
            disabled={disabled || isMaxReached}
            className="pl-9 pr-8"
            aria-label="Search for seed tracks"
            aria-expanded={showDropdown && (searchResults?.length ?? 0) > 0}
            aria-haspopup="listbox"
            aria-controls={listboxId}
            aria-activedescendant={focusedTrackId ? getOptionId(focusedTrackId) : undefined}
            aria-autocomplete="list"
          />
          {isLoading && (
            <>
              <Loader2
                size={16}
                className="absolute right-3 top-1/2 -translate-y-1/2 text-text-muted animate-spin"
                aria-hidden="true"
              />
              <span className="sr-only" role="status">Searching for tracks...</span>
            </>
          )}
        </div>

        {/* Dropdown with results */}
        {showDropdown && searchResults && searchResults.length > 0 && (
          <div
            id={listboxId}
            className={cn(
              'absolute z-50 mt-1 w-full max-h-64 overflow-y-auto',
              'rounded-lg border border-white/10 bg-background-secondary shadow-lg'
            )}
            role="listbox"
            aria-label="Search results"
          >
            {searchResults.map((track, index) => (
              <TrackResultItem
                key={track.id}
                id={getOptionId(track.id)}
                track={track}
                isSelected={selectedTrackIds.includes(track.id)}
                isFocused={index === focusedIndex}
                onSelect={handleSelect}
              />
            ))}
          </div>
        )}

        {/* Error message */}
        {showDropdown && isError && (
          <div
            className={cn(
              'absolute z-50 mt-1 w-full py-3 px-4',
              'rounded-lg border border-error/20 bg-background-secondary shadow-lg',
              'text-sm text-error-text flex items-center gap-2'
            )}
            role="alert"
          >
            <AlertCircle size={16} aria-hidden="true" />
            Failed to search tracks. Please try again.
          </div>
        )}

        {/* No results message */}
        {showDropdown && !isLoading && !isError && searchResults?.length === 0 && (
          <div
            className={cn(
              'absolute z-50 mt-1 w-full py-3 px-4',
              'rounded-lg border border-white/10 bg-background-secondary shadow-lg',
              'text-sm text-text-muted text-center'
            )}
            role="status"
          >
            No tracks found for "{debouncedQuery}"
          </div>
        )}
      </div>

      {/* Helper text */}
      <p className="text-xs text-text-muted">
        {selectedTrackIds.length}/{VALIDATION_LIMITS.MAX_SEED_TRACKS} seed tracks selected
      </p>
    </div>
  )
})

// Export component types for testing
export type { TrackSearchInputProps, TrackResult }
