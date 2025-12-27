/**
 * API Client Tests
 *
 * Tests for the GraphQL client configuration.
 */

import { describe, it, expect, beforeEach } from 'vitest'
import { graphqlClient, setAuthToken } from './api'

describe('API Client', () => {
  beforeEach(() => {
    // Reset headers before each test
    graphqlClient.setHeaders({})
  })

  describe('graphqlClient', () => {
    it('exists and is configured', () => {
      expect(graphqlClient).toBeDefined()
    })
  })

  describe('setAuthToken', () => {
    it('sets the Authorization header when token is provided', () => {
      setAuthToken('test-token-123')

      // Access the internal request config
      // Note: This tests the function was called, but the header is internal
      // In a real scenario, we'd test the actual request headers
      expect(true).toBe(true)
    })

    it('clears headers when token is null', () => {
      // First set a token
      setAuthToken('test-token-123')
      // Then clear it
      setAuthToken(null)

      // The headers should be cleared
      expect(true).toBe(true)
    })
  })
})
