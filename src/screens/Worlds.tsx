import { useCallback, useEffect, useState } from 'react'
import { Button, Card, DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from 'pixel-retroui'
import {
  instanceList,
  worldBackupCreate,
  worldBackupDelete,
  worldBackupRestore,
  worldBackupsList,
  worldBackupsOpen,
  worldsList,
  type InstanceInfo,
  type WorldBackup,
  type WorldInfo,
} from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'

const fmtSize = (bytes: number) => {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} ГБ`
  if (bytes >= 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024 / 1024))} МБ`
  return `${Math.max(1, Math.round(bytes / 1024))} КБ`
}

const fmtDate = (secs: number) =>
  secs > 0 ? new Date(secs * 1000).toLocaleString('ru-RU') : '—'

export function Worlds() {
  const [instances, setInstances] = useState<InstanceInfo[]>([])
  const [id, setId] = useState(localStorage.getItem('daylo.worlds.instance') ?? '')
  const [worlds, setWorlds] = useState<WorldInfo[]>([])
  const [backups, setBackups] = useState<WorldBackup[]>([])
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')

  const online = hasTauri()

  useEffect(() => {
    if (!online) return
    void instanceList()
      .then((list) => {
        setInstances(list)
        const stored = localStorage.getItem('daylo.worlds.instance')
        const pick = stored && list.some((i) => i.id === stored) ? stored : (list[0]?.id ?? '')
        setId(pick)
      })
      .catch((e: unknown) => setError(String(e)))
  }, [online])

  const reload = useCallback(
    (inst: string) => {
      if (!inst) return
      setBusy('Читаем миры…')
      void Promise.all([worldsList(inst), worldBackupsList(inst)])
        .then(([w, b]) => {
          setWorlds(w)
          setBackups(b)
        })
        .catch((e: unknown) => setError(String(e)))
        .finally(() => setBusy(''))
    },
    [],
  )

  useEffect(() => {
    if (!online || !id) return
    localStorage.setItem('daylo.worlds.instance', id)
    reload(id)
  }, [id, online, reload])

  const backup = (world: string) => {
    if (!id) return
    setBusy(`Бэкап «${world}»…`)
    setError('')
    void worldBackupCreate(id, world)
      .then(() => reload(id))
      .catch((e: unknown) => setError(String(e)))
      .finally(() => setBusy(''))
  }

  const restore = (b: WorldBackup) => {
    if (!id) return
    if (!window.confirm(`Заменить мир «${b.world}» копией из бэкапа? Текущая версия сохранится рядом (keep-…).`)) return
    setBusy('Восстановление…')
    setError('')
    void worldBackupRestore(id, b.file)
      .then(() => reload(id))
      .catch((e: unknown) => setError(String(e)))
      .finally(() => setBusy(''))
  }

  const remove = (b: WorldBackup) => {
    if (!id) return
    void worldBackupDelete(id, b.file)
      .then(() => setBackups((list) => list.filter((x) => x.file !== b.file)))
      .catch((e: unknown) => setError(String(e)))
  }

  const pick = (inst: string) => setId(inst)

  const nameOf = (inst: string) => {
    const i = instances.find((x) => x.id === inst)
    return i ? (i.name || i.id) : inst
  }

  return (
    <div className="screen worlds-screen">
      <div className="shots-head">
        <h1 className="retro-title">Миры</h1>
        <DropdownMenu>
          <DropdownMenuTrigger>{id ? nameOf(id) : 'Сборка…'}</DropdownMenuTrigger>
          <DropdownMenuContent className="version-list">
            {instances.map((i) => (
              <DropdownMenuItem key={i.id}>
                <div className="version-item" onClick={() => pick(i.id)}>
                  {i.name || i.id}
                </div>
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
        <Button
          onClick={() => void worldBackupsOpen(id).catch((e: unknown) => setError(String(e)))}
          disabled={!id}
        >
          Папка бэкапов
        </Button>
      </div>

      {!online && <p className="muted">Ядро не запущено — режим браузера.</p>}
      {busy && <p className="muted">{busy}</p>}
      {error && <p className="error">{error}</p>}

      {id && !busy && worlds.length === 0 && (
        <Card className="play-card">
          <p className="muted">
            Миров нет. Запусти игру в этой сборке и создай мир — он появится здесь, и его можно будет сохранить в бэкап.
          </p>
        </Card>
      )}

      <div className="worlds-grid">
        {worlds.map((w) => (
          <Card key={w.name} className="world-card">
            <div className="world-head">
              <span className="world-name">{w.name}</span>
              <span className="pack-badge">{fmtSize(w.size)}</span>
            </div>
            <span className="muted">Играл: {fmtDate(w.modified)}</span>
            <Button bg="#2f7d4f" disabled={!!busy} onClick={() => backup(w.name)}>
              Сделать бэкап
            </Button>
          </Card>
        ))}
      </div>

      {backups.length > 0 && (
        <>
          <h2 className="retro-title backups-title">Бэкапы</h2>
          <div className="backup-list">
            {backups.map((b) => (
              <div key={b.file} className="backup-row">
                <div className="backup-info">
                  <span className="world-name">{b.world}</span>
                  <span className="muted">
                    {fmtDate(b.created)} · {fmtSize(b.size)}
                  </span>
                </div>
                <div className="backup-actions">
                  <Button bg="#2f7d4f" disabled={!!busy} onClick={() => restore(b)}>
                    Восстановить
                  </Button>
                  <Button bg="#a03030" disabled={!!busy} onClick={() => remove(b)}>
                    Удалить
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
