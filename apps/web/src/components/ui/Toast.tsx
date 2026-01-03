import * as ToastPrimitive from '@radix-ui/react-toast'
import { cva, type VariantProps } from 'class-variance-authority'
import { X, CheckCircle, AlertCircle, Info, AlertTriangle } from 'lucide-react'
import { forwardRef, type ComponentPropsWithoutRef, type ElementRef } from 'react'
import { cn } from '../../lib/utils'

const ToastProvider = ToastPrimitive.Provider

const ToastViewport = forwardRef<
  ElementRef<typeof ToastPrimitive.Viewport>,
  ComponentPropsWithoutRef<typeof ToastPrimitive.Viewport>
>(({ className, ...props }, ref) => (
  <ToastPrimitive.Viewport
    ref={ref}
    className={cn(
      'fixed bottom-0 right-0 z-[100] flex max-h-screen w-full flex-col-reverse gap-2 p-4 sm:max-w-[420px]',
      className
    )}
    {...props}
  />
))
ToastViewport.displayName = ToastPrimitive.Viewport.displayName

const toastVariants = cva(
  cn(
    'group pointer-events-auto relative flex w-full items-start gap-3 overflow-hidden rounded-lg p-4 shadow-lg',
    'bg-background-secondary/95 backdrop-blur-xl border border-white/10',
    'data-[state=open]:animate-in data-[state=closed]:animate-out',
    'data-[swipe=end]:animate-out data-[state=closed]:fade-out-80',
    'data-[state=closed]:slide-out-to-right-full data-[state=open]:slide-in-from-bottom-full',
    'data-[swipe=cancel]:translate-x-0 data-[swipe=end]:translate-x-[var(--radix-toast-swipe-end-x)]',
    'data-[swipe=move]:translate-x-[var(--radix-toast-swipe-move-x)] data-[swipe=move]:transition-none',
    'transition-all duration-200'
  ),
  {
    variants: {
      variant: {
        success: 'border-l-4 border-l-mint',
        error: 'border-l-4 border-l-[#C47D8F]',
        info: 'border-l-4 border-l-navy',
        warning: 'border-l-4 border-l-[#C4A66B]',
      },
    },
    defaultVariants: {
      variant: 'info',
    },
  }
)

const iconVariants: Record<string, { icon: typeof CheckCircle; className: string }> = {
  success: { icon: CheckCircle, className: 'text-mint' },
  error: { icon: AlertCircle, className: 'text-[#C47D8F]' },
  info: { icon: Info, className: 'text-navy' },
  warning: { icon: AlertTriangle, className: 'text-[#C4A66B]' },
}

interface ToastProps
  extends ComponentPropsWithoutRef<typeof ToastPrimitive.Root>,
    VariantProps<typeof toastVariants> {}

const Toast = forwardRef<ElementRef<typeof ToastPrimitive.Root>, ToastProps>(
  ({ className, variant, ...props }, ref) => {
    return (
      <ToastPrimitive.Root
        ref={ref}
        className={cn(toastVariants({ variant }), className)}
        {...props}
      />
    )
  }
)
Toast.displayName = ToastPrimitive.Root.displayName

const ToastAction = forwardRef<
  ElementRef<typeof ToastPrimitive.Action>,
  ComponentPropsWithoutRef<typeof ToastPrimitive.Action>
>(({ className, ...props }, ref) => (
  <ToastPrimitive.Action
    ref={ref}
    className={cn(
      'inline-flex h-8 shrink-0 items-center justify-center rounded-md border border-white/10 bg-transparent px-3',
      'text-sm font-medium text-text-secondary',
      'hover:bg-background-tertiary hover:text-text-primary',
      'focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow',
      'disabled:pointer-events-none disabled:opacity-50',
      'transition-colors duration-150',
      className
    )}
    {...props}
  />
))
ToastAction.displayName = ToastPrimitive.Action.displayName

const ToastClose = forwardRef<
  ElementRef<typeof ToastPrimitive.Close>,
  ComponentPropsWithoutRef<typeof ToastPrimitive.Close>
>(({ className, ...props }, ref) => (
  <ToastPrimitive.Close
    ref={ref}
    className={cn(
      'absolute right-2 top-2 rounded-sm p-1 opacity-0 transition-opacity',
      'group-hover:opacity-100',
      'hover:bg-background-tertiary',
      'focus:outline-none focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-accent-glow',
      'text-text-muted hover:text-text-secondary',
      className
    )}
    toast-close=""
    {...props}
  >
    <X className="h-4 w-4" />
    <span className="sr-only">Close</span>
  </ToastPrimitive.Close>
))
ToastClose.displayName = ToastPrimitive.Close.displayName

const ToastTitle = forwardRef<
  ElementRef<typeof ToastPrimitive.Title>,
  ComponentPropsWithoutRef<typeof ToastPrimitive.Title>
>(({ className, ...props }, ref) => (
  <ToastPrimitive.Title
    ref={ref}
    className={cn('text-sm font-semibold text-text-primary', className)}
    {...props}
  />
))
ToastTitle.displayName = ToastPrimitive.Title.displayName

const ToastDescription = forwardRef<
  ElementRef<typeof ToastPrimitive.Description>,
  ComponentPropsWithoutRef<typeof ToastPrimitive.Description>
>(({ className, ...props }, ref) => (
  <ToastPrimitive.Description
    ref={ref}
    className={cn('text-sm text-text-secondary', className)}
    {...props}
  />
))
ToastDescription.displayName = ToastPrimitive.Description.displayName

export {
  Toast,
  ToastAction,
  ToastClose,
  ToastDescription,
  ToastProvider,
  ToastTitle,
  ToastViewport,
  iconVariants,
  type ToastProps,
}
