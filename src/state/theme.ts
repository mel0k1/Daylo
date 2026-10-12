import { create } from 'zustand'

// Темы оформления — чистый фронт: переменные в base.css, переключение
// через data-theme на <html>. Сохранение в localStorage (UI-настройка,
// ядро не нужно — работает и в браузерном режиме)
export type ThemeName = 'dark' | 'light' | 'plains' | 'nether'

const KEY = 'daylo.theme'
const THEMES: ThemeName[] = ['dark', 'light', 'plains', 'nether']

function load(): ThemeName {
  try {
    const t = localStorage.getItem(KEY) as ThemeName | null
    return t && THEMES.includes(t) ? t : 'dark'
  } catch {
    return 'dark'
  }
}

function apply(t: ThemeName) {
  if (t === 'dark') delete document.documentElement.dataset.theme
  else document.documentElement.dataset.theme = t
}

export const useTheme = create<{ theme: ThemeName; set: (t: ThemeName) => void }>((set) => ({
  theme: load(),
  set: (theme) => {
    try {
      localStorage.setItem(KEY, theme)
    } catch {
      /* приватный режим — тема просто не запомнится */
    }
    apply(theme)
    set({ theme })
  },
}))

// Накинуть сохранённую тему до первого кадра — вызывает App при старте
export function initTheme() {
  apply(load())
}
