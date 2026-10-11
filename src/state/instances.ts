import { create } from 'zustand'
import {
  instanceDelete,
  instanceList,
  instanceVerify,
  launchInstance,
  type InstanceInfo,
} from '../ipc/commands'
import {
  onGameExit,
  onPackInstalled,
  onPackProgress,
  onVerifyDone,
} from '../ipc/events'
import { hasTauri } from '../ipc/tauri'
import { NICK_RE, useGame } from './game'
import { useAccount } from './account'

interface InstancesState {
  instances: InstanceInfo[]
  // Прогресс установки/проверки (pack-progress): текст и проценты
  progress: string
  packPct: number
  // Статус вне прогресса: «игра запущена», сводка проверки
  stage: string
  // id сборки, которую запускаем/проверяем прямо сейчас
  launching: string
  verifying: string
  error: string
  init: () => void
  refresh: () => Promise<void>
  launch: (id: string) => Promise<void>
  verify: (id: string) => Promise<void>
  remove: (info: InstanceInfo) => Promise<void>
}

let wired = false

// Установленные сборки — общий стор для экрана «Играть» (библиотека)
// и экрана «Сборки»: слушатели событий ставятся один раз
export const useInstances = create<InstancesState>((set, get) => ({
  instances: [],
  progress: '',
  packPct: 0,
  stage: '',
  launching: '',
  verifying: '',
  error: '',

  init: () => {
    if (!hasTauri() || wired) return
    wired = true

    void onPackProgress((p) => {
      set({
        progress: p.error ? '' : `${p.stage} ${p.done}/${p.total}`,
        packPct: p.total > 0 ? Math.round((p.done / p.total) * 100) : 0,
      })
    })
    void onPackInstalled((p) => {
      set({ progress: '', packPct: 0 })
      void get().refresh()
      // Установка могла докачать базовую версию игры — обновляем список версий
      void useGame.getState().refresh()
      if (p.error) set({ error: p.error })
    })
    void onVerifyDone((p) => {
      set({ progress: '', packPct: 0, verifying: '' })
      void get().refresh()
      if (p.error) {
        set({ error: p.error })
        return
      }
      const parts: string[] = []
      if (p.broken > 0) parts.push(`исправлено файлов: ${p.broken}`)
      if (p.mods_fixed > 0) parts.push(`модов восстановлено: ${p.mods_fixed}`)
      if (p.mods_failed.length > 0) parts.push(`не вернулись: ${p.mods_failed.join(', ')}`)
      set({
        stage:
          parts.length === 0
            ? 'Файлы проверены — всё в порядке'
            : `Файлы проверены: ${parts.join('; ')}`,
      })
    })
    // Игра может быть запущена с любого экрана; по выходу гасим статус
    void onGameExit(() => set({ stage: '' }))

    void get().refresh()
  },

  refresh: async () => {
    if (!hasTauri()) return
    try {
      set({ instances: await instanceList(), error: '' })
    } catch {
      set({ instances: [] })
    }
  },

  launch: async (id) => {
    if (!hasTauri() || get().launching || get().verifying) return
    const acc = useAccount.getState().info
    // Имя активного аккаунта (ely, msa или оффлайн-профиль) важнее поля ника
    const accName = acc?.name ?? ''
    const nick = useGame.getState().nick.trim()
    const name = accName || nick || 'Player'
    // Без аккаунта имя уйдёт в игру как есть — проверяем до запуска
    if (!accName && !NICK_RE.test(name)) {
      set({ error: 'ник: только латиница, цифры и подчёркивания, 1–16 символов' })
      return
    }
    set({ launching: id, error: '' })
    try {
      await launchInstance(id, name)
      set({ stage: 'игра запущена' })
    } catch (e) {
      set({ error: String(e) })
    } finally {
      set({ launching: '' })
    }
  },

  // Проверка файлов: скан хешей, перекачка битого, возврат удалённых модов;
  // прогресс идёт через pack-progress, итог — verify-done
  verify: async (id) => {
    if (!hasTauri() || get().progress || get().verifying || get().launching) return
    set({ error: '', verifying: id })
    try {
      await instanceVerify(id)
    } catch (e) {
      set({ error: String(e), verifying: '' })
    }
  },

  remove: async (info) => {
    if (!window.confirm(`Удалить сборку «${info.name}» вместе с мирами и модами?`)) return
    set({ error: '' })
    try {
      await instanceDelete(info.id)
      await get().refresh()
    } catch (e) {
      set({ error: String(e) })
    }
  },
}))
