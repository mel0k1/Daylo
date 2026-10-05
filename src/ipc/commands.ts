import { invoke } from './tauri'

export interface AppInfo {
  name: string
  version: string
}

export interface VersionEntry {
  id: string
  type: string
  release_time: string
}

export const appInfo = () => invoke<AppInfo>('app_info')

export const listVersions = () => invoke<VersionEntry[]>('list_versions')

export const installVersion = (version: string) =>
  invoke<void>('install_version', { version })

export const launchGame = (version: string, nick: string) =>
  invoke<void>('launch_game', { version, nick })

export const stopGame = (version: string) => invoke<void>('stop_game', { version })
