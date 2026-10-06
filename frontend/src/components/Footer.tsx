import type { ReactElement } from 'react'
import { FolderDown, Scissors, RefreshCw } from 'lucide-react'
import type { AspectRatioOption, OutputMode } from '../types/screenshot'

const RATIO_OPTIONS: readonly AspectRatioOption[] = [
  { label: '16:9 (standard)', value: '16:9' },
  { label: '21:9 (ultrawide)', value: '21:9' },
  { label: '4:3 (retro)', value: '4:3' },
  { label: '16:10 (laptop)', value: '16:10' },
]

interface FooterProps {
  readonly totalCount: number
  readonly selectedCount: number
  readonly ratio: string
  readonly onRatioChange: (ratio: string) => void
  readonly outputFolder: string
  readonly onPickOutputFolder: () => void
  readonly outputMode: OutputMode
  readonly onOutputModeChange: (mode: OutputMode) => void
  readonly onSelectAll: () => void
  readonly onClear: () => void
  readonly onCrop: () => void
  readonly isProcessing: boolean
}

export function Footer({
  totalCount,
  selectedCount,
  ratio,
  onRatioChange,
  outputFolder,
  onPickOutputFolder,
  outputMode,
  onOutputModeChange,
  onSelectAll,
  onClear,
  onCrop,
  isProcessing,
}: FooterProps): ReactElement {
  const hasSelection = selectedCount > 0
  const isAllSelected = totalCount > 0 && selectedCount === totalCount

  return (
    <footer className="h-16 border-t border-border bg-card/90 px-4 flex items-center justify-between shrink-0 select-none">
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={onSelectAll}
          disabled={totalCount === 0 || isAllSelected || isProcessing}
          className="px-2.5 py-1.5 rounded-md text-xs font-medium text-foreground bg-muted hover:bg-muted/80 active:bg-muted/60 border border-border transition-colors disabled:opacity-40 disabled:pointer-events-none cursor-pointer"
        >
          Select all
        </button>
        <button
          type="button"
          onClick={onClear}
          disabled={!hasSelection || isProcessing}
          className="px-2.5 py-1.5 rounded-md text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-muted/60 active:bg-muted/40 transition-colors disabled:opacity-40 disabled:pointer-events-none cursor-pointer"
        >
          Clear
        </button>
      </div>

      <div className="text-xs text-muted-foreground font-mono">
        <span className="text-foreground font-medium">{selectedCount}</span> of{' '}
        <span>{totalCount}</span> selected
      </div>

      <div className="flex items-center gap-3">
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <span>Ratio:</span>
          <select
            value={ratio}
            onChange={(e) => onRatioChange(e.target.value)}
            disabled={isProcessing}
            className="bg-muted text-foreground text-xs rounded-md border border-border px-2 py-1 outline-none focus-visible:ring-1 focus-visible:ring-ring cursor-pointer"
          >
            {RATIO_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value} className="bg-card text-foreground">
                {opt.label}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <span>Mode:</span>
          <select
            value={outputMode}
            onChange={(e) => onOutputModeChange(e.target.value as OutputMode)}
            disabled={isProcessing}
            className="bg-muted text-foreground text-xs rounded-md border border-border px-2 py-1 outline-none focus-visible:ring-1 focus-visible:ring-ring cursor-pointer"
          >
            <option value="directory" className="bg-card text-foreground">
              Save to folder
            </option>
            <option value="replace" className="bg-card text-foreground">
              Replace originals
            </option>
          </select>
        </div>

        {outputMode === 'directory' ? (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <span>Save to:</span>
            <button
              type="button"
              onClick={onPickOutputFolder}
              disabled={isProcessing}
              title={`Destination folder: ${outputFolder || 'None'}\nClick to choose a different folder`}
              className="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs text-foreground bg-muted hover:bg-muted/80 active:bg-muted/60 border border-border rounded-md transition-colors cursor-pointer group"
            >
              <FolderDown className="w-3.5 h-3.5 text-primary shrink-0" />
              <span className="truncate font-mono text-[11px] max-w-[120px]">
                {outputFolder ? outputFolder.split(/[\\/]/).pop() || outputFolder : 'Choose folder'}
              </span>
              <span className="text-[11px] text-primary underline underline-offset-2 shrink-0">
                Change...
              </span>
            </button>
          </div>
        ) : (
          <div
            title="Cropped images will overwrite the original screenshot files directly"
            className="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs text-amber-500/90 bg-amber-500/10 border border-amber-500/20 rounded-md cursor-help"
          >
            <RefreshCw className="w-3.5 h-3.5 shrink-0" />
            <span>Overwrites originals</span>
          </div>
        )}

        <button
          type="button"
          onClick={onCrop}
          disabled={!hasSelection || isProcessing}
          className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-md text-xs font-medium text-primary-foreground bg-primary hover:bg-primary/90 active:bg-primary/80 transition-colors disabled:opacity-40 disabled:pointer-events-none cursor-pointer"
        >
          <Scissors className="w-3.5 h-3.5" />
          {outputMode === 'replace' ? 'Crop & replace' : 'Crop selected'}
        </button>
      </div>
    </footer>
  )
}
