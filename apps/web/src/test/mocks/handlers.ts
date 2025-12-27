/**
 * MSW Request Handlers
 *
 * Define mock API handlers for testing.
 * These handlers intercept network requests during tests.
 */

import { graphql, http, HttpResponse } from 'msw'

// Mock data
export const mockTracks = [
  {
    id: '1',
    title: 'Test Track 1',
    artist: 'Test Artist',
    albumId: 'album-1',
    albumTitle: 'Test Album',
    duration: 180,
    coverUrl: '/covers/1.jpg',
  },
  {
    id: '2',
    title: 'Test Track 2',
    artist: 'Test Artist',
    albumId: 'album-1',
    albumTitle: 'Test Album',
    duration: 240,
    coverUrl: '/covers/2.jpg',
  },
]

export const mockAlbums = [
  {
    id: 'album-1',
    title: 'Test Album',
    artist: 'Test Artist',
    year: 2024,
    trackCount: 10,
    coverUrl: '/covers/album-1.jpg',
  },
]

export const mockArtists = [
  {
    id: 'artist-1',
    name: 'Test Artist',
    albumCount: 5,
    trackCount: 50,
    imageUrl: '/artists/1.jpg',
  },
]

export const mockPlaylists = [
  {
    id: 'playlist-1',
    name: 'Test Playlist',
    description: 'A test playlist',
    trackCount: 10,
    coverUrl: '/playlists/1.jpg',
  },
]

export const mockUser = {
  id: 'user-1',
  username: 'testuser',
  email: 'test@example.com',
}

// GraphQL handlers
export const graphqlHandlers = [
  // Library queries
  graphql.query('GetTracks', () => {
    return HttpResponse.json({
      data: {
        tracks: mockTracks,
      },
    })
  }),

  graphql.query('GetAlbums', () => {
    return HttpResponse.json({
      data: {
        albums: mockAlbums,
      },
    })
  }),

  graphql.query('GetArtists', () => {
    return HttpResponse.json({
      data: {
        artists: mockArtists,
      },
    })
  }),

  graphql.query('GetPlaylists', () => {
    return HttpResponse.json({
      data: {
        playlists: mockPlaylists,
      },
    })
  }),

  // Search query
  graphql.query('Search', ({ variables }) => {
    const query = (variables as { query: string }).query?.toLowerCase() || ''

    return HttpResponse.json({
      data: {
        search: {
          tracks: mockTracks.filter(
            (t) =>
              t.title.toLowerCase().includes(query) ||
              t.artist.toLowerCase().includes(query)
          ),
          albums: mockAlbums.filter(
            (a) =>
              a.title.toLowerCase().includes(query) ||
              a.artist.toLowerCase().includes(query)
          ),
          artists: mockArtists.filter((a) =>
            a.name.toLowerCase().includes(query)
          ),
        },
      },
    })
  }),

  // User query
  graphql.query('GetCurrentUser', () => {
    return HttpResponse.json({
      data: {
        currentUser: mockUser,
      },
    })
  }),

  // Mutations
  graphql.mutation('CreatePlaylist', ({ variables }) => {
    const { name, description } = variables as {
      name: string
      description?: string
    }
    return HttpResponse.json({
      data: {
        createPlaylist: {
          id: 'new-playlist',
          name,
          description: description || '',
          trackCount: 0,
          coverUrl: null,
        },
      },
    })
  }),
]

// REST API handlers
export const restHandlers = [
  // Stream endpoint
  http.get('/api/stream/:trackId', () => {
    return new HttpResponse(new ArrayBuffer(0), {
      headers: {
        'Content-Type': 'audio/mpeg',
      },
    })
  }),

  // Cover art
  http.get('/api/covers/:id', () => {
    return new HttpResponse(new Blob(), {
      headers: {
        'Content-Type': 'image/jpeg',
      },
    })
  }),

  // Health check
  http.get('/api/health', () => {
    return HttpResponse.json({ status: 'ok' })
  }),
]

// Combined handlers
export const handlers = [...graphqlHandlers, ...restHandlers]
