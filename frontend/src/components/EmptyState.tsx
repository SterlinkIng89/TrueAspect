import type { ReactElement } from 'react'
import { FolderUp } from 'lucide-react'

interface EmptyStateProps {
  readonly onPickFolder: () => void
  readonly disabled?: boolean
}

export function EmptyState({ onPickFolder, disabled }: EmptyStateProps): ReactElement {
  return (
    <div className="flex-1 flex flex-col items-center justify-center p-8 select-none">
      <div className="flex flex-col items-center text-center max-w-sm">
        <div className="w-12 h-12 rounded-xl bg-card border border-border flex items-center justify-center mb-4 text-muted-foreground">
          <FolderUp className="w-6 h-6 stroke-[1.5]" />
        </div>
        <h2 className="text-base font-medium text-foreground mb-1">
          Drop screenshots or a folder here
        </h2>
        <p className="text-xs text-muted-foreground mb-6 leading-relaxed">
          Drag and drop your Steam screenshot folder or files anywhere on this window to start cropping.
        </p>
        <button
          type="button"
          onClick={onPickFolder}
          disabled={disabled}
          className="inline-flex items-center gap-2 px-4 py-2 rounded-md text-xs font-medium text-foreground bg-card hover:bg-muted active:bg-muted/70 border border-border transition-colors disabled:opacity-50 disabled:pointer-events-none cursor-pointer"
        >
          Choose folder
        </button>
      </div>
    </div>
  )
}
