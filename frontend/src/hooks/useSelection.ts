import { useState, useCallback } from 'react'
import type { ScreenshotItem } from '../types/screenshot'

export function useSelection() {
  const [selectedPaths, setSelected] = useState<ReadonlySet<string>>(() => new Set())
  const [lastPath, setLastPath] = useState<string | null>(null)

  const toggle = useCallback((path: string, isShift: boolean, items: readonly ScreenshotItem[]) => {
    setSelected(prev => {
      const next = new Set(prev)
      if (isShift && lastPath) {
        const i1 = items.findIndex(i => i.path === lastPath)
        const i2 = items.findIndex(i => i.path === path)
        if (i1 !== -1 && i2 !== -1) {
          const [start, end] = [Math.min(i1, i2), Math.max(i1, i2)]
          const add = !prev.has(path)
          for (let i = start; i <= end; i++) add ? next.add(items[i].path) : next.delete(items[i].path)
          return next
        }
      }
      next.has(path) ? next.delete(path) : next.add(path)
      return next
    })
    setLastPath(path)
  }, [lastPath])

  return {
    selectedPaths,
    isSelected: useCallback((path: string) => selectedPaths.has(path), [selectedPaths]),
    toggle,
    selectAll: useCallback((items: readonly ScreenshotItem[]) => setSelected(new Set(items.map(i => i.path))), []),
    clear: useCallback(() => { setSelected(new Set()); setLastPath(null) }, []),
    count: selectedPaths.size,
  }
}
