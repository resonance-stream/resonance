/**
 * Search Page Tests
 *
 * Tests for the Search page component.
 */

import { describe, it, expect } from 'vitest'
import { render, screen, userEvent } from '@/test/test-utils'
import Search from './Search'

describe('Search', () => {
  it('renders the search heading', () => {
    render(<Search />)

    expect(screen.getByRole('heading', { name: /search/i })).toBeInTheDocument()
  })

  it('renders the search input', () => {
    render(<Search />)

    const searchInput = screen.getByPlaceholderText(
      /search for songs, albums, or artists/i
    )
    expect(searchInput).toBeInTheDocument()
  })

  it('allows typing in the search input', async () => {
    const user = userEvent.setup()
    render(<Search />)

    const searchInput = screen.getByPlaceholderText(
      /search for songs, albums, or artists/i
    )

    await user.type(searchInput, 'test query')

    expect(searchInput).toHaveValue('test query')
  })

  it('search input has correct type attribute', () => {
    render(<Search />)

    const searchInput = screen.getByPlaceholderText(
      /search for songs, albums, or artists/i
    )
    expect(searchInput).toHaveAttribute('type', 'text')
  })
})
