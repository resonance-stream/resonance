/**
 * MSW Server Setup
 *
 * This creates the MSW server for use in Node.js test environment.
 * The browser version would use setupWorker instead.
 */

import { setupServer } from 'msw/node'
import { handlers } from './handlers'

// Create the server with default handlers
export const server = setupServer(...handlers)
