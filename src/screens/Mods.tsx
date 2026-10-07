import { useEffect, useState } from 'react'
import {
  Button,
  Card,
  Input,
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from 'pixel-retroui'
import { listVersions, modrinthSearch, modrinthVersions, modrinthInstall, curseforgeSearch, curseforgeVersions, curseforgeInstall, modsList, modsDelete, modsUpdates, modsUpdate, type VersionEntry, type SearchHit, type ModVersion, type ModUpdate } from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'

type View = 'mine' | 'catalog'
type Source = 'modrinth' | 'curseforge'

function fmtDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`
  return String(n)
}

export function Mods() {
  const [versions, setVersions] = useState<VersionEntry[]>([])
  const [instance, setInstance] = useState(localStorage.getItem('daylo.mods.version') ?? '')
  const [view, setView] = useState<View>('mine')
  const [src, setSrc] = useState<Source>('modrinth')
  const [query, setQuery] = useState('')
  const [hits, setHits] = useState<SearchHit[]>([])
  const [picked, setPicked] = useState<SearchHit | null>(null)
  const [modVersions, setModVersions] = useState<ModVersion[]>([])
  const [installed, setInstalled] = useState<string[]>([])
  const [updates, setUpdates] = useState<ModUpdate[]>([])
  const [checkBusy, setCheckBusy] = useState(false)
  const [updating, setUpdating] = useState('')
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')

  const online = hasTauri()

  const refreshInstalled = (id: string) => {
    if (!online || !id) return
    void modsList(id)
      .then(setInstalled)
      .catch(() => setInstalled([]))
  }

  const check = async () => {
    if (!online || !instance) return
    setCheckBusy(true)
    setError('')
    try {
      const list = await modsUpdates(instance)
      setUpdates(list)
      refreshInstalled(instance)
    } catch (e) {
      setError(String(e))
    } finally {
      setCheckBusy(false)
    }
  }

  const applyUpdate = async (u: ModUpdate) => {
    if (!online || !instance) return
    setUpdating(u.file)
    setError('')
    try {
      await modsUpdate(instance, u.file, u.latest_version_id)
      setUpdates((list) => list.filter((x) => x.file !== u.file))
      refreshInstalled(instance)
    } catch (e) {
      setError(String(e))
    } finally {
      setUpdating('')
    }
  }

  const applyAll = async () => {
    for (const u of [...updates]) {
      await applyUpdate(u)
    }
  }

  useEffect(() => {
    if (!online) return
    void listVersions()
      .then((list) => {
        setVersions(list)
        const stored = localStorage.getItem('daylo.mods.version')
        const fallback = list.find((v) => v.type === 'release')?.id ?? ''
        const id = stored && list.some((v) => v.id === stored) ? stored : fallback
        setInstance(id)
        refreshInstalled(id)
      })
      .catch((e) => setError(String(e)))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const pickInstance = (id: string) => {
    localStorage.setItem('daylo.mods.version', id)
    setInstance(id)
    setUpdates([])
    refreshInstalled(id)
  }

  const search = async () => {
    if (!online || !query.trim()) return
    setBusy('search')
    setError('')
    setPicked(null)
    setModVersions([])
    try {
      setHits(
        src === 'modrinth'
          ? await modrinthSearch(query.trim(), instance)
          : await curseforgeSearch(query.trim(), instance),
      )
      if (!instance) setError('выберите версию игры, чтобы отфильтровать совместимые моды')
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy('')
    }
  }

  const pickMod = async (hit: SearchHit) => {
    setPicked(hit)
    setModVersions([])
    if (!online) return
    setBusy(`mod:${hit.project_id}`)
    setError('')
    try {
      setModVersions(
        src === 'modrinth'
          ? await modrinthVersions(hit.project_id, instance)
          : await curseforgeVersions(hit.project_id, instance),
      )
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy('')
    }
  }

  const install = async (mv: ModVersion) => {
    if (!online || !instance) return
    setBusy(`install:${mv.version_id}`)
    setError('')
    try {
      if (src === 'modrinth') {
        await modrinthInstall(instance, mv.version_id)
      } else if (picked) {
        await curseforgeInstall(instance, picked.project_id, mv.version_id)
      }
      refreshInstalled(instance)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy('')
    }
  }

  const uninstall = async (file: string) => {
    if (!online || !instance) return
    setError('')
    try {
      await modsDelete(instance, file)
      refreshInstalled(instance)
    } catch (e) {
      setError(String(e))
    }
  }

  const releases = versions.filter((v) => v.type === 'release')

  const instancePicker = (
    <DropdownMenu>
      <DropdownMenuTrigger>{instance || 'Версия…'}</DropdownMenuTrigger>
      <DropdownMenuContent className="version-list">
        {releases.map((v) => (
          <DropdownMenuItem key={v.id}>
            <div className="version-item" onClick={() => pickInstance(v.id)}>
              {v.id}
            </div>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )

  return (
    <div className="screen mods-screen">
      <div className="mods-head">
        <h1 className="retro-title">Моды</h1>
        <div className="tabs">
          <button
            className={view === 'mine' ? 'tab on' : 'tab'}
            onClick={() => setView('mine')}
          >
            Мои моды{installed.length > 0 ? ` · ${installed.length}` : ''}
          </button>
          <button
            className={view === 'catalog' ? 'tab on' : 'tab'}
            onClick={() => setView('catalog')}
          >
            Каталог
          </button>
        </div>
      </div>

      {error && <p className="error">{error}</p>}
      {!online && <p className="muted">Ядро не запущено — режим браузера.</p>}

      {view === 'mine' && (
        <Card className="mods-installed">
          <div className="mods-head">
            {instancePicker}
            <Button onClick={() => void check()} disabled={!online || !instance || checkBusy}>
              {checkBusy ? 'Проверяем…' : 'Проверить обновления'}
            </Button>
          </div>
          {updates.length > 0 && (
            <p className="muted mods-updates-note">
              Доступно обновлений: {updates.length}.{' '}
              <button className="link-btn" onClick={() => void applyAll()} disabled={updating !== ''}>
                Обновить все
              </button>
            </p>
          )}
          {installed.length === 0 ? (
            <div className="mods-empty">
              <div className="mods-empty-icon" />
              <p className="muted">
                В сборке {instance || '—'} пока нет модов.
                <br />
                Загляни в «Каталог» — Modrinth и CurseForge под рукой.
              </p>
              <Button onClick={() => setView('catalog')}>Открыть каталог</Button>
            </div>
          ) : (
            <div className="mods-version-list">
              {installed.map((file) => {
                const upd = updates.find((u) => u.file === file)
                return (
                  <div className="mods-version-row" key={file}>
                    <div className="mods-version-info">
                      <span className="mods-version-name">{file}</span>
                      {upd && (
                        <span className="mods-update-hint">
                          {upd.current_version || 'старая версия'} → {upd.latest_version_number}
                        </span>
                      )}
                    </div>
                    <div className="mods-row-actions">
                      {upd && (
                        <Button
                          bg="#2f7d4f"
                          onClick={() => void applyUpdate(upd)}
                          disabled={updating !== ''}
                        >
                          {updating === upd.file ? 'Обновляем…' : 'Обновить'}
                        </Button>
                      )}
                      <Button bg="#a03030" onClick={() => void uninstall(file)}>
                        Удалить
                      </Button>
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </Card>
      )}

      {view === 'catalog' && (
        <>
          <Card className="mods-search">
            <div className="tabs source-tabs">
              <button
                className={src === 'modrinth' ? 'tab on' : 'tab'}
                onClick={() => {
                  setSrc('modrinth')
                  setPicked(null)
                  setModVersions([])
                }}
              >
                Modrinth
              </button>
              <button
                className={src === 'curseforge' ? 'tab on' : 'tab'}
                onClick={() => {
                  setSrc('curseforge')
                  setPicked(null)
                  setModVersions([])
                }}
              >
                CurseForge
              </button>
            </div>
            <div className="play-row">
              <Input
                className="mods-query"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && void search()}
                placeholder={src === 'modrinth' ? 'Поиск на Modrinth…' : 'Поиск на CurseForge…'}
              />
              {instancePicker}
              <Button onClick={() => void search()} disabled={!online || busy === 'search'}>
                Найти
              </Button>
            </div>
          </Card>

          {picked && (
            <Card className="mods-versions">
              <h2 className="retro-title">Версии: {picked.title}</h2>
              {busy === `mod:${picked.project_id}` && <p className="muted">Загрузка…</p>}
              {modVersions.length === 0 && busy !== `mod:${picked.project_id}` && (
                <p className="muted">Под эту версию игры подходящих сборок нет.</p>
              )}
              <div className="mods-version-list">
                {modVersions.map((mv) => (
                  <div className="mods-version-row" key={mv.version_id}>
                    <div className="mods-version-info">
                      <span className="mods-version-name">{mv.version_number}</span>
                      <span className="muted">
                        {mv.loaders.join(', ') || ' vanilla'} · {mv.file_name}
                      </span>
                    </div>
                    <Button
                      bg="#2f7d4f"
                      onClick={() => void install(mv)}
                      disabled={!instance || busy === `install:${mv.version_id}`}
                    >
                      {busy === `install:${mv.version_id}` ? 'Ставим…' : 'Установить'}
                    </Button>
                  </div>
                ))}
              </div>
            </Card>
          )}

          <div className="mods-grid">
            {hits.map((h) => (
              <Card
                key={h.project_id}
                className={picked?.project_id === h.project_id ? 'mod-card on' : 'mod-card'}
              >
                <div className="mod-head" onClick={() => void pickMod(h)}>
                  {h.icon_url ? (
                    <img
                      className="mod-icon"
                      src={h.icon_url}
                      alt=""
                      onError={(e) => (e.currentTarget.style.display = 'none')}
                    />
                  ) : (
                    <div className="mod-icon empty" />
                  )}
                  <div>
                    <div className="mod-title">{h.title}</div>
                    <div className="muted">{fmtDownloads(h.downloads)} скачиваний</div>
                  </div>
                </div>
                <p className="mod-desc">{h.description}</p>
              </Card>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
