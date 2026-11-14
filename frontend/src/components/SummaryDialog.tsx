import { useEffect, useState, type ReactElement } from 'react'
import {
  CheckCircle2,
  Copy,
  SkipForward,
  AlertCircle,
  X,
  FolderOpen,
  ExternalLink,
  ChevronDown,
  ChevronUp,
  type LucideIcon,
} from 'lucide-react'
import { cn } from '../lib/cn'
import type {
  BarMeasurement,
  CropReason,
  CropResultItem,
  CropStatus,
  OutputMode,
} from '../types/screenshot'

type ReasonGroupKey = CropReason | 'unknown'

const REASON_LABELS: Readonly<Record<ReasonGroupKey, string>> = {
  already_matches_ratio: 'Already matches target ratio',
  non_black_margins: 'Non-black margins',
  output_exists: 'Already exists in output folder',
  decode_failed: 'Failed to decode image',
  write_failed: 'Failed to write output',
  unknown: 'Other',
}

const STATUS_BADGE: Readonly<Record<Exclude<CropStatus, 'cropped'>, { label: string; className: string }>> = {
  skipped: {
    label: 'Skipped',
    className: 'bg-muted text-muted-foreground border-border/80',
  },
  copied: {
    label: 'Copied',
    className: 'bg-muted text-foreground border-border/80',
  },
  error: {
    label: 'Error',
    className: 'bg-destructive/10 text-destructive border-destructive/20',
  },
}

function formatBarMeasurement(m: BarMeasurement): string {
  const side = m.side.charAt(0).toUpperCase() + m.side.slice(1)
  return `${side} bar: ${m.bright_percent}% bright (max lum ${m.max_luminance}, threshold ${m.threshold})`
}

function formatResultDetail(item: CropResultItem): string {
  const dims = item.dimensions ? ` (${item.dimensions.width}×${item.dimensions.height})` : ''
  if (item.reason === 'non_black_margins' && item.margin_measurements && item.margin_measurements.length > 0) {
    const failedBars = item.margin_measurements.filter((m) => !m.passed)
    if (failedBars.length > 0) {
      return `${failedBars.map(formatBarMeasurement).join(' · ')}${dims}`
    }
  }
  return `${item.message}${dims}`
}

function getFileName(filePath: string): string {
  return filePath.split(/[\\/]/).pop() || filePath
}

interface SummaryStats {
  readonly croppedCount: number
  readonly copiedCount: number
  readonly skippedCount: number
  readonly errorCount: number
  readonly nonCroppedCount: number
  readonly skippedPaths: readonly string[]
  readonly groupedResults: ReadonlyMap<ReasonGroupKey, readonly CropResultItem[]>
}

function summarizeResults(results: readonly CropResultItem[]): SummaryStats {
  let croppedCount = 0
  let copiedCount = 0
  let skippedCount = 0
  let errorCount = 0
  let nonCroppedCount = 0
  const skippedPaths: string[] = []
  const groupedResults = new Map<ReasonGroupKey, CropResultItem[]>()

  for (const item of results) {
    if (item.status === 'cropped') {
      croppedCount++
      continue
    }

    nonCroppedCount++
    if (item.status === 'copied') {
      copiedCount++
      skippedPaths.push(item.path)
    } else if (item.status === 'skipped') {
      skippedCount++
      skippedPaths.push(item.path)
    } else {
      errorCount++
    }

    const key: ReasonGroupKey = item.reason ?? 'unknown'
    const bucket = groupedResults.get(key)
    if (bucket) bucket.push(item)
    else groupedResults.set(key, [item])
  }

  return { croppedCount, copiedCount, skippedCount, errorCount, nonCroppedCount, skippedPaths, groupedResults }
}

interface StatCardProps {
  readonly label: string
  readonly count: number
  readonly icon: LucideIcon
  readonly iconColor: string
  readonly isError?: boolean
  readonly interactive: boolean
  readonly isExpanded: boolean
  readonly onToggle: () => void
}

function StatCard({
  label,
  count,
  icon: Icon,
  iconColor,
  isError = false,
  interactive,
  isExpanded,
  onToggle,
}: StatCardProps): ReactElement {
  const cardClass = cn(
    'rounded-lg p-3 border',
    isError ? 'bg-destructive/10 border-destructive/20' : 'bg-muted/50 border-border/60',
    interactive &&
      (isError
        ? 'text-left hover:bg-destructive/15 transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
        : 'text-left hover:bg-muted/80 transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring')
  )

  const content = (
    <>
      <div className="flex items-center justify-between gap-2 mb-1">
        <div className="flex items-center gap-2">
          <Icon className={cn('w-4 h-4', iconColor)} />
          <span className={cn('text-xs font-medium', isError ? 'text-destructive' : 'text-foreground')}>
            {label}
          </span>
        </div>
        {interactive &&
          (isExpanded ? (
            <ChevronUp className={cn('w-3.5 h-3.5', isError ? 'text-destructive' : 'text-muted-foreground')} />
          ) : (
            <ChevronDown className={cn('w-3.5 h-3.5', isError ? 'text-destructive' : 'text-muted-foreground')} />
          ))}
      </div>
      <span className={cn('text-lg font-semibold tabular-nums', isError ? 'text-destructive' : 'text-foreground')}>
        {count}
      </span>
    </>
  )

  if (!interactive) {
    return <div className={cardClass}>{content}</div>
  }

  return (
    <button type="button" onClick={onToggle} aria-expanded={isExpanded} className={cardClass}>
      {content}
    </button>
  )
}

interface SummaryDialogProps {
  readonly results: readonly CropResultItem[] | null
  readonly outputFolder: string
  readonly outputMode?: OutputMode
  readonly onClose: () => void
  readonly onViewCropped?: () => void
  readonly onOpenFolder?: () => void
  readonly onSelectSkipped?: (paths: readonly string[]) => void
}

export function SummaryDialog({
  results,
  outputFolder,
  outputMode = 'directory',
  onClose,
  onViewCropped,
  onOpenFolder,
  onSelectSkipped,
}: SummaryDialogProps): ReactElement | null {
  const [isExpanded, setIsExpanded] = useState<boolean>(false)

  useEffect(() => {
    setIsExpanded(false)
  }, [results])

  if (!results || results.length === 0) return null

  const {
    croppedCount,
    copiedCount,
    skippedCount,
    errorCount,
    nonCroppedCount,
    skippedPaths,
    groupedResults,
  } = summarizeResults(results)

  const stats: readonly Omit<StatCardProps, 'isExpanded' | 'onToggle'>[] = [
    { label: 'Cropped', count: croppedCount, icon: CheckCircle2, iconColor: 'text-primary', interactive: false },
    { label: 'Copied', count: copiedCount, icon: Copy, iconColor: 'text-muted-foreground', interactive: copiedCount > 0 },
    { label: 'Skipped', count: skippedCount, icon: SkipForward, iconColor: 'text-muted-foreground', interactive: skippedCount > 0 },
    ...(errorCount > 0
      ? [{ label: 'Errors', count: errorCount, icon: AlertCircle, iconColor: 'text-destructive', isError: true, interactive: true }]
      : []),
  ]

  const toggleExpanded = () => setIsExpanded((prev) => !prev)

  return (
    <div className="fixed inset-0 bg-background/80 backdrop-blur-xs z-50 flex items-center justify-center p-4 select-none">
      <div
        className={cn(
          'w-full bg-card border border-border rounded-xl p-6 relative transition-all',
          isExpanded ? 'max-w-lg' : 'max-w-md'
        )}
      >
        <button
          type="button"
          onClick={onClose}
          aria-label="Close summary dialog"
          className="absolute top-4 right-4 text-muted-foreground hover:text-foreground transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-sm"
        >
          <X className="w-4 h-4" />
        </button>

        <h3 className="text-sm font-medium text-foreground mb-1">Processing complete</h3>
        <p className="text-xs text-muted-foreground mb-5 tabular-nums">
          Processed {results.length} screenshots.
        </p>

        <div className="grid grid-cols-2 gap-2.5 mb-5">
          {stats.map((stat) => (
            <StatCard key={stat.label} {...stat} isExpanded={isExpanded} onToggle={toggleExpanded} />
          ))}
        </div>

        {nonCroppedCount > 0 && (
          <div className="mb-4">
            <button
              type="button"
              onClick={toggleExpanded}
              aria-expanded={isExpanded}
              className="w-full flex items-center justify-between px-3 py-2 rounded-lg text-xs font-medium text-foreground bg-muted/40 hover:bg-muted/70 border border-border/60 transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              <span className="tabular-nums">
                {isExpanded
                  ? 'Hide details'
                  : `Show details for ${nonCroppedCount} non-cropped ${nonCroppedCount === 1 ? 'file' : 'files'}`}
              </span>
              {isExpanded ? (
                <ChevronUp className="w-3.5 h-3.5 text-muted-foreground" />
              ) : (
                <ChevronDown className="w-3.5 h-3.5 text-muted-foreground" />
              )}
            </button>

            {isExpanded && (
              <div className="mt-2 max-h-52 overflow-y-auto rounded-lg border border-border/60 bg-muted/20 p-2.5 space-y-3 select-text">
                {Array.from(groupedResults.entries()).map(([reasonKey, groupItems]) => (
                  <div key={reasonKey} className="space-y-1.5">
                    <div className="flex items-center justify-between px-1">
                      <span className="text-xs font-medium text-foreground">
                        {REASON_LABELS[reasonKey]}
                      </span>
                      <span className="text-[11px] font-mono text-muted-foreground tabular-nums">
                        {groupItems.length}
                      </span>
                    </div>

                    <div className="space-y-1">
                      {groupItems.map((item) => {
                        const badge = item.status === 'cropped' ? null : STATUS_BADGE[item.status]
                        return (
                          <div
                            key={`${item.status}:${item.path}`}
                            className="rounded-md border border-border/50 bg-card/80 px-2.5 py-2"
                          >
                            <div className="flex items-center justify-between gap-2 mb-0.5">
                              <span className="text-xs font-mono text-foreground truncate" title={item.path}>
                                {getFileName(item.path)}
                              </span>
                              {badge && (
                                <span
                                  className={cn(
                                    'shrink-0 px-1.5 py-0.5 rounded text-[11px] font-medium border',
                                    badge.className
                                  )}
                                >
                                  {badge.label}
                                </span>
                              )}
                            </div>
                            <p className="text-[11px] text-muted-foreground leading-snug tabular-nums break-words">
                              {formatResultDetail(item)}
                            </p>
                          </div>
                        )
                      })}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        <div className="bg-muted/30 border border-border/50 rounded-lg p-3 mb-5">
          <p className="text-[11px] text-muted-foreground mb-0.5">
            {outputMode === 'replace' ? 'Destination:' : 'Saved to:'}
          </p>
          <p className="text-xs text-foreground font-mono truncate" title={outputMode === 'replace' ? 'Original files replaced' : outputFolder}>
            {outputMode === 'replace' ? 'Replaced original files in-place' : outputFolder}
          </p>
        </div>

        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex flex-wrap items-center gap-2">
            {outputMode === 'directory' && onViewCropped && (
              <button
                type="button"
                onClick={onViewCropped}
                className="px-3 py-1.5 rounded-md text-xs font-medium text-foreground bg-muted hover:bg-muted/80 border border-border transition-colors cursor-pointer inline-flex items-center gap-1.5"
              >
                <FolderOpen className="w-3.5 h-3.5 text-primary" />
                <span>View in app</span>
              </button>
            )}
            {outputMode === 'directory' && onOpenFolder && (
              <button
                type="button"
                onClick={onOpenFolder}
                className="px-3 py-1.5 rounded-md text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-muted/60 border border-border/60 transition-colors cursor-pointer inline-flex items-center gap-1.5"
              >
                <ExternalLink className="w-3.5 h-3.5" />
                <span>Open folder</span>
              </button>
            )}
            {skippedPaths.length > 0 && onSelectSkipped && (
              <button
                type="button"
                onClick={() => onSelectSkipped(skippedPaths)}
                className="px-3 py-1.5 rounded-md text-xs font-medium text-foreground bg-muted hover:bg-muted/80 border border-border transition-colors cursor-pointer tabular-nums"
              >
                Select skipped ({skippedPaths.length})
              </button>
            )}
          </div>
          <div className="ml-auto">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-1.5 rounded-md text-xs font-medium text-primary-foreground bg-primary hover:bg-primary/90 transition-colors cursor-pointer"
            >
              Done
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
