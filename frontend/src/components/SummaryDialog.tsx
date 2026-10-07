import type { ReactElement } from 'react'
import { CheckCircle2, Copy, SkipForward, AlertCircle, X, FolderOpen, ExternalLink } from 'lucide-react'
import type { CropResultItem, OutputMode } from '../types/screenshot'

interface SummaryDialogProps {
  readonly results: readonly CropResultItem[] | null
  readonly outputFolder: string
  readonly outputMode?: OutputMode
  readonly onClose: () => void
  readonly onViewCropped?: () => void
  readonly onOpenFolder?: () => void
}

export function SummaryDialog({
  results,
  outputFolder,
  outputMode = 'directory',
  onClose,
  onViewCropped,
  onOpenFolder,
}: SummaryDialogProps): ReactElement | null {
  if (!results || results.length === 0) return null

  const stats = [
    { label: 'Cropped', count: results.filter((r) => r.status === 'cropped').length, icon: CheckCircle2, color: 'text-primary' },
    { label: 'Copied', count: results.filter((r) => r.status === 'copied').length, icon: Copy, color: 'text-muted-foreground' },
    { label: 'Skipped', count: results.filter((r) => r.status === 'skipped').length, icon: SkipForward, color: 'text-muted-foreground' },
  ]
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

        <h3 className="text-sm font-medium text-foreground mb-1">Processing complete</h3>
        <p className="text-xs text-muted-foreground mb-5">Processed {results.length} screenshots.</p>

        <div className="grid grid-cols-2 gap-2.5 mb-5">
          {stats.map(({ label, count, icon: Icon, color }) => (
            <div key={label} className="bg-muted/50 border border-border/60 rounded-lg p-3">
              <div className="flex items-center gap-2 mb-1">
                <Icon className={`w-4 h-4 ${color}`} />
                <span className="text-xs font-medium text-foreground">{label}</span>
              </div>
              <span className="text-lg font-semibold text-foreground">{count}</span>
            </div>
          ))}

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
          <p className="text-xs text-foreground font-mono truncate" title={outputMode === 'replace' ? 'Original files replaced' : outputFolder}>
            {outputMode === 'replace' ? 'Replaced original files in-place' : outputFolder}
          </p>
        </div>

        <div className="flex items-center justify-between gap-2">
          {outputMode === 'directory' && (
            <div className="flex items-center gap-2">
              {onViewCropped && (
                <button
                  type="button"
                  onClick={onViewCropped}
                  className="px-3 py-1.5 rounded-md text-xs font-medium text-foreground bg-muted hover:bg-muted/80 border border-border transition-colors cursor-pointer inline-flex items-center gap-1.5"
                >
                  <FolderOpen className="w-3.5 h-3.5 text-primary" />
                  <span>View in app</span>
                </button>
              )}
              {onOpenFolder && (
                <button
                  type="button"
                  onClick={onOpenFolder}
                  className="px-3 py-1.5 rounded-md text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-muted/60 border border-border/60 transition-colors cursor-pointer inline-flex items-center gap-1.5"
                >
                  <ExternalLink className="w-3.5 h-3.5" />
                  <span>Open folder</span>
                </button>
              )}
            </div>
          )}
          <div className="ml-auto">
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
    </div>
  )
}
