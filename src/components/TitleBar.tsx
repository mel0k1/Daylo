import { useEffect, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { hasTauri } from '../ipc/tauri'
import { useT } from '../i18n'

// Пиксельные иконки кнопок окна: сетка 10×10, заливка currentColor
type Rect = [x: number, y: number, w: number, h: number]

function WinIcon({ r }: { r: Rect[] }) {
  return (
    <svg viewBox="0 0 10 10" width={10} height={10} shapeRendering="crispEdges" aria-hidden="true">
      {r.map(([x, y, w, h], i) => (
        <rect key={i} x={x} y={y} width={w} height={h} fill="currentColor" />
      ))}
    </svg>
  )
}

// Свернуть — черта по центру
const ICON_MIN: Rect[] = [[1, 4, 8, 2]]

// Развернуть — контур квадрата
const ICON_MAX: Rect[] = [
  [1, 1, 8, 1],
  [1, 8, 8, 1],
  [1, 2, 1, 6],
  [8, 2, 1, 6],
]

// Восстановить — два квадрата: задний рисуем только верхом и правым боком,
// передний перекрывает остальное
const ICON_RESTORE: Rect[] = [
  [4, 1, 5, 1],
  [8, 2, 1, 4],
  [1, 3, 5, 1],
  [1, 4, 1, 4],
  [1, 7, 5, 1],
  [5, 4, 1, 4],
]

// Закрыть — пиксельный крест
const ICON_CLOSE: Rect[] = [
  [1, 1, 2, 2],
  [3, 3, 2, 2],
  [5, 5, 2, 2],
  [7, 7, 2, 2],
  [7, 1, 2, 2],
  [5, 3, 2, 2],
  [3, 5, 2, 2],
  [1, 7, 2, 2],
]

// Кастомный титлбар: рамка окна наша — таскаем за полосу, двойной клик
// разворачивает (это делает само ядро для drag-region), справа плоские кнопки
// «свернуть / развернуть / закрыть». В браузерном режиме ядро не запущено —
// полоса рисуется без кнопок
export function TitleBar() {
  const t = useT()
  const [maximized, setMaximized] = useState(false)
  const online = hasTauri()

  useEffect(() => {
    if (!online) return
    // Ядро всегда даёт метаданные окна, но падение титлбара не должно
    // ронять весь интерфейс — оборачиваем на всякий случай
    try {
      const win = getCurrentWindow()
      let off: (() => void) | undefined
      // Состояние «развернуто» синхронизируем по любому ресайзу окна —
      // отдельного события на maximize/unmaximize не всегда хватает
      const sync = () => void win.isMaximized().then(setMaximized).catch(() => {})
      sync()
      void win
        .onResized(() => sync())
        .then((un) => (off = un))
        .catch(() => {})
      return () => off?.()
    } catch {
      return
    }
  }, [online])

  return (
    <header className="titlebar" data-tauri-drag-region={online ? '' : undefined}>
      <div className="titlebar-brand" data-tauri-drag-region={online ? '' : undefined}>
        <span className="titlebar-logo">Daylo</span>
        <span className="titlebar-strip" aria-hidden="true" />
      </div>
      {online && (
        <div className="win-controls">
          <button
            className="win-btn"
            aria-label={t('win.minimize')}
            title={t('win.minimize')}
            onClick={() => void getCurrentWindow().minimize()}
          >
            <WinIcon r={ICON_MIN} />
          </button>
          <button
            className="win-btn"
            aria-label={maximized ? t('win.restore') : t('win.maximize')}
            title={maximized ? t('win.restore') : t('win.maximize')}
            onClick={() => void getCurrentWindow().toggleMaximize()}
          >
            <WinIcon r={maximized ? ICON_RESTORE : ICON_MAX} />
          </button>
          <button
            className="win-btn close"
            aria-label={t('win.close')}
            title={t('win.close')}
            onClick={() => void getCurrentWindow().close()}
          >
            <WinIcon r={ICON_CLOSE} />
          </button>
        </div>
      )}
    </header>
  )
}
