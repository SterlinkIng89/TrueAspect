import { useState, useEffect, useCallback, type ReactElement } from 'react'
import { FolderUp, FolderDown } from 'lucide-react'
import { Header } from './components/Header'
import { ScreenshotGrid } from './components/ScreenshotGrid'
import { Footer } from './components/Footer'
import { SummaryDialog } from './components/SummaryDialog'
import { useSelection } from './hooks/useSelection'
import type { ScreenshotItem, CropResultItem, CropProgress, OutputMode } from './types/screenshot'
import {
  getDefaultOutputDir,
  pickFolder,
  pickOutputFolder,
  loadScreenshots,
  cropScreenshots,
  onCropProgress,
  onDragDropFiles,
  openFolder,
} from './lib/tauriApi'

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
    getDefaultOutputDir().then(setOutputFolder).catch(() => setOutputFolder('~/Pictures/steam-cropped'))
  }, [])

  const handleLoadPaths = useCallback(async (paths: readonly string[]) => {
    if (paths.length === 0) return
    setIsLoading(true)
    try {
      const loaded = await loadScreenshots(paths)
      setItems(loaded.map((s) => ({ path: s.path, name: s.name })))
      clear()
      if (paths.length === 1 && paths[0]) setCurrentFolder(paths[0])
      else if (loaded.length > 0 && loaded[0]) {
        const p = loaded[0].path
        const i = Math.max(p.lastIndexOf('\\'), p.lastIndexOf('/'))
        setCurrentFolder(i > 0 ? p.substring(0, i) : p)
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
    onCropProgress((data) => setProgress(data)).then((u) => { unlistenProgress = u })
    onDragDropFiles((paths) => { setIsDragging(false); handleLoadPaths(paths) }).then((u) => { unlistenDragDrop = u })
    return () => {
      if (unlistenProgress) unlistenProgress()
      if (unlistenDragDrop) unlistenDragDrop()
    }
  }, [handleLoadPaths])

  useEffect(() => {
    let dragCounter = 0
    const onEnter = (e: DragEvent) => {
      e.preventDefault()
      if (++dragCounter === 1 && e.dataTransfer?.types.includes('Files')) setIsDragging(true)
    }
    const onLeave = (e: DragEvent) => {
      e.preventDefault()
      if (--dragCounter <= 0) { dragCounter = 0; setIsDragging(false) }
    }
    const onOver = (e: DragEvent) => e.preventDefault()
    const onDrop = (e: DragEvent) => { e.preventDefault(); dragCounter = 0; setIsDragging(false) }

    window.addEventListener('dragenter', onEnter)
    window.addEventListener('dragleave', onLeave)
    window.addEventListener('dragover', onOver)
    window.addEventListener('drop', onDrop)
    return () => {
      window.removeEventListener('dragenter', onEnter)
      window.removeEventListener('dragleave', onLeave)
      window.removeEventListener('dragover', onOver)
      window.removeEventListener('drop', onDrop)
    }
  }, [])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const isInput = ['INPUT', 'TEXTAREA'].includes((e.target as HTMLElement).tagName)
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a' && !isInput && items.length > 0) {
        e.preventDefault()
        selectAll(items)
      } else if (e.key === 'Escape') {
        summaryResults ? setSummaryResults(null) : count > 0 && clear()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [items, selectAll, clear, count, summaryResults])

  const handlePickFolder = useCallback(async () => {
    const selected = await pickFolder().catch(() => null)
    if (selected) await handleLoadPaths([selected])
  }, [handleLoadPaths])

  const handlePickOutputFolder = useCallback(async () => {
    const selected = await pickOutputFolder(outputFolder).catch(() => null)
    if (selected) setOutputFolder(selected)
  }, [outputFolder])

  const handleCrop = useCallback(async () => {
    const selectedList = items.filter((item) => selectedPaths.has(item.path)).map((item) => item.path)
    if (selectedList.length === 0) return

    setProgress({ current: 0, total: selectedList.length, name: 'Starting...' })
    try {
      const raw = await cropScreenshots(selectedList, ratio, outputFolder, outputMode)
      setSummaryResults(raw)
      if (outputMode === 'replace') setCacheVersion(Date.now())
    } catch (err) {
      console.error('Crop failed:', err)
    } finally {
      setProgress(null)
    }
  }, [items, selectedPaths, ratio, outputFolder, outputMode])

  const isProcessing = progress !== null || isLoading
  const progressPct = progress && progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0

  return (
    <div className="flex flex-col h-screen w-screen bg-background text-foreground overflow-hidden select-none">
      <Header currentFolder={currentFolder} onPickFolder={handlePickFolder} disabled={isProcessing} />

      {items.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center p-8 select-none">
          <div className="flex flex-col items-center text-center max-w-sm">
            <div className="w-12 h-12 rounded-xl bg-card border border-border flex items-center justify-center mb-4 text-muted-foreground">
              <FolderUp className="w-6 h-6 stroke-[1.5]" />
            </div>
            <h2 className="text-base font-medium text-foreground mb-1">Drop screenshots or a folder here</h2>
            <p className="text-xs text-muted-foreground mb-6 leading-relaxed">
              Drag and drop your Steam screenshot folder or files anywhere on this window to start cropping.
            </p>
            <button
              type="button"
              onClick={handlePickFolder}
              disabled={isProcessing}
              className="px-4 py-2 rounded-md text-xs font-medium text-foreground bg-card hover:bg-muted border border-border transition-colors disabled:opacity-50 disabled:pointer-events-none cursor-pointer"
            >
              Choose folder
            </button>
          </div>
        </div>
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

      {isDragging && (
        <div className="fixed inset-0 z-40 bg-background/90 backdrop-blur-xs flex items-center justify-center p-8 pointer-events-none select-none">
          <div className="w-full h-full border-2 border-dashed border-primary/80 rounded-2xl flex flex-col items-center justify-center bg-primary/5">
            <FolderDown className="w-12 h-12 text-primary stroke-[1.5] mb-3" />
            <h3 className="text-base font-medium text-foreground mb-1">Drop folder or screenshots</h3>
            <p className="text-xs text-muted-foreground">Release to load images into the grid</p>
          </div>
        </div>
      )}

      {progress && (
        <div className="fixed inset-0 bg-background/80 backdrop-blur-xs z-50 flex items-center justify-center p-4 select-none">
          <div className="w-full max-w-md bg-card border border-border rounded-xl p-6">
            <div className="flex items-center justify-between mb-2">
              <h3 className="text-sm font-medium text-foreground">Cropping screenshots</h3>
              <span className="text-xs text-muted-foreground font-mono">{progress.current} of {progress.total} ({progressPct}%)</span>
            </div>
            <div className="w-full h-2 bg-muted rounded-full overflow-hidden mb-3">
              <div className="h-full bg-primary transition-all duration-150" style={{ width: `${progressPct}%` }} />
            </div>
            <p className="text-xs text-muted-foreground truncate font-mono">{progress.name || 'Processing...'}</p>
          </div>
        </div>
      )}

      <SummaryDialog
        results={summaryResults}
        outputFolder={outputFolder}
        outputMode={outputMode}
        onClose={() => setSummaryResults(null)}
        onViewCropped={() => {
          setSummaryResults(null)
          handleLoadPaths([outputFolder])
        }}
        onOpenFolder={() => {
          openFolder(outputFolder).catch((err) => console.error('Failed to open folder:', err))
        }}
      />
    </div>
  )
}
