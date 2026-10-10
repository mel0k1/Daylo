import { create } from 'zustand'
import { appInfo, updateCheck, updateInstall, type UpdateInfo } from '../ipc/commands'
import { onUpdateProgress, type UpdateProgress } from '../ipc/events'
import { hasTauri } from '../ipc/tauri'

export type ScreenId = 'play' | 'packs' | 'mods' | 'shots' | 'settings'

interface UiState {
  screen: ScreenId
  setScreen: (screen: ScreenId) => void
  ver: string
  update: UpdateInfo | null
  progress: UpdateProgress | null
  updateError: string
  updateInit: () => void
  updateCheckNow: () => Promise<void>
  updateApply: () => Promise<void>
}

let inited = false

export const useUi = create<UiState>((set, get) => ({
  screen: 'play',
  setScreen: (screen) => set({ screen }),
  ver: '',
  update: null,
  progress: null,
  updateError: '',

  // Слушатель прогресса ставится один раз даже в StrictMode;
  // свежая версия проверяется сразу при старте, сбой сети не мешает работе
  updateInit: () => {
    if (inited) return
    inited = true
    void appInfo()
      .then((a) => set({ ver: a.version }))
      .catch(() => {})
    if (!hasTauri()) return
    void onUpdateProgress((p) => set({ progress: p }))
    void get().updateCheckNow()
  },

  updateCheckNow: async () => {
    try {
      set({ update: await updateCheck() })
    } catch {
      // проверим в следующий раз
    }
  },

  updateApply: async () => {
    const update = get().update
    if (!update) return
    set({ updateError: '' })
    try {
      await updateInstall(update)
    } catch (e) {
      set({ updateError: String(e), progress: null })
    }
  },
}))
