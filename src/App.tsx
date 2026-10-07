import { useEffect, useState } from 'react'
import { useUi } from './state/ui'
import { useAccount } from './state/account'
import { Play } from './screens/Play'
import { Packs } from './screens/Packs'
import { Mods } from './screens/Mods'
import { Shots } from './screens/Shots'
import { Settings } from './screens/Settings'
import { hasTauri } from './ipc/tauri'
import { appInfo, updateInstall, updateCheck, type UpdateInfo } from './ipc/commands'
import { onUpdateProgress, type UpdateProgress } from './ipc/events'

const navItems: { id: 'play' | 'packs' | 'mods' | 'shots' | 'settings'; label: string }[] = [
  { id: 'play', label: 'Играть' },
  { id: 'packs', label: 'Сборки' },
  { id: 'mods', label: 'Моды' },
  { id: 'shots', label: 'Скриншоты' },
  { id: 'settings', label: 'Настройки' },
]

export default function App() {
  const screen = useUi((s) => s.screen)
  const [update, setUpdate] = useState<UpdateInfo | null>(null)
  const [progress, setProgress] = useState<UpdateProgress | null>(null)
  const [updateError, setUpdateError] = useState('')
  const [ver, setVer] = useState('')
  const account = useAccount((s) => s.info)

  useEffect(() => {
    void useAccount.getState().init()
    void appInfo()
      .then((a) => setVer(a.version))
      .catch(() => {})
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
        {account && (
          <div className="acc-chip">
            <span className={account.mode === 'ely' ? 'acc-dot ely' : 'acc-dot'} />
            <span className="name">{account.name}</span>
            <span className="mode">{account.mode === 'ely' ? 'ely.by' : 'офлайн'}</span>
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
    </div>
  )
}
