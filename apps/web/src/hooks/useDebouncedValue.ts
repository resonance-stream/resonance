/**
 * Debounce hook for delaying value updates
 *
 * Useful for search inputs to avoid making API requests on every keystroke.
 */

import { useState, useEffect, useRef, useCallback } from 'react'

/**
 * Returns a debounced version of the provided value
 *
 * The returned value will only update after the specified delay
 * has passed without the input value changing.
 *
 * @param value - The value to debounce
 * @param delay - Delay in milliseconds (default: 300ms)
 * @returns The debounced value
 *
 * @example
 * ```tsx
 * function SearchInput() {
 *   const [query, setQuery] = useState('')
 *   const debouncedQuery = useDebouncedValue(query, 300)
 *
 *   // Use debouncedQuery for API calls
 *   const { data } = useSearch(debouncedQuery)
 *
 *   return (
 *     <input
 *       value={query}
 *       onChange={(e) => setQuery(e.target.value)}
 *       placeholder="Search..."
 *     />
 *   )
 * }
 * ```
 */
export function useDebouncedValue<T>(value: T, delay = 300): T {
  const [debouncedValue, setDebouncedValue] = useState(value)

  useEffect(() => {
    // Set up a timeout to update the debounced value
    const timer = setTimeout(() => {
      setDebouncedValue(value)
    }, delay)

    // Clean up the timeout if value changes before delay completes
    return () => {
      clearTimeout(timer)
    }
  }, [value, delay])

  return debouncedValue
}

/**
 * Debounce callback version - useful when you need to debounce a function
 *
 * Uses useRef for proper cleanup and useCallback for memoization.
 *
 * @param callback - The callback to debounce
 * @param delay - Delay in milliseconds (default: 300ms)
 * @returns A debounced version of the callback
 *
 * @example
 * ```tsx
 * function SearchInput() {
 *   const [query, setQuery] = useState('')
 *
 *   const debouncedSearch = useDebouncedCallback((value: string) => {
 *     // Perform search
 *     searchApi(value)
 *   }, 300)
 *
 *   const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
 *     setQuery(e.target.value)
 *     debouncedSearch(e.target.value)
 *   }
 *
 *   return <input value={query} onChange={handleChange} />
 * }
 * ```
 */
export function useDebouncedCallback<T extends (...args: unknown[]) => void>(
  callback: T,
  delay = 300
): (...args: Parameters<T>) => void {
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const callbackRef = useRef(callback)

  // Keep callback ref up to date
  useEffect(() => {
    callbackRef.current = callback
  }, [callback])

  // Cleanup on unmount only
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [])

  return useCallback(
    (...args: Parameters<T>) => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }

      timeoutRef.current = setTimeout(() => {
        callbackRef.current(...args)
      }, delay)
    },
    [delay]
  )
}
