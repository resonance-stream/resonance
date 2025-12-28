/* eslint-disable react-refresh/only-export-components */
/**
 * Test Utilities
 *
 * Custom render function that wraps components with necessary providers.
 * Re-exports everything from @testing-library/react for convenience.
 */

import { ReactElement, ReactNode } from 'react'
import { render, RenderOptions } from '@testing-library/react'
import { BrowserRouter } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

// Create a fresh QueryClient for each test to avoid cache pollution
function createTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
        staleTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  })
}

interface AllProvidersProps {
  children: ReactNode
}

function AllProviders({ children }: AllProvidersProps): ReactElement {
  const queryClient = createTestQueryClient()

  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>{children}</BrowserRouter>
    </QueryClientProvider>
  )
}

/**
 * Custom render function that wraps the component with all necessary providers.
 * Use this instead of @testing-library/react's render for components that need routing or queries.
 */
function customRender(
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>
): ReturnType<typeof render> {
  return render(ui, { wrapper: AllProviders, ...options })
}

// Re-export everything from testing-library
export * from '@testing-library/react'
export { default as userEvent } from '@testing-library/user-event'

// Export our custom render as the default render
export { customRender as render }

// Export the providers wrapper for special cases
export { AllProviders }

/**
 * Helper to wait for loading states to resolve
 */
export async function waitForLoadingToFinish(): Promise<void> {
  const { waitFor, screen } = await import('@testing-library/react')

  await waitFor(
    () => {
      const loaders = screen.queryAllByText(/loading/i)
      if (loaders.length > 0) {
        throw new Error('Still loading')
      }
    },
    { timeout: 5000 }
  )
}
