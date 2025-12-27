/**
 * Home Page Tests
 *
 * Tests for the Home page component.
 */

import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/test-utils'
import Home from './Home'

describe('Home', () => {
  it('renders the welcome heading', () => {
    render(<Home />)

    expect(
      screen.getByRole('heading', { name: /welcome to resonance/i })
    ).toBeInTheDocument()
  })

  it('renders the tagline', () => {
    render(<Home />)

    expect(
      screen.getByText(/your self-hosted music streaming platform/i)
    ).toBeInTheDocument()
  })

  it('renders the Recently Played section', () => {
    render(<Home />)

    expect(
      screen.getByRole('heading', { name: /recently played/i })
    ).toBeInTheDocument()
  })

  it('renders the Made For You section', () => {
    render(<Home />)

    expect(
      screen.getByRole('heading', { name: /made for you/i })
    ).toBeInTheDocument()
  })

  it('renders placeholder cards in Recently Played', () => {
    render(<Home />)

    const albumTitles = screen.getAllByText('Album Title')
    expect(albumTitles.length).toBeGreaterThan(0)
  })

  it('renders Discover Weekly cards', () => {
    render(<Home />)

    const discoverCards = screen.getAllByText('Discover Weekly')
    expect(discoverCards.length).toBeGreaterThan(0)
  })
})
