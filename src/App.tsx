import { useUi } from './state/ui'
import { Play } from './screens/Play'
import { Settings } from './screens/Settings'

export default function App() {
  const screen = useUi((s) => s.screen)

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="logo">Daylo</div>
        <button
          className={screen === 'play' ? 'nav on' : 'nav'}
          onClick={() => useUi.getState().setScreen('play')}
        >
          Играть
        </button>
        <button
          className={screen === 'settings' ? 'nav on' : 'nav'}
          onClick={() => useUi.getState().setScreen('settings')}
        >
          Настройки
        </button>
      </nav>
      <main className="content">{screen === 'play' ? <Play /> : <Settings />}</main>
    </div>
  )
}
