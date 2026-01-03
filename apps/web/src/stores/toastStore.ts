/**
 * Toast notification store for Resonance
 *
 * Manages toast notifications state including:
 * - Toast queue management
 * - Adding/removing toasts
 * - Auto-dismiss with configurable duration
 */

import { create } from 'zustand'

export type ToastType = 'success' | 'error' | 'info' | 'warning'

export interface Toast {
  id: string
  type: ToastType
  title: string
  description?: string
  duration?: number
}

interface ToastState {
  // State
  toasts: Toast[]

  // Actions
  addToast: (toast: Omit<Toast, 'id'>) => string
  removeToast: (id: string) => void
  clearToasts: () => void
}

/**
 * Default durations per toast type (in milliseconds)
 */
const DEFAULT_DURATION: Record<ToastType, number> = {
  success: 3000,
  info: 4000,
  warning: 5000,
  error: 6000,
}

/**
 * Generate a unique toast ID
 */
function generateId(): string {
  return `toast-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

export const useToastStore = create<ToastState>()((set) => ({
  // Initial state
  toasts: [],

  /**
   * Add a new toast notification
   * Returns the toast ID for programmatic removal if needed
   */
  addToast: (toast) => {
    const id = generateId()
    const duration = toast.duration ?? DEFAULT_DURATION[toast.type]

    set((state) => ({
      toasts: [
        ...state.toasts,
        {
          ...toast,
          id,
          duration,
        },
      ],
    }))

    return id
  },

  /**
   * Remove a toast by ID
   */
  removeToast: (id) => {
    set((state) => ({
      toasts: state.toasts.filter((toast) => toast.id !== id),
    }))
  },

  /**
   * Clear all toasts
   */
  clearToasts: () => {
    set({ toasts: [] })
  },
}))

/**
 * Convenience functions for adding toasts of specific types
 */
export const toast = {
  success: (title: string, description?: string, duration?: number): string => {
    return useToastStore.getState().addToast({ type: 'success', title, description, duration })
  },
  error: (title: string, description?: string, duration?: number): string => {
    return useToastStore.getState().addToast({ type: 'error', title, description, duration })
  },
  info: (title: string, description?: string, duration?: number): string => {
    return useToastStore.getState().addToast({ type: 'info', title, description, duration })
  },
  warning: (title: string, description?: string, duration?: number): string => {
    return useToastStore.getState().addToast({ type: 'warning', title, description, duration })
  },
}
