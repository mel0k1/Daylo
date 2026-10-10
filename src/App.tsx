import { useEffect } from 'react'
import { useUi } from './state/ui'
import { useAccount } from './state/account'
import { useGame } from './state/game'
import { Play } from './screens/Play'
import { Packs } from './screens/Packs'
import { Mods } from './screens/Mods'
import { Shots } from './screens/Shots'
import { Settings } from './screens/Settings'
import { CrashModal } from './components/CrashModal'

const navItems: { id: 'play' | 'packs' | 'mods' | 'shots' | 'settings'; label: string }[] = [
  { id: 'play', label: 'Играть' },
  { id: 'packs', label: 'Сборки' },
  { id: 'mods', label: 'Моды' },
  { id: 'shots', label: 'Скриншоты' },
  { id: 'settings', label: 'Настройки' },
]

export default function App() {
  const screen = useUi((s) => s.screen)
  const ver = useUi((s) => s.ver)
  const update = useUi((s) => s.update)
  const progress = useUi((s) => s.progress)
  const updateError = useUi((s) => s.updateError)
  const account = useAccount((s) => s.info)
  // Краш-модал на уровне приложения: краш виден с любого экрана,
  // игра может быть запущена из «Сборок», а не только из «Играть»
  const crash = useGame((s) => s.crash)
  const dropCrash = useGame((s) => s.dropCrash)

  useEffect(() => {
    void useAccount.getState().init()
    // Версия, проверка обновлений и слушатель прогресса — в общем сторе
    useUi.getState().updateInit()
  }, [])

  const pct =
    progress && progress.total > 0 ? Math.min(100, Math.round((progress.received / progress.total) * 100)) : 0

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="logo">Daylo</div>
        <div className="logo-strip" />
        {navItems.map((item) => (
          <button
            key={item.id}
            className={screen === item.id ? 'nav on' : 'nav'}
            onClick={() => useUi.getState().setScreen(item.id)}
          >
            {item.label}
          </button>
        ))}
        {update && !progress && (
          <div className="update-banner">
            <span className="muted">Доступна v{update.version}</span>
            <button
              className="link-btn"
              onClick={() => void useUi.getState().updateApply()}
            >
              Обновить
            </button>
          </div>
        )}
        {progress && (
          <div className="update-banner">
            <span className="muted">
              {progress.done ? 'Устанавливаем…' : `Обновление: ${pct}%`}
            </span>
            <div className="update-bar">
              <div className="update-bar-fill" style={{ width: `${progress.done ? 100 : pct}%` }} />
            </div>
          </div>
        )}
        {updateError && <span className="error upd-err">{updateError}</span>}
        {account && account.mode !== 'offline' && (
          <div className="acc-chip">
            <span className={account.mode === 'ely' ? 'acc-dot ely' : 'acc-dot'} />
            <span className="name">{account.name}</span>
            <span className="mode">{account.mode === 'ely' ? 'ely.by' : 'Microsoft'}</span>
          </div>
        )}
        {ver && <div className="side-ver">v{ver}</div>}
      </nav>
      <main className="content">
        {screen === 'play' && <Play />}
        {screen === 'packs' && <Packs />}
        {screen === 'mods' && <Mods />}
        {screen === 'shots' && <Shots />}
        {screen === 'settings' && <Settings />}
      </main>
      {crash && <CrashModal info={crash} onClose={dropCrash} />}
    </div>
  )
}
