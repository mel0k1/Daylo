import { useEffect } from 'react'
import { useUi } from './state/ui'
import { useAccount } from './state/account'
import { Play } from './screens/Play'
import { Mods } from './screens/Mods'
import { Settings } from './screens/Settings'

const navItems: { id: 'play' | 'mods' | 'settings'; label: string }[] = [
  { id: 'play', label: 'Играть' },
  { id: 'mods', label: 'Моды' },
  { id: 'settings', label: 'Настройки' },
]

export default function App() {
  const screen = useUi((s) => s.screen)

  useEffect(() => {
    void useAccount.getState().init()
  }, [])

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="logo">Daylo</div>
        {navItems.map((item) => (
          <button
            key={item.id}
            className={screen === item.id ? 'nav on' : 'nav'}
            onClick={() => useUi.getState().setScreen(item.id)}
          >
            {item.label}
          </button>
        ))}
      </nav>
      <main className="content">
        {screen === 'play' && <Play />}
        {screen === 'mods' && <Mods />}
        {screen === 'settings' && <Settings />}
      </main>
    </div>
  )
}
