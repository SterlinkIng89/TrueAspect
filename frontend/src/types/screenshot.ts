export interface ScreenshotItem {
  readonly path: string
  readonly name: string
}

export type CropStatus = 'cropped' | 'copied' | 'skipped' | 'error'

export function isCropStatus(status: string): status is CropStatus {
  return status === 'cropped' || status === 'copied' || status === 'skipped' || status === 'error'
}

export interface CropResultItem {
  readonly path: string
  readonly status: CropStatus
  readonly message: string
}

export interface AspectRatioOption {
  readonly label: string
  readonly value: string
}

export interface CropProgress {
  readonly current: number
  readonly total: number
  readonly name: string
}
