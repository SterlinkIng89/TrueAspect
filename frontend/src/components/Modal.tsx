import type { ReactNode, ReactElement } from 'react'
import { X } from 'lucide-react'
import { cn } from '../lib/cn'

export interface ModalProps {
  readonly isOpen: boolean
  readonly onClose?: () => void
  readonly title?: string
  readonly subtitle?: string
  readonly maxWidth?: 'sm' | 'md'
  readonly children: ReactNode
}

export function Modal({
  isOpen,
  onClose,
  title,
  subtitle,
  maxWidth = 'md',
  children,
}: ModalProps): ReactElement | null {
  if (!isOpen) return null

  return (
    <div className="fixed inset-0 bg-background/80 backdrop-blur-xs z-50 flex items-center justify-center p-4 select-none">
      <div
        className={cn(
          'w-full bg-card border border-border rounded-xl p-6 relative',
          maxWidth === 'sm' ? 'max-w-sm' : 'max-w-md'
        )}
      >
        {onClose && (
          <button
            type="button"
            onClick={onClose}
            className="absolute top-4 right-4 text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        )}

        {title && (
          <h3 className="text-sm font-medium text-foreground mb-1">
            {title}
          </h3>
        )}

        {subtitle && (
          <p className="text-xs text-muted-foreground mb-5">
            {subtitle}
          </p>
        )}

        {children}
      </div>
    </div>
  )
}
