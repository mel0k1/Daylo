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
}

export const onInstallProgress = (fn: (p: InstallProgress) => void): Promise<UnlistenFn> =>
  listen<InstallProgress>('install-progress', (e) => fn(e.payload))

export const onLaunchProgress = (fn: (p: LaunchProgress) => void): Promise<UnlistenFn> =>
  listen<LaunchProgress>('launch-progress', (e) => fn(e.payload))

export const onGameLog = (fn: (l: GameLog) => void): Promise<UnlistenFn> =>
  listen<GameLog>('game-log', (e) => fn(e.payload))

export const onGameExit = (fn: (e: GameExit) => void): Promise<UnlistenFn> =>
  listen<GameExit>('game-exit', (e) => fn(e.payload))
