import { memo, type ReactElement, type KeyboardEvent } from 'react'
import { Check } from 'lucide-react'
import type { ScreenshotItem } from '../types/screenshot'
import { cn } from '../lib/cn'

interface ScreenshotTileProps {
  readonly item: ScreenshotItem
  readonly isSelected: boolean
  readonly onToggle: (path: string, isShiftKey: boolean) => void
}

export const ScreenshotTile = memo(function ScreenshotTile({
  item,
  isSelected,
  onToggle,
}: ScreenshotTileProps): ReactElement {
  const thumbUrl = `/api/thumb?path=${encodeURIComponent(item.path)}`

  const handleClick = (e: React.MouseEvent) => {
    onToggle(item.path, e.shiftKey)
  }

  const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault()
      onToggle(item.path, e.shiftKey)
    }
  }

  return (
    <div
      role="checkbox"
      aria-checked={isSelected}
      tabIndex={0}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      className={cn(
        'group relative aspect-video rounded-lg overflow-hidden border cursor-pointer select-none bg-card transition-colors duration-150 outline-none focus-visible:ring-1 focus-visible:ring-ring',
        isSelected
          ? 'border-primary ring-1 ring-primary'
          : 'border-border/60 hover:border-border'
      )}
    >
      <img
        src={thumbUrl}
        alt={item.name}
        loading="lazy"
        className="w-full h-full object-cover pointer-events-none"
      />

      <div
        className={cn(
          'absolute top-2 left-2 w-5 h-5 rounded flex items-center justify-center transition-colors',
          isSelected
            ? 'bg-primary text-primary-foreground'
            : 'bg-black/50 border border-white/20 text-transparent group-hover:border-white/50'
        )}
      >
        <Check className="w-3.5 h-3.5 stroke-[2.5]" />
      </div>

      <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent p-2 pt-5 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
        <p className="text-[11px] text-white/90 truncate font-mono">
          {item.name}
        </p>
      </div>
    </div>
  )
})
