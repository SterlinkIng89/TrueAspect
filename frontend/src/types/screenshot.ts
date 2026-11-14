export interface ScreenshotItem {
  readonly path: string
  readonly name: string
}

export type OutputMode = 'directory' | 'replace'

export type MarginSensitivity = 'strict' | 'normal' | 'tolerant'

export function isMarginSensitivity(value: string | null): value is MarginSensitivity {
  return value === 'strict' || value === 'normal' || value === 'tolerant'
}

export interface SensitivityOption {
  readonly label: string
  readonly value: MarginSensitivity
}

export type CropStatus = 'cropped' | 'copied' | 'skipped' | 'error'

export function isCropStatus(status: string): status is CropStatus {
  return status === 'cropped' || status === 'copied' || status === 'skipped' || status === 'error'
}

export type CropReason =
  | 'already_matches_ratio'
  | 'non_black_margins'
  | 'output_exists'
  | 'decode_failed'
  | 'write_failed'

export type BarSide = 'left' | 'right' | 'top' | 'bottom'

export interface ImageDimensions {
  readonly width: number
  readonly height: number
}

export interface CropBoxRect {
  readonly min_x: number
  readonly min_y: number
  readonly max_x: number
  readonly max_y: number
}

export interface BarMeasurement {
  readonly side: BarSide
  readonly passed: boolean
  readonly bright_percent: number
  readonly max_luminance: number
  readonly threshold: number
}

export interface CropResultItem {
  readonly path: string
  readonly status: CropStatus
  readonly message: string
  readonly reason?: CropReason | null
  readonly dimensions?: ImageDimensions | null
  readonly target_box?: CropBoxRect | null
  readonly margin_measurements?: readonly BarMeasurement[] | null
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
