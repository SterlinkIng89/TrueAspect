import type { ReactElement } from 'react'
import { FolderDown } from 'lucide-react'

interface DragOverlayProps {
  readonly isDragging: boolean
}

export function DragOverlay({ isDragging }: DragOverlayProps): ReactElement | null {
  if (!isDragging) return null

  return (
    <div className="fixed inset-0 z-40 bg-background/90 backdrop-blur-xs flex items-center justify-center p-8 pointer-events-none select-none">
      <div className="w-full h-full border-2 border-dashed border-primary/80 rounded-2xl flex flex-col items-center justify-center bg-primary/5">
        <FolderDown className="w-12 h-12 text-primary stroke-[1.5] mb-3" />
        <h3 className="text-base font-medium text-foreground mb-1">
          Drop folder or screenshots
        </h3>
        <p className="text-xs text-muted-foreground">
          Release to load images into the grid
        </p>
      </div>
    </div>
  )
}
