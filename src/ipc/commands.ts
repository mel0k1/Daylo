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

export interface ModUpdate {
  file: string
  project_id: string
  current_version: string
  latest_version_id: string
  latest_version_number: string
}

export const modsUpdates = (version: string) =>
  invoke<ModUpdate[]>('mods_updates', { version })

export const modsUpdate = (version: string, file: string, modVersionId: string) =>
  invoke<void>('mods_update', { version, file, modVersionId })

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

export interface ServerStatus {
  online: boolean
  ms: number
  motd: string
  version: string
  players_online: number
  players_max: number
  favicon: string | null
  error: string | null
}

export interface DeviceStart {
  device_code: string
  user_code: string
  verification_uri: string
  interval: number
  expires_in: number
}

export interface PollResult {
  status: 'pending' | 'done' | 'error'
  message?: string
}

export interface AccountInfo {
  mode: 'ely' | 'offline'
  name: string
  uuid: string
}

export interface LauncherSettings {
  use_mirrors: boolean
  cf_api_key: string | null
}

export const pingServer = (addr: string) =>
  invoke<ServerStatus>('ping_server', { addr })

export const serversList = () => invoke<string[]>('servers_list')

export const serversAdd = (addr: string) => invoke<void>('servers_add', { addr })

export const serversRemove = (addr: string) => invoke<void>('servers_remove', { addr })

export interface GameServer {
  name: string
  address: string
  icon: string | null
}

export const gameServers = (version: string) =>
  invoke<GameServer[]>('game_servers', { version })

export const elyLoginStart = () => invoke<DeviceStart>('ely_login_start')

export const elyLoginPoll = (deviceCode: string) =>
  invoke<PollResult>('ely_login_poll', { deviceCode })

export const elyLogout = () => invoke<void>('ely_logout')

export const accountInfo = () => invoke<AccountInfo>('account_info')

export const elySkin = (nick: string) => invoke<string | null>('ely_skin', { nick })

export const getLauncherSettings = () => invoke<LauncherSettings>('get_launcher_settings')

export const setMirrors = (enabled: boolean) => invoke<void>('set_mirrors', { enabled })

export interface UpdateInfo {
  version: string
  notes: string | null
  url: string
  asset_name: string
  size: number
}

export const updateCheck = () => invoke<UpdateInfo | null>('update_check')

export const updateInstall = (info: UpdateInfo) => invoke<void>('update_install', { info })

export const packExport = (version: string) =>
  invoke<string | null>('pack_export', { version })

export const packImport = () => invoke<string | null>('pack_import')

// --- Каталог сборок: Modrinth / CurseForge / FTB ---

export interface PackHit {
  source: string
  id: string
  slug: string
  title: string
  description: string
  icon_url: string
  downloads: number
}

export interface PackVersionInfo {
  version_id: string
  name: string
  date: string
  mc_versions: string[]
  loaders: string[]
}

export interface InstanceInfo {
  id: string
  kind: string
  name: string
  mc_version: string
  loader: string
  launch_version: string
  source: string
  pack_version: string
  icon_url: string
  ram_mb: number
  mods_count: number
}

export const packSearch = (source: string, query: string, gameVersion: string) =>
  invoke<PackHit[]>('pack_search', { source, query, gameVersion })

export const packVersions = (source: string, packId: string, gameVersion: string) =>
  invoke<PackVersionInfo[]>('pack_versions', { source, packId, gameVersion })

export const packInstall = (source: string, packId: string, versionId: string, icon: string) =>
  invoke<void>('pack_install', { source, packId, versionId, icon })

export const instanceList = () => invoke<InstanceInfo[]>('instance_list')

export const instanceDelete = (id: string) => invoke<void>('instance_delete', { id })

export const launchInstance = (instance: string, nick: string) =>
  invoke<void>('launch_instance', { instance, nick })

export const curseforgeSearch = (query: string, gameVersion: string) =>
  invoke<SearchHit[]>('curseforge_search', { query, gameVersion })

export const curseforgeVersions = (modId: string, gameVersion: string) =>
  invoke<ModVersion[]>('curseforge_versions', { modId, gameVersion })

export const curseforgeInstall = (version: string, modId: string, fileId: string) =>
  invoke<void>('curseforge_install', { version, modId, fileId })

export const setCfKey = (key: string) => invoke<void>('set_cf_key', { key })

export interface Shot {
  name: string
  modified: number
  size: number
}

export const shotsList = (version: string) => invoke<Shot[]>('shots_list', { version })

export const shotsRead = (version: string, name: string) =>
  invoke<string>('shots_read', { version, name })

export const shotsDelete = (version: string, name: string) =>
  invoke<void>('shots_delete', { version, name })

export const shotsOpen = (version: string) => invoke<void>('shots_open', { version })
