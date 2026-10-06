import type { ReactElement } from 'react'
import { CheckCircle2, Copy, SkipForward, AlertCircle, X } from 'lucide-react'
import type { CropResultItem, OutputMode } from '../types/screenshot'

interface SummaryDialogProps {
  readonly results: readonly CropResultItem[] | null
  readonly outputFolder: string
  readonly outputMode?: OutputMode
  readonly onClose: () => void
}

export function SummaryDialog({
  results,
  outputFolder,
  outputMode = 'directory',
  onClose,
}: SummaryDialogProps): ReactElement | null {
  if (!results || results.length === 0) return null

  const croppedCount = results.filter((r) => r.status === 'cropped').length
  const copiedCount = results.filter((r) => r.status === 'copied').length
  const skippedCount = results.filter((r) => r.status === 'skipped').length
  const errorCount = results.filter((r) => r.status === 'error').length

  return (
    <div className="fixed inset-0 bg-background/80 backdrop-blur-xs z-50 flex items-center justify-center p-4 select-none">
      <div className="w-full max-w-md bg-card border border-border rounded-xl p-6 relative">
        <button
          type="button"
          onClick={onClose}
          className="absolute top-4 right-4 text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
        >
          <X className="w-4 h-4" />
        </button>

        <h3 className="text-sm font-medium text-foreground mb-1">
          Processing complete
        </h3>
        <p className="text-xs text-muted-foreground mb-5">
          Processed {results.length} screenshots.
        </p>

        <div className="grid grid-cols-2 gap-2.5 mb-5">
          <div className="bg-muted/50 border border-border/60 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1">
              <CheckCircle2 className="w-4 h-4 text-primary" />
              <span className="text-xs font-medium text-foreground">Cropped</span>
            </div>
            <span className="text-lg font-semibold text-foreground">{croppedCount}</span>
          </div>

          <div className="bg-muted/50 border border-border/60 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1">
              <Copy className="w-4 h-4 text-muted-foreground" />
              <span className="text-xs font-medium text-foreground">Copied</span>
            </div>
            <span className="text-lg font-semibold text-foreground">{copiedCount}</span>
          </div>

          <div className="bg-muted/50 border border-border/60 rounded-lg p-3">
            <div className="flex items-center gap-2 mb-1">
              <SkipForward className="w-4 h-4 text-muted-foreground" />
              <span className="text-xs font-medium text-foreground">Skipped</span>
            </div>
            <span className="text-lg font-semibold text-foreground">{skippedCount}</span>
          </div>

          {errorCount > 0 && (
            <div className="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
              <div className="flex items-center gap-2 mb-1">
                <AlertCircle className="w-4 h-4 text-destructive" />
                <span className="text-xs font-medium text-destructive">Errors</span>
              </div>
              <span className="text-lg font-semibold text-destructive">{errorCount}</span>
            </div>
          )}
        </div>

        <div className="bg-muted/30 border border-border/50 rounded-lg p-3 mb-5">
          <p className="text-[11px] text-muted-foreground mb-0.5">
            {outputMode === 'replace' ? 'Destination:' : 'Saved to:'}
          </p>
          <p
            className="text-xs text-foreground font-mono truncate"
            title={outputMode === 'replace' ? 'Original files replaced in-place' : outputFolder}
          >
            {outputMode === 'replace' ? 'Replaced original files in-place' : outputFolder}
          </p>
        </div>

        <div className="flex justify-end">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-1.5 rounded-md text-xs font-medium text-primary-foreground bg-primary hover:bg-primary/90 transition-colors cursor-pointer"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  )
}
