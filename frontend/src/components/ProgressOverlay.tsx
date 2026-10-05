import type { ReactElement } from 'react'
import type { CropProgress } from '../types/screenshot'

interface ProgressOverlayProps {
  readonly progress: CropProgress | null
}

export function ProgressOverlay({ progress }: ProgressOverlayProps): ReactElement | null {
  if (!progress) return null

  const percentage = progress.total > 0
    ? Math.round((progress.current / progress.total) * 100)
    : 0

  return (
    <div className="fixed inset-0 bg-background/80 backdrop-blur-xs z-50 flex items-center justify-center p-4 select-none">
      <div className="w-full max-w-md bg-card border border-border rounded-xl p-6">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-sm font-medium text-foreground">
            Cropping screenshots
          </h3>
          <span className="text-xs text-muted-foreground font-mono">
            {progress.current} of {progress.total} ({percentage}%)
          </span>
        </div>

        <div className="w-full h-2 bg-muted rounded-full overflow-hidden mb-3">
          <div
            className="h-full bg-primary transition-all duration-150"
            style={{ width: `${percentage}%` }}
          />
        </div>

        <p className="text-xs text-muted-foreground truncate font-mono">
          {progress.name || 'Processing...'}
        </p>
      </div>
    </div>
  )
}
