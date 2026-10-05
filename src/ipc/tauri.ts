import { invoke as tauriInvoke } from '@tauri-apps/api/core'

export const hasTauri = () => '__TAURI_INTERNALS__' in window

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!hasTauri()) throw new Error('Приложение запущено вне ядра Daylo')
  return tauriInvoke<T>(cmd, args)
}
