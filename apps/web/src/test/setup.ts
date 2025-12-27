/**
 * Test Setup File
 *
 * This file is executed before each test file and sets up:
 * - Testing Library DOM matchers
 * - MSW server for API mocking
 * - Global test utilities
 */

import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterAll, afterEach, beforeAll } from 'vitest'
import { server } from './mocks/server'

// Start MSW server before all tests
beforeAll(() => {
  server.listen({ onUnhandledRequest: 'warn' })
})

// Reset handlers after each test for test isolation
afterEach(() => {
  cleanup()
  server.resetHandlers()
})

// Clean up MSW server after all tests
afterAll(() => {
  server.close()
})

// Mock window.matchMedia for components that use media queries
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
})

// Mock ResizeObserver for components that use it
class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}
window.ResizeObserver = ResizeObserverMock

// Mock IntersectionObserver for components using lazy loading
class IntersectionObserverMock {
  readonly root = null
  readonly rootMargin = ''
  readonly thresholds: ReadonlyArray<number> = []

  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords(): IntersectionObserverEntry[] {
    return []
  }
}
window.IntersectionObserver = IntersectionObserverMock

// Mock scrollTo to avoid errors in tests
window.scrollTo = () => {}

// Mock Audio API for player tests
window.HTMLMediaElement.prototype.play = async () => {}
window.HTMLMediaElement.prototype.pause = () => {}
window.HTMLMediaElement.prototype.load = () => {}
Object.defineProperty(window.HTMLMediaElement.prototype, 'currentTime', {
  writable: true,
  value: 0,
})
Object.defineProperty(window.HTMLMediaElement.prototype, 'duration', {
  writable: true,
  value: 100,
})
