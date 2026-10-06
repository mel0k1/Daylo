import { create } from 'zustand'

export type ScreenId = 'play' | 'mods' | 'settings'

interface UiState {
  screen: ScreenId
  setScreen: (screen: ScreenId) => void
}

export const useUi = create<UiState>((set) => ({
  screen: 'play',
  setScreen: (screen) => set({ screen }),
}))
