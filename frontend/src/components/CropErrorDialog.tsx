import type { ReactElement } from 'react'
import { AlertCircle, X } from 'lucide-react'

interface CropErrorDialogProps {
  readonly message: string | null
  readonly isProcessing: boolean
  readonly onDismiss: () => void
  readonly onRetry: () => void
}

export function CropErrorDialog({
  message,
  isProcessing,
  onDismiss,
  onRetry,
}: CropErrorDialogProps): ReactElement | null {
  if (message === null) return null

  return (
    <div
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="crop-error-title"
      aria-describedby="crop-error-message"
      className="fixed inset-0 bg-background/80 backdrop-blur-xs z-50 flex items-center justify-center p-4 select-none"
    >
      <div className="w-full max-w-md bg-card border border-destructive/30 rounded-xl p-6 relative">
        <button
          type="button"
          onClick={onDismiss}
          aria-label="Dismiss error"
          className="absolute top-4 right-4 text-muted-foreground hover:text-foreground transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-sm"
        >
          <X className="w-4 h-4" />
        </button>

        <div className="flex items-start gap-3 mb-4">
          <div className="w-9 h-9 rounded-lg bg-destructive/10 border border-destructive/20 flex items-center justify-center shrink-0 text-destructive">
            <AlertCircle className="w-5 h-5" />
          </div>
          <div className="min-w-0">
            <h3 id="crop-error-title" className="text-sm font-medium text-foreground mb-1">
              Crop operation failed
            </h3>
            <p id="crop-error-message" className="text-xs text-muted-foreground leading-relaxed break-words select-text">
              {message}
            </p>
          </div>
        </div>

        <div className="flex items-center justify-end gap-2">
          <button
            type="button"
            onClick={onDismiss}
            className="px-3 py-1.5 rounded-md text-xs font-medium text-muted-foreground hover:text-foreground bg-muted/60 hover:bg-muted border border-border transition-colors cursor-pointer"
          >
            Dismiss
          </button>
          <button
            type="button"
            onClick={onRetry}
            disabled={isProcessing}
            className="px-4 py-1.5 rounded-md text-xs font-medium text-primary-foreground bg-primary hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:pointer-events-none cursor-pointer"
          >
            Retry
          </button>
        </div>
      </div>
    </div>
  )
}
