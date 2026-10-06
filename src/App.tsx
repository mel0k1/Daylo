import { useEffect, useState } from 'react'
import { useUi } from './state/ui'
import { useAccount } from './state/account'
import { Play } from './screens/Play'
import { Mods } from './screens/Mods'
import { Settings } from './screens/Settings'
import { hasTauri } from './ipc/tauri'
import { updateInstall, updateCheck, type UpdateInfo } from './ipc/commands'
import { onUpdateProgress, type UpdateProgress } from './ipc/events'

const navItems: { id: 'play' | 'mods' | 'settings'; label: string }[] = [
  { id: 'play', label: 'Играть' },
  { id: 'mods', label: 'Моды' },
  { id: 'settings', label: 'Настройки' },
]

export default function App() {
  const screen = useUi((s) => s.screen)
  const [update, setUpdate] = useState<UpdateInfo | null>(null)
  const [progress, setProgress] = useState<UpdateProgress | null>(null)
  const [updateError, setUpdateError] = useState('')

  useEffect(() => {
    void useAccount.getState().init()
    if (!hasTauri()) return
    // Проверка свежей версии при старте; сбой сети не мешает работе
    void updateCheck()
      .then(setUpdate)
      .catch(() => {})
    void onUpdateProgress((p) => setProgress(p))
  }, [])

  const applyUpdate = async () => {
    if (!update) return
    setUpdateError('')
    try {
      await updateInstall(update)
    } catch (e) {
      setUpdateError(String(e))
      setProgress(null)
    }
  }

  const pct =
    progress && progress.total > 0 ? Math.min(100, Math.round((progress.received / progress.total) * 100)) : 0

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
        {update && !progress && (
          <div className="update-banner">
            <span className="muted">Доступна v{update.version}</span>
            <button className="link-btn" onClick={() => void applyUpdate()}>
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
      </nav>
      <main className="content">
        {screen === 'play' && <Play />}
        {screen === 'mods' && <Mods />}
        {screen === 'settings' && <Settings />}
      </main>
    </div>
  )
}
