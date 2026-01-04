/**
 * Test Utilities Index
 *
 * Re-exports all test utilities for convenient importing.
 *
 * Usage:
 *   import { render, screen, userEvent } from '@/test'
 */

export * from './test-utils'
export { mockTracks, mockAlbums, mockArtists, mockPlaylists, mockUser } from './mocks/handlers'
export {
  MockWebSocket,
  resetMockWebSocket,
  installMockWebSocket,
  createConnectedMessage,
  createChatTokenMessage,
  createChatCompleteMessage,
  createChatErrorMessage,
  createPongMessage,
  createDeviceListMessage,
  createPlaybackSyncMessage,
} from './mocks/websocket'
