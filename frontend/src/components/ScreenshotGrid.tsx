import type { ReactElement } from 'react'
import type { ScreenshotItem } from '../types/screenshot'
import { ScreenshotTile } from './ScreenshotTile'

interface ScreenshotGridProps {
  readonly items: readonly ScreenshotItem[]
  readonly isSelected: (path: string) => boolean
  readonly onToggle: (path: string, isShiftKey: boolean) => void
}

export function ScreenshotGrid({
  items,
  isSelected,
  onToggle,
}: ScreenshotGridProps): ReactElement {
  return (
    <div className="flex-1 overflow-y-auto p-4">
      <div className="grid grid-cols-[repeat(auto-fill,minmax(210px,1fr))] gap-3">
        {items.map((item) => (
          <ScreenshotTile
            key={item.path}
            item={item}
            isSelected={isSelected(item.path)}
            onToggle={onToggle}
          />
        ))}
      </div>
    </div>
  )
}
