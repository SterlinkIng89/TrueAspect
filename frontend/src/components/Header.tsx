import type { ReactElement } from 'react'
import { FolderOpen } from 'lucide-react'
import { UiButton } from './UiButton'

interface HeaderProps {
  readonly currentFolder: string | null
  readonly onPickFolder: () => void
  readonly disabled?: boolean
}

export function Header({ currentFolder, onPickFolder, disabled }: HeaderProps): ReactElement {
  return (
    <header className="h-14 border-b border-border bg-card/80 px-4 flex items-center justify-between shrink-0 select-none">
      <div className="flex items-center gap-2.5">
        <div className="w-2.5 h-2.5 rounded-full bg-primary" />
        <h1 className="text-sm font-medium tracking-tight text-foreground">
          Steam screenshot cropper
        </h1>
      </div>

      <div className="flex items-center gap-3">
        {currentFolder && (
          <span
            className="text-xs text-muted-foreground truncate max-w-[320px] font-mono bg-muted/60 px-2.5 py-1 rounded"
            title={currentFolder}
          >
            {currentFolder}
          </span>
        )}
        <UiButton
          onClick={onPickFolder}
          disabled={disabled}
          title="Select folder with screenshots to crop"
        >
          <FolderOpen className="w-3.5 h-3.5 text-muted-foreground" />
          Open screenshots
        </UiButton>
      </div>
    </header>
  )
}
