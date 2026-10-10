import { create } from 'zustand'
import {
  accountInfo,
  elyLoginPoll,
  elyLoginStart,
  elyLogout,
  msaLoginPoll,
  msaLoginStart,
  type AccountInfo,
} from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'

type Provider = 'ely' | 'msa'

interface LoginFlow {
  provider: Provider
  deviceCode: string
  userCode: string
  verificationUri: string
  interval: number
  deadline: number
}

interface AccountState {
  info: AccountInfo | null
  login: LoginFlow | null
  error: string
  init: () => Promise<void>
  startLogin: (provider: Provider) => Promise<void>
  cancelLogin: () => void
  logout: () => Promise<void>
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

export const useAccount = create<AccountState>((set, get) => ({
  info: null,
  login: null,
  error: '',

  init: async () => {
    if (!hasTauri()) return
    try {
      set({ info: await accountInfo(), error: '' })
    } catch (e) {
      set({ error: String(e) })
    }
  },

  startLogin: async (provider) => {
    if (!hasTauri() || get().login) return
    set({ error: '' })
    try {
      const d = provider === 'msa' ? await msaLoginStart() : await elyLoginStart()
      set({
        login: {
          provider,
          deviceCode: d.device_code,
          userCode: d.user_code,
          verificationUri: d.verification_uri,
          interval: Math.max(d.interval, 2),
          deadline: Date.now() + d.expires_in * 1000,
        },
      })
      void pollLoop(set, get)
    } catch (e) {
      set({ error: String(e) })
    }
  },

  cancelLogin: () => set({ login: null }),

  logout: async () => {
    try {
      await elyLogout()
    } catch (e) {
      set({ error: String(e) })
    }
    await get().init()
  },
}))

// Опрос токена живёт, пока открыт экран входа или не истёк код
async function pollLoop(
  set: (s: Partial<AccountState>) => void,
  get: () => AccountState,
): Promise<void> {
  while (true) {
    const login = get().login
    if (!login) return
    await sleep(login.interval * 1000)
    const current = get().login
    if (!current || current.deviceCode !== login.deviceCode) return
    if (Date.now() > login.deadline) {
      set({ login: null, error: 'время действия кода истекло' })
      return
    }
    try {
      const r = login.provider === 'msa'
        ? await msaLoginPoll(login.deviceCode)
        : await elyLoginPoll(login.deviceCode)
      if (r.status === 'done') {
        set({ login: null })
        await get().init()
        return
      }
      if (r.status === 'error') {
        set({ login: null, error: r.message ?? 'ely.by отклонил вход' })
        return
      }
    } catch {
      // сбой сети не отменяет вход — пробуем до истечения кода
    }
  }
}
