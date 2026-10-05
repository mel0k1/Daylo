import { useEffect, useState } from 'react'
import { appInfo, type AppInfo } from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'

export function Play() {
  const [info, setInfo] = useState<AppInfo | null>(null)

  useEffect(() => {
    if (!hasTauri()) return
    appInfo()
      .then(setInfo)
      .catch(() => {})
  }, [])

  return (
    <div className="screen">
      <h1>Играть</h1>
      <p>Здесь появятся сборки и запуск игры.</p>
      {info && (
        <p className="muted">
          {info.name} v{info.version}
        </p>
      )}
      {!hasTauri() && <p className="muted">Ядро не запущено — режим браузера.</p>}
    </div>
  )
}
