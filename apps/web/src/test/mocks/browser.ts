/**
 * MSW Browser Setup
 *
 * This creates the MSW service worker for use in the browser.
 * Use this for development or E2E testing with mocked APIs.
 */

import { setupWorker } from 'msw/browser'
import { handlers } from './handlers'

// Create the browser worker with default handlers
export const worker = setupWorker(...handlers)
