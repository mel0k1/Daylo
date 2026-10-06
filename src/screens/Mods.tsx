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
import { listVersions, modrinthSearch, modrinthVersions, modrinthInstall, modsList, modsDelete, type VersionEntry, type SearchHit, type ModVersion } from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'

function fmtDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`
  return String(n)
}

export function Mods() {
  const [versions, setVersions] = useState<VersionEntry[]>([])
  const [instance, setInstance] = useState(localStorage.getItem('daylo.mods.version') ?? '')
  const [query, setQuery] = useState('')
  const [hits, setHits] = useState<SearchHit[]>([])
  const [picked, setPicked] = useState<SearchHit | null>(null)
  const [modVersions, setModVersions] = useState<ModVersion[]>([])
  const [installed, setInstalled] = useState<string[]>([])
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')

  const online = hasTauri()

  const refreshInstalled = (id: string) => {
    if (!online || !id) return
    void modsList(id)
      .then(setInstalled)
      .catch(() => setInstalled([]))
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
    refreshInstalled(id)
  }

  const search = async () => {
    if (!online || !query.trim()) return
    setBusy('search')
    setError('')
    setPicked(null)
    setModVersions([])
    try {
      setHits(await modrinthSearch(query.trim(), instance))
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
      setModVersions(await modrinthVersions(hit.project_id, instance))
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
      await modrinthInstall(instance, mv.version_id)
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

  return (
    <div className="screen mods-screen">
      <h1 className="retro-title">Моды</h1>

      <Card className="mods-search">
        <div className="play-row">
          <Input
            className="mods-query"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && void search()}
            placeholder="Поиск на Modrinth…"
          />
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
          <Button onClick={() => void search()} disabled={!online || busy === 'search'}>
            Найти
          </Button>
        </div>
        {!online && <p className="muted">Ядро не запущено — режим браузера.</p>}
        {error && <p className="error">{error}</p>}
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

      {installed.length > 0 && (
        <Card className="mods-installed">
          <h2 className="retro-title">Установлено · {instance}</h2>
          <div className="mods-version-list">
            {installed.map((file) => (
              <div className="mods-version-row" key={file}>
                <span className="mods-version-name">{file}</span>
                <Button bg="#a03030" onClick={() => void uninstall(file)}>
                  Удалить
                </Button>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  )
}
