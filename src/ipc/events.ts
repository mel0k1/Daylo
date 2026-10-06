import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface InstallProgress {
  version: string
  stage: string
  done: number
  total: number
  error: string | null
}

export interface LaunchProgress {
  version: string
  stage: string
}

export interface GameLog {
  version: string
  line: string
}

export interface GameExit {
  version: string
  code: number | null
  crash: CrashInfo | null
}

export interface CrashInfo {
  title: string
  reason: string
  advice: string[]
  excerpt: string[]
  exit_code: number | null
}

export const onInstallProgress = (fn: (p: InstallProgress) => void): Promise<UnlistenFn> =>
  listen<InstallProgress>('install-progress', (e) => fn(e.payload))

export const onLaunchProgress = (fn: (p: LaunchProgress) => void): Promise<UnlistenFn> =>
  listen<LaunchProgress>('launch-progress', (e) => fn(e.payload))

export const onGameLog = (fn: (l: GameLog) => void): Promise<UnlistenFn> =>
  listen<GameLog>('game-log', (e) => fn(e.payload))

export const onGameExit = (fn: (e: GameExit) => void): Promise<UnlistenFn> =>
  listen<GameExit>('game-exit', (e) => fn(e.payload))

export interface LoaderInstalled {
  id: string
  base: string
}

export const onLoaderInstalled = (fn: (p: LoaderInstalled) => void): Promise<UnlistenFn> =>
  listen<LoaderInstalled>('loader-installed', (e) => fn(e.payload))

export interface UpdateProgress {
  received: number
  total: number
  done: boolean
}

export const onUpdateProgress = (fn: (p: UpdateProgress) => void): Promise<UnlistenFn> =>
  listen<UpdateProgress>('update-progress', (e) => fn(e.payload))

export interface PackProgress {
  stage: string
  done: number
  total: number
  error: string | null
}

export interface PackImported {
  id: string
  error: string | null
}

export const onPackProgress = (fn: (p: PackProgress) => void): Promise<UnlistenFn> =>
  listen<PackProgress>('pack-progress', (e) => fn(e.payload))

export const onPackImported = (fn: (p: PackImported) => void): Promise<UnlistenFn> =>
  listen<PackImported>('pack-imported', (e) => fn(e.payload))
