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
  type OutputMode,
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
  const [outputMode, setOutputMode] = useState<OutputMode>('directory')
  const [cacheVersion, setCacheVersion] = useState<number>(1)
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
      clear()

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
  }, [clear])

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
    const setDrag = (active: boolean) => setIsDragging(active)

    const onDragEnter = (e: DragEvent) => {
      e.preventDefault()
      if (++dragCounter === 1 && e.dataTransfer?.types.includes('Files')) setDrag(true)
    }

    const onDragLeave = (e: DragEvent) => {
      e.preventDefault()
      if (--dragCounter <= 0) {
        dragCounter = 0
        setDrag(false)
      }
    }

    const onDragOver = (e: DragEvent) => e.preventDefault()
    const onDrop = (e: DragEvent) => {
      e.preventDefault()
      dragCounter = 0
      setDrag(false)
    }

    window.addEventListener('dragenter', onDragEnter)
    window.addEventListener('dragleave', onDragLeave)
    window.addEventListener('dragover', onDragOver)
    window.addEventListener('drop', onDrop)

    return () => {
      window.removeEventListener('dragenter', onDragEnter)
      window.removeEventListener('dragleave', onDragLeave)
      window.removeEventListener('dragover', onDragOver)
      window.removeEventListener('drop', onDrop)
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
      const selected = await pickOutputFolder(outputFolder)
      if (selected) {
        setOutputFolder(selected)
      }
    } catch (err) {
      console.error('Pick output folder failed:', err)
    }
  }, [outputFolder])

  const handleCrop = useCallback(async () => {
    const selectedList = items
      .filter((item) => selectedPaths.has(item.path))
      .map((item) => item.path)

    if (selectedList.length === 0) return

    setProgress({ current: 0, total: selectedList.length, name: 'Starting...' })
    try {
      const rawResults = await cropScreenshots(selectedList, ratio, outputFolder, outputMode)
      const mappedResults: CropResultItem[] = rawResults.map((r) => ({
        path: r.path,
        status: isCropStatus(r.status) ? r.status : 'error',
        message: r.message,
      }))
      setSummaryResults(mappedResults)
      if (outputMode === 'replace') {
        setCacheVersion((v) => v + 1)
      }
    } catch (err) {
      console.error('Crop failed:', err)
    } finally {
      setProgress(null)
    }
  }, [items, selectedPaths, ratio, outputFolder, outputMode])

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
          version={cacheVersion}
        />
      )}

      <Footer
        totalCount={items.length}
        selectedCount={count}
        ratio={ratio}
        onRatioChange={setRatio}
        outputFolder={outputFolder}
        onPickOutputFolder={handlePickOutputFolder}
        outputMode={outputMode}
        onOutputModeChange={setOutputMode}
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
        outputMode={outputMode}
        onClose={() => setSummaryResults(null)}
      />
    </div>
  )
}
