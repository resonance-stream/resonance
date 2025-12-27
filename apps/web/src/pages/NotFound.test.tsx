/**
 * NotFound Page Tests
 *
 * Tests for the 404 Not Found page component.
 */

import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/test-utils'
import NotFound from './NotFound'

describe('NotFound', () => {
  it('renders the 404 heading', () => {
    render(<NotFound />)

    expect(screen.getByRole('heading', { name: '404' })).toBeInTheDocument()
  })

  it('renders the error message', () => {
    render(<NotFound />)

    expect(screen.getByText(/page not found/i)).toBeInTheDocument()
  })

  it('renders a link to go home', () => {
    render(<NotFound />)

    const homeLink = screen.getByRole('link', { name: /go home/i })
    expect(homeLink).toBeInTheDocument()
    expect(homeLink).toHaveAttribute('href', '/')
  })
})
