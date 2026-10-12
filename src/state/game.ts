import { create } from 'zustand'
import { listVersions, launchGame, stopGame, type VersionEntry } from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'
import { useAccount } from './account'
import {
  onInstallProgress,
  onLaunchProgress,
  onGameLog,
  onGameExit,
  onLoaderInstalled,
  type InstallProgress,
  type LaunchProgress,
  type GameLog,
  type GameExit,
  type CrashInfo,
} from '../ipc/events'
import { t } from '../i18n'

interface GameState {
  versions: VersionEntry[]
  version: string
  nick: string
  busy: boolean
  stage: string
  progress: number
  playing: boolean
  logs: string[]
  error: string
  crash: CrashInfo | null
  dropCrash: () => void
  init: () => Promise<void>
  refresh: () => Promise<void>
  setVersion: (version: string) => void
  setNick: (nick: string) => void
  play: () => Promise<void>
  stop: () => Promise<void>
}

let wired = false

// Ник оффлайна идёт в игру как есть — те же правила, что у ванильных серверов
export const NICK_RE = /^[A-Za-z0-9_]{1,16}$/

export const useGame = create<GameState>((set, get) => ({
  versions: [],
  version: '',
  nick: localStorage.getItem('daylo.nick') ?? '',
  busy: false,
  stage: '',
  progress: 0,
  playing: false,
  logs: [],
  error: '',
  crash: null,

  dropCrash: () => set({ crash: null }),

  init: async () => {
    if (!hasTauri() || wired) return
    wired = true

    onInstallProgress((p: InstallProgress) => {
      if (p.error) {
        set({ busy: false, stage: t('game.stageError'), error: p.error })
        return
      }
      const pct = p.total > 0 ? Math.round((p.done / p.total) * 100) : 0
      set({ stage: p.stage, progress: p.stage === 'готово' ? 100 : pct })
      if (p.stage === 'готово') {
        // Версия доустановлена — обновляем список (флаг installed в библиотеке)
        void get().refresh()
        setTimeout(() => set({ progress: 0, stage: '' }), 1500)
      }
    })
    onLaunchProgress((p: LaunchProgress) => set({ stage: p.stage }))
    onGameLog((l: GameLog) =>
      set((s) => ({ logs: [...s.logs.slice(-300), l.line] })),
    )
    onLoaderInstalled((p) => {
      void get().refresh().then(() => set({ version: p.id }))
    })
    onGameExit((e: GameExit) => {
      set({ playing: false, stage: '', progress: 0, crash: e.crash })
      const tail = e.code === 0 ? t('game.exited') : t('game.exitedCode', { code: e.code ?? '?' })
      set((s) => ({ logs: [...s.logs.slice(-300), tail] }))
    })

    try {
      const versions = await listVersions()
      const stored = localStorage.getItem('daylo.version')
      const fallback = versions.find((v) => v.type === 'release')?.id ?? ''
      set({
        versions,
        version: stored && versions.some((v) => v.id === stored) ? stored : fallback,
      })
    } catch (e) {
      set({ error: String(e) })
    }
  },

  refresh: async () => {
    if (!hasTauri()) return
    try {
      const versions = await listVersions()
      set({ versions })
    } catch (e) {
      set({ error: String(e) })
    }
  },

  setVersion: (version) => {
    localStorage.setItem('daylo.version', version)
    set({ version })
  },

  setNick: (nick) => {
    localStorage.setItem('daylo.nick', nick)
    set({ nick })
  },

  play: async () => {
    const { version, nick, busy, playing } = get()
    if (busy || playing || !version) return
    const acc = useAccount.getState().info
    // Имя активного аккаунта (ely, msa или оффлайн-профиль) важнее поля ника
    const accName = acc?.name ?? ''
    const name = accName || nick.trim() || 'Player'
    // Гард для UX: без аккаунта имя уйдёт в игру как есть, ядро его отвергнет
    if (!accName && !NICK_RE.test(name)) {
      set({ error: t('error.nick') })
      return
    }
    set({ busy: true, error: '', stage: t('game.stagePreparing'), progress: 0 })
    try {
      // Аккаунт Ely.by важнее локального ника: ядро возьмёт его имя и токен
      await launchGame(version, name)
      set({ busy: false, playing: true, stage: t('inst.launched') })
    } catch (e) {
      set({ busy: false, error: String(e) })
    }
  },

  stop: async () => {
    const { version } = get()
    if (version) await stopGame(version).catch(() => {})
  },
}))
