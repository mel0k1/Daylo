import { invoke } from './tauri'

export interface AppInfo {
  name: string
  version: string
}

export const appInfo = () => invoke<AppInfo>('app_info')
