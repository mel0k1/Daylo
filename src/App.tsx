import { useEffect, type CSSProperties, type ReactElement } from 'react'
import { useUi } from './state/ui'
import {
  IconPlay,
  IconPacks,
  IconMods,
  IconWorlds,
  IconShots,
  IconSettings,
} from './components/icons'
import { useAccount } from './state/account'
import { useGame } from './state/game'
import { Play } from './screens/Play'
import { Packs } from './screens/Packs'
import { Mods } from './screens/Mods'
import { Worlds } from './screens/Worlds'
import { Shots } from './screens/Shots'
import { Settings } from './screens/Settings'
import { CrashModal } from './components/CrashModal'

const navItems: {
  id: 'play' | 'packs' | 'mods' | 'worlds' | 'shots' | 'settings'
  label: string
  icon: ReactElement
}[] = [
  { id: 'play', label: 'Играть', icon: <IconPlay /> },
  { id: 'packs', label: 'Сборки', icon: <IconPacks /> },
  { id: 'mods', label: 'Моды', icon: <IconMods /> },
  { id: 'worlds', label: 'Миры', icon: <IconWorlds /> },
  { id: 'shots', label: 'Скриншоты', icon: <IconShots /> },
  { id: 'settings', label: 'Настройки', icon: <IconSettings /> },
]

// Сплэш у логотипа — один на запуск, как на титульном экране игры
const SPLASHES = [
  'Не копай прямо вниз!',
  '100% пиксельно!',
  'Алмазы на -54!',
  'Не забудь про бэкапы!',
  'Собери свою сборку!',
  'Крипера не будет. Наверно.',
  'Также попробуй настоящую игру!',
  'Сделано с любовью к пикселям',
  'Моды? Моды!',
  'Пиксели навсегда!',
  'Джава внутри!',
  'Осторожно, зелья!',
]
const splash = SPLASHES[Math.floor(Math.random() * SPLASHES.length)]

// Медленные пиксель-частицы, поднимающиеся снизу вверх (стиль портала).
// Детерминированный набор: без рандома на каждом рендере
const PARTICLES: {
  left: string
  size: number
  dur: string
  delay: string
  color: string
  op: number
}[] = [
  { left: '4%', size: 5, dur: '31s', delay: '-4s', color: 'var(--p1, #3d55e8)', op: 0.4 },
  { left: '13%', size: 4, dur: '24s', delay: '-14s', color: 'var(--p2, #6fbf83)', op: 0.3 },
  { left: '22%', size: 6, dur: '38s', delay: '-22s', color: 'var(--p3, #8f5f9f)', op: 0.35 },
  { left: '31%', size: 4, dur: '27s', delay: '-9s', color: 'var(--p4, #e6e8eb)', op: 0.22 },
  { left: '40%', size: 5, dur: '34s', delay: '-18s', color: 'var(--p1, #3d55e8)', op: 0.4 },
  { left: '52%', size: 4, dur: '22s', delay: '-2s', color: 'var(--p2, #6fbf83)', op: 0.3 },
  { left: '61%', size: 6, dur: '41s', delay: '-27s', color: 'var(--p1, #3d55e8)', op: 0.32 },
  { left: '70%', size: 4, dur: '26s', delay: '-11s', color: 'var(--p5, #f0b64b)', op: 0.25 },
  { left: '79%', size: 5, dur: '36s', delay: '-20s', color: 'var(--p3, #8f5f9f)', op: 0.35 },
  { left: '88%', size: 4, dur: '23s', delay: '-6s', color: 'var(--p4, #e6e8eb)', op: 0.22 },
  { left: '95%', size: 5, dur: '30s', delay: '-16s', color: 'var(--p1, #3d55e8)', op: 0.4 },
  { left: '47%', size: 4, dur: '44s', delay: '-33s', color: 'var(--p2, #6fbf83)', op: 0.26 },
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
      <div className="bg-fx" aria-hidden="true">
        {PARTICLES.map((p, i) => (
          <i
            key={i}
            style={{
              left: p.left,
              width: p.size,
              height: p.size,
              background: p.color,
              '--o': p.op,
              '--dur': p.dur,
              animationDelay: p.delay,
            } as CSSProperties}
          />
        ))}
      </div>
      <main className="content">
        {screen === 'play' && <Play />}
        {screen === 'packs' && <Packs />}
        {screen === 'mods' && <Mods />}
        {screen === 'worlds' && <Worlds />}
        {screen === 'shots' && <Shots />}
        {screen === 'settings' && <Settings />}
      </main>
      <nav className="bottombar">
        <div className="logo-wrap">
          <div className="logo">Daylo</div>
          <div className="logo-strip" />
          <span className="splash" aria-hidden="true">
            {splash}
          </span>
        </div>
        {navItems.map((item) => (
          <button
            key={item.id}
            className={screen === item.id ? 'nav on' : 'nav'}
            onClick={() => useUi.getState().setScreen(item.id)}
          >
            <span className="nav-ico">{item.icon}</span>
            {item.label}
          </button>
        ))}
        <div className="bar-right">
          {update && !progress && (
            <div className="update-chip">
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
            <div className="update-chip">
              <span className="muted">
                {progress.done ? 'Устанавливаем…' : `Обновление: ${pct}%`}
              </span>
              <div className="update-bar">
                <div
                  className="update-bar-fill"
                  style={{ width: `${progress.done ? 100 : pct}%` }}
                />
              </div>
            </div>
          )}
          {updateError && (
            <span className="error upd-err" title={updateError}>
              {updateError}
            </span>
          )}
          {account && account.mode !== 'offline' && (
            <div className="acc-chip">
              <span className={account.mode === 'ely' ? 'acc-dot ely' : 'acc-dot'} />
              <span className="name">{account.name}</span>
              <span className="mode">{account.mode === 'ely' ? 'ely.by' : 'Microsoft'}</span>
            </div>
          )}
          {ver && <div className="side-ver">v{ver}</div>}
        </div>
      </nav>
      {crash && <CrashModal info={crash} onClose={dropCrash} />}
    </div>
  )
}
