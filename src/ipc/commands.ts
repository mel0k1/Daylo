import { invoke } from './tauri'

export interface AppInfo {
  name: string
  version: string
}

export interface VersionEntry {
  id: string
  type: string
  release_time: string
  // Профили загрузчиков: тег и базовая версия игры
  loader: string
  base: string
}

export interface LoaderBuild {
  version: string
  stable: boolean
  recommended: boolean
}

export const appInfo = () => invoke<AppInfo>('app_info')

export const listVersions = () => invoke<VersionEntry[]>('list_versions')

export const installVersion = (version: string) =>
  invoke<void>('install_version', { version })

export const launchGame = (version: string, nick: string) =>
  invoke<void>('launch_game', { version, nick })

export const stopGame = (version: string) => invoke<void>('stop_game', { version })

export interface SearchHit {
  project_id: string
  slug: string
  title: string
  description: string
  icon_url: string
  downloads: number
}

export interface ModVersion {
  version_id: string
  version_number: string
  file_name: string
  date: string
  size: number
  loaders: string[]
}

export interface InstanceConfig {
  ram_mb: number
  jvm_args: string[]
}

export const modrinthSearch = (query: string, gameVersion: string) =>
  invoke<SearchHit[]>('modrinth_search', { query, gameVersion })

export const modrinthVersions = (projectId: string, gameVersion: string) =>
  invoke<ModVersion[]>('modrinth_versions', { projectId, gameVersion })

export const modrinthInstall = (version: string, modVersionId: string) =>
  invoke<void>('modrinth_install', { version, modVersionId })

export const modsList = (version: string) => invoke<string[]>('mods_list', { version })

export const modsDelete = (version: string, file: string) =>
  invoke<void>('mods_delete', { version, file })

export const getInstanceConfig = (version: string) =>
  invoke<InstanceConfig>('get_instance_config', { version })

export const saveInstanceConfig = (version: string, ramMb: number, jvmArgs: string[]) =>
  invoke<void>('save_instance_config', { version, ramMb, jvmArgs })

export const loaderBuilds = (loader: string, version: string) =>
  invoke<LoaderBuild[]>('loader_builds', { loader, version })

export const installLoader = (loader: string, version: string, build: string) =>
  invoke<void>('install_loader', { loader, version, build })

export const skinSave = (nick: string, pngBase64: string) =>
  invoke<void>('skin_save', { nick, pngBase64 })

export const skinLoad = (nick: string) => invoke<string | null>('skin_load', { nick })

export const skinDelete = (nick: string) => invoke<void>('skin_delete', { nick })
