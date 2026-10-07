import type { ReactElement } from 'react'
import { CheckCircle2, Copy, SkipForward, AlertCircle } from 'lucide-react'
import type { CropResultItem, OutputMode } from '../types/screenshot'
import { Modal } from './Modal'
import { UiButton } from './UiButton'

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

  const stats = [
    { label: 'Cropped', count: results.filter((r) => r.status === 'cropped').length, icon: CheckCircle2, color: 'text-primary' },
    { label: 'Copied', count: results.filter((r) => r.status === 'copied').length, icon: Copy, color: 'text-muted-foreground' },
    { label: 'Skipped', count: results.filter((r) => r.status === 'skipped').length, icon: SkipForward, color: 'text-muted-foreground' },
  ]
  const errorCount = results.filter((r) => r.status === 'error').length

  return (
    <Modal
      isOpen={true}
      onClose={onClose}
      title="Processing complete"
      subtitle={`Processed ${results.length} screenshots.`}
    >
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
        <p
          className="text-xs text-foreground font-mono truncate"
          title={outputMode === 'replace' ? 'Original files replaced in-place' : outputFolder}
        >
          {outputMode === 'replace' ? 'Replaced original files in-place' : outputFolder}
        </p>
      </div>

      <div className="flex justify-end">
        <UiButton variant="primary" onClick={onClose} className="px-4 py-1.5">
          Done
        </UiButton>
      </div>
    </Modal>
  )
}
