import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { ScreenshotItem, CropResultItem, CropProgress, OutputMode } from '../types/screenshot'

export async function getDefaultOutputDir(): Promise<string> {
  return await invoke<string>('get_default_output_dir')
}

export async function pickFolder(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: 'Select screenshots folder',
  })
  return selected ? (selected as string) : null
}

export async function pickOutputFolder(defaultPath?: string): Promise<string | null> {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: defaultPath || undefined,
      title: 'Select output folder',
    })
    return selected ? (selected as string) : null
  } catch {
    const fallback = await open({
      directory: true,
      multiple: false,
      title: 'Select output folder',
    })
    return fallback ? (fallback as string) : null
  }
}

export async function loadScreenshots(paths: readonly string[]): Promise<readonly ScreenshotItem[]> {
  return await invoke<ScreenshotItem[]>('load_screenshots', { paths: [...paths] })
}

export async function cropScreenshots(
  paths: readonly string[],
  ratioStr: string,
  outDir: string,
  mode: OutputMode = 'directory'
): Promise<readonly CropResultItem[]> {
  return await invoke<CropResultItem[]>('crop_screenshots', {
    paths: [...paths],
    ratioStr,
    outDirStr: outDir,
    mode,
  })
}

export async function onCropProgress(callback: (payload: CropProgress) => void): Promise<UnlistenFn> {
  return await listen<CropProgress>('crop:progress', (event) => {
    callback(event.payload)
  })
}

export async function onDragDropFiles(callback: (paths: readonly string[]) => void): Promise<UnlistenFn> {
  const webview = getCurrentWebview()
  return await webview.onDragDropEvent((event) => {
    if (event.payload.type === 'drop') {
      const paths = event.payload.paths
      if (Array.isArray(paths) && paths.length > 0) {
        callback(paths)
      }
    }
  })
}

export function getThumbnailUrl(filePath: string, version?: number): string {
  const v = version !== undefined ? `&v=${version}` : ''
  return `http://thumb.localhost/?path=${encodeURIComponent(filePath)}${v}`
}
