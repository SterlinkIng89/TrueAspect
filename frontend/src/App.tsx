import { useState, useEffect, useCallback, type ReactElement } from 'react'
import { Header } from './components/Header'
import { EmptyState } from './components/EmptyState'
import { ScreenshotGrid } from './components/ScreenshotGrid'
import { Footer } from './components/Footer'
import { ProgressOverlay } from './components/ProgressOverlay'
import { SummaryDialog } from './components/SummaryDialog'
import { DragOverlay } from './components/DragOverlay'
import { useSelection } from './hooks/useSelection'
import {
  type ScreenshotItem,
  type CropResultItem,
  type CropProgress,
  isCropStatus,
} from './types/screenshot'
import {
  getDefaultOutputDir,
  pickFolder,
  pickOutputFolder,
  loadScreenshots,
  cropScreenshots,
  onCropProgress,
  onDragDropFiles,
} from './lib/tauriApi'

function getParentDirectory(filePath: string): string {
  const lastIndex = Math.max(filePath.lastIndexOf('\\'), filePath.lastIndexOf('/'))
  return lastIndex > 0 ? filePath.substring(0, lastIndex) : filePath
}

export default function App(): ReactElement {
  const [items, setItems] = useState<readonly ScreenshotItem[]>([])
  const [currentFolder, setCurrentFolder] = useState<string | null>(null)
  const [outputFolder, setOutputFolder] = useState<string>('')
  const [ratio, setRatio] = useState<string>('16:9')
  const [isDragging, setIsDragging] = useState<boolean>(false)
  const [progress, setProgress] = useState<CropProgress | null>(null)
  const [summaryResults, setSummaryResults] = useState<readonly CropResultItem[] | null>(null)
  const [isLoading, setIsLoading] = useState<boolean>(false)

  const { selectedPaths, isSelected, toggle, selectAll, clear, count } = useSelection()

  useEffect(() => {
    getDefaultOutputDir()
      .then((dir) => setOutputFolder(dir))
      .catch(() => setOutputFolder('~/Pictures/steam-cropped'))
  }, [])

  const handleLoadPaths = useCallback(async (paths: readonly string[]) => {
    if (paths.length === 0) return
    setIsLoading(true)
    try {
      const loaded = await loadScreenshots(paths)
      const mapped: ScreenshotItem[] = loaded.map((s) => ({
        path: s.path,
        name: s.name,
      }))
      setItems(mapped)
      selectAll(mapped)

      if (paths.length === 1 && paths[0]) {
        setCurrentFolder(paths[0])
      } else if (mapped.length > 0 && mapped[0]) {
        setCurrentFolder(getParentDirectory(mapped[0].path))
      }
    } catch (err) {
      console.error('Failed to load screenshots:', err)
    } finally {
      setIsLoading(false)
    }
  }, [selectAll])

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined
    let unlistenDragDrop: (() => void) | undefined

    onCropProgress((data: CropProgress) => {
      setProgress(data)
    }).then((unlisten) => {
      unlistenProgress = unlisten
    })

    onDragDropFiles((paths: readonly string[]) => {
      setIsDragging(false)
      handleLoadPaths(paths)
    }).then((unlisten) => {
      unlistenDragDrop = unlisten
    })

    return () => {
      if (unlistenProgress) unlistenProgress()
      if (unlistenDragDrop) unlistenDragDrop()
    }
  }, [handleLoadPaths])

  useEffect(() => {
    let dragCounter = 0

    const handleDragEnter = (e: DragEvent) => {
      e.preventDefault()
      dragCounter++
      if (e.dataTransfer && e.dataTransfer.types.includes('Files')) {
        setIsDragging(true)
      }
    }

    const handleDragLeave = (e: DragEvent) => {
      e.preventDefault()
      dragCounter--
      if (dragCounter <= 0) {
        dragCounter = 0
        setIsDragging(false)
      }
    }

    const handleDragOver = (e: DragEvent) => {
      e.preventDefault()
    }

    const handleDrop = (e: DragEvent) => {
      e.preventDefault()
      dragCounter = 0
      setIsDragging(false)
    }

    window.addEventListener('dragenter', handleDragEnter)
    window.addEventListener('dragleave', handleDragLeave)
    window.addEventListener('dragover', handleDragOver)
    window.addEventListener('drop', handleDrop)

    return () => {
      window.removeEventListener('dragenter', handleDragEnter)
      window.removeEventListener('dragleave', handleDragLeave)
      window.removeEventListener('dragover', handleDragOver)
      window.removeEventListener('drop', handleDrop)
    }
  }, [])

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') {
        const target = e.target as HTMLElement
        if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
          e.preventDefault()
          if (items.length > 0) {
            selectAll(items)
          }
        }
      } else if (e.key === 'Escape') {
        if (summaryResults) {
          setSummaryResults(null)
        } else if (count > 0) {
          clear()
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [items, selectAll, clear, count, summaryResults])

  const handlePickFolder = useCallback(async () => {
    try {
      const selected = await pickFolder()
      if (selected) {
        await handleLoadPaths([selected])
      }
    } catch (err) {
      console.error('Pick folder failed:', err)
    }
  }, [handleLoadPaths])

  const handlePickOutputFolder = useCallback(async () => {
    try {
      const selected = await pickOutputFolder()
      if (selected) {
        setOutputFolder(selected)
      }
    } catch (err) {
      console.error('Pick output folder failed:', err)
    }
  }, [])

  const handleCrop = useCallback(async () => {
    const selectedList = items
      .filter((item) => selectedPaths.has(item.path))
      .map((item) => item.path)

    if (selectedList.length === 0) return

    setProgress({ current: 0, total: selectedList.length, name: 'Starting...' })
    try {
      const rawResults = await cropScreenshots(selectedList, ratio, outputFolder)
      const mappedResults: CropResultItem[] = rawResults.map((r) => ({
        path: r.path,
        status: isCropStatus(r.status) ? r.status : 'error',
        message: r.message,
      }))
      setSummaryResults(mappedResults)
    } catch (err) {
      console.error('Crop failed:', err)
    } finally {
      setProgress(null)
    }
  }, [items, selectedPaths, ratio, outputFolder])

  const isProcessing = progress !== null || isLoading

  return (
    <div className="flex flex-col h-screen w-screen bg-background text-foreground overflow-hidden select-none">
      <Header
        currentFolder={currentFolder}
        onPickFolder={handlePickFolder}
        disabled={isProcessing}
      />

      {items.length === 0 ? (
        <EmptyState onPickFolder={handlePickFolder} disabled={isProcessing} />
      ) : (
        <ScreenshotGrid
          items={items}
          isSelected={isSelected}
          onToggle={(path, isShift) => toggle(path, isShift, items)}
        />
      )}

      <Footer
        totalCount={items.length}
        selectedCount={count}
        ratio={ratio}
        onRatioChange={setRatio}
        outputFolder={outputFolder}
        onPickOutputFolder={handlePickOutputFolder}
        onSelectAll={() => selectAll(items)}
        onClear={clear}
        onCrop={handleCrop}
        isProcessing={isProcessing}
      />

      <DragOverlay isDragging={isDragging} />
      <ProgressOverlay progress={progress} />
      <SummaryDialog
        results={summaryResults}
        outputFolder={outputFolder}
        onClose={() => setSummaryResults(null)}
      />
    </div>
  )
}
