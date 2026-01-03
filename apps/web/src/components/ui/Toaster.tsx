import {
  Toast,
  ToastClose,
  ToastDescription,
  ToastProvider,
  ToastTitle,
  ToastViewport,
  iconVariants,
} from './Toast'
import { useToastStore, type ToastType } from '../../stores/toastStore'

/**
 * Toaster component that renders all active toast notifications.
 * Place this component once at the root of your application (e.g., in App.tsx).
 *
 * Usage:
 * ```tsx
 * // In App.tsx
 * import { Toaster } from './components/ui/Toaster'
 *
 * function App() {
 *   return (
 *     <>
 *       <YourApp />
 *       <Toaster />
 *     </>
 *   )
 * }
 *
 * // Anywhere in your app
 * import { toast } from './stores/toastStore'
 *
 * toast.success('Saved!', 'Your changes have been saved.')
 * toast.error('Error', 'Something went wrong.')
 * toast.info('Info', 'Here is some information.')
 * toast.warning('Warning', 'Please be careful.')
 * ```
 */
export function Toaster(): JSX.Element {
  const toasts = useToastStore((state) => state.toasts)
  const removeToast = useToastStore((state) => state.removeToast)

  return (
    <ToastProvider swipeDirection="right">
      {toasts.map((toast) => {
        const variant = toast.type as ToastType
        const IconConfig = iconVariants[variant]
        const Icon = IconConfig?.icon

        return (
          <Toast
            key={toast.id}
            variant={variant}
            duration={toast.duration}
            onOpenChange={(open) => {
              if (!open) {
                removeToast(toast.id)
              }
            }}
          >
            {Icon && (
              <Icon className={`h-5 w-5 shrink-0 ${IconConfig.className}`} />
            )}
            <div className="flex-1 min-w-0">
              <ToastTitle>{toast.title}</ToastTitle>
              {toast.description && (
                <ToastDescription>{toast.description}</ToastDescription>
              )}
            </div>
            <ToastClose />
          </Toast>
        )
      })}
      <ToastViewport />
    </ToastProvider>
  )
}
