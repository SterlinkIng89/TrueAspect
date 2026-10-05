import { useState, useCallback } from 'react'
import type { ScreenshotItem } from '../types/screenshot'

export interface UseSelectionReturn {
  readonly selectedPaths: ReadonlySet<string>
  readonly isSelected: (path: string) => boolean
  readonly toggle: (path: string, isShiftKey: boolean, items: readonly ScreenshotItem[]) => void
  readonly selectAll: (items: readonly ScreenshotItem[]) => void
  readonly clear: () => void
  readonly count: number
}

export function useSelection(): UseSelectionReturn {
  const [selectedPaths, setSelectedPaths] = useState<ReadonlySet<string>>(() => new Set())
  const [lastClickedPath, setLastClickedPath] = useState<string | null>(null)

  const isSelected = useCallback((path: string) => {
    return selectedPaths.has(path)
  }, [selectedPaths])

  const toggle = useCallback((path: string, isShiftKey: boolean, items: readonly ScreenshotItem[]) => {
    setSelectedPaths(prev => {
      const next = new Set(prev)

      if (isShiftKey && lastClickedPath && items.length > 0) {
        const lastIndex = items.findIndex(item => item.path === lastClickedPath)
        const currentIndex = items.findIndex(item => item.path === path)

        if (lastIndex !== -1 && currentIndex !== -1) {
          const start = Math.min(lastIndex, currentIndex)
          const end = Math.max(lastIndex, currentIndex)
          const shouldAdd = !prev.has(path)

          for (let i = start; i <= end; i++) {
            const currentItem = items[i]
            if (currentItem) {
              if (shouldAdd) {
                next.add(currentItem.path)
              } else {
                next.delete(currentItem.path)
              }
            }
          }
          return next
        }
      }

      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
      }
      return next
    })

    setLastClickedPath(path)
  }, [lastClickedPath])

  const selectAll = useCallback((items: readonly ScreenshotItem[]) => {
    const next = new Set<string>()
    for (const item of items) {
      next.add(item.path)
    }
    setSelectedPaths(next)
  }, [])

  const clear = useCallback(() => {
    setSelectedPaths(new Set())
    setLastClickedPath(null)
  }, [])

  return {
    selectedPaths,
    isSelected,
    toggle,
    selectAll,
    clear,
    count: selectedPaths.size,
  }
}
