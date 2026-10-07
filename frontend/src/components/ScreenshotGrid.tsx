import { useState, useRef, useEffect, type ReactElement } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import type { ScreenshotItem } from '../types/screenshot'
import { ScreenshotTile } from './ScreenshotTile'

interface ScreenshotGridProps {
  readonly items: readonly ScreenshotItem[]
  readonly isSelected: (path: string) => boolean
  readonly onToggle: (path: string, isShiftKey: boolean) => void
  readonly version?: number
}

const MIN_COL_WIDTH = 210
const GAP = 12

export function ScreenshotGrid({
  items,
  isSelected,
  onToggle,
  version,
}: ScreenshotGridProps): ReactElement {
  const parentRef = useRef<HTMLDivElement>(null)
  const [containerWidth, setContainerWidth] = useState<number>(0)

  useEffect(() => {
    const element = parentRef.current
    if (!element) return

    const updateWidth = () => {
      const computed = window.getComputedStyle(element)
      const paddingX =
        parseFloat(computed.paddingLeft || '0') +
        parseFloat(computed.paddingRight || '0')
      const width = element.clientWidth - paddingX
      if (width > 0) {
        setContainerWidth(width)
      }
    }

    updateWidth()

    const observer = new ResizeObserver(() => {
      updateWidth()
    })
    observer.observe(element)

    return () => observer.disconnect()
  }, [])

  const columns = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COL_WIDTH + GAP))
  )

  const colWidth =
    containerWidth > 0
      ? (containerWidth - (columns - 1) * GAP) / columns
      : MIN_COL_WIDTH

  const rowHeight = Math.round(colWidth * (9 / 16))
  const rowCount = Math.ceil(items.length / columns)

  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => rowHeight,
    gap: GAP,
    overscan: 2,
  })

  useEffect(() => {
    virtualizer.measure()
  }, [virtualizer, columns, rowHeight])

  const virtualRows = virtualizer.getVirtualItems()

  return (
    <div ref={parentRef} className="flex-1 overflow-y-auto p-4">
      <div
        className="relative w-full"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualRows.map((virtualRow) => {
          const startIndex = virtualRow.index * columns
          const rowItems = items.slice(startIndex, startIndex + columns)

          return (
            <div
              key={virtualRow.key}
              data-index={virtualRow.index}
              className="absolute top-0 left-0 w-full grid gap-3"
              style={{
                transform: `translateY(${virtualRow.start}px)`,
                gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                height: `${rowHeight}px`,
              }}
            >
              {rowItems.map((item) => (
                <ScreenshotTile
                  key={item.path}
                  item={item}
                  isSelected={isSelected(item.path)}
                  onToggle={onToggle}
                  version={version}
                />
              ))}
            </div>
          )
        })}
      </div>
    </div>
  )
}

