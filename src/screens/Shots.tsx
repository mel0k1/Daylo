import { useEffect, useState } from 'react'
import { Button, Card, DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from 'pixel-retroui'
import { shotsDelete, shotsList, shotsOpen, shotsRead, type Shot } from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'

const MAX_SHOWN = 80

// Превью грузится лениво, по одному снимку
function ShotCard({
  version,
  shot,
  onOpen,
}: {
  version: string
  shot: Shot
  onOpen: () => void
}) {
  const [src, setSrc] = useState('')
  const [dead, setDead] = useState(false)

  useEffect(() => {
    let alive = true
    void shotsRead(version, shot.name)
      .then((s) => alive && setSrc(s))
      .catch(() => alive && setDead(true))
    return () => {
      alive = false
    }
  }, [version, shot.name])

  if (dead) return null
  return (
    <button className="shot-card" onClick={onOpen} title={shot.name}>
      {src ? (
        <img className="shot-img" src={src} alt={shot.name} loading="lazy" />
      ) : (
        <div className="shot-img empty" />
      )}
      <span className="shot-name">{shot.name}</span>
    </button>
  )
}

export function Shots() {
  const [versions, setVersions] = useState<string[]>([])
  const [version, setVersion] = useState(localStorage.getItem('daylo.shots.version') ?? '')
  const [list, setList] = useState<Shot[]>([])
  const [open, setOpen] = useState<string | null>(null)
  const [zoom, setZoom] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  const online = hasTauri()

  useEffect(() => {
    if (!online) return
    void listVersions()
      .then((v) => {
        const ids = v.filter((x) => x.type === 'release' || x.loader).map((x) => x.id)
        setVersions(ids)
        const stored = localStorage.getItem('daylo.shots.version')
        const id = stored && ids.includes(stored) ? stored : (ids[0] ?? '')
        setVersion(id)
      })
      .catch((e: unknown) => setError(String(e)))
  }, [online])

  useEffect(() => {
    if (!online || !version) return
    localStorage.setItem('daylo.shots.version', version)
    setBusy(true)
    void shotsList(version)
      .then(setList)
      .catch(() => setList([]))
      .finally(() => setBusy(false))
  }, [version, online])

  useEffect(() => {
    if (!open) {
      setZoom('')
      return
    }
    void shotsRead(version, open)
      .then(setZoom)
      .catch((e: unknown) => setError(String(e)))
  }, [open, version])

  const remove = async (name: string) => {
    setError('')
    try {
      await shotsDelete(version, name)
      setOpen(null)
      setList((l) => l.filter((s) => s.name !== name))
    } catch (e) {
      setError(String(e))
    }
  }

  const pick = (id: string) => {
    setVersion(id)
    setOpen(null)
  }

  return (
    <div className="screen shots-screen">
      <div className="shots-head">
        <h1 className="retro-title">Скриншоты</h1>
        <DropdownMenu>
          <DropdownMenuTrigger>{version || 'Сборка…'}</DropdownMenuTrigger>
          <DropdownMenuContent className="version-list">
            {versions.map((id) => (
              <DropdownMenuItem key={id}>
                <div className="version-item" onClick={() => pick(id)}>
                  {id}
                </div>
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
        <Button
          onClick={() =>
            void shotsOpen(version).catch((e: unknown) => setError(String(e)))
          }
          disabled={!version}
        >
          Открыть папку
        </Button>
      </div>

      {!online && <p className="muted">Ядро не запущено — режим браузера.</p>}
      {busy && <p className="muted">Читаем папку…</p>}
      {error && <p className="error">{error}</p>}

      {version && !busy && list.length === 0 && (
        <Card className="play-card">
          <p className="muted">
            Скриншотов нет. Снимки из игры (клавиша F2) появятся здесь автоматически.
          </p>
        </Card>
      )}

      <div className="shots-grid">
        {list.slice(0, MAX_SHOWN).map((s) => (
          <ShotCard key={s.name} version={version} shot={s} onOpen={() => setOpen(s.name)} />
        ))}
      </div>
      {list.length > MAX_SHOWN && (
        <p className="muted">Показаны последние {MAX_SHOWN}. Остальные — в папке снимков.</p>
      )}

      {open && (
        <div className="modal-backdrop" onClick={() => setOpen(null)}>
          <div className="shot-view" onClick={(e) => e.stopPropagation()}>
            <div className="shot-view-head">
              <span className="shot-name">{open}</span>
              <Button bg="#a03030" onClick={() => void remove(open)}>
                Удалить
              </Button>
            </div>
            {zoom && <img className="shot-full" src={zoom} alt={open} />}
          </div>
        </div>
      )}
    </div>
  )
}
