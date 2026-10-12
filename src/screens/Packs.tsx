import { useEffect, useState } from 'react'
import {
  Button,
  Card,
  Input,
  ProgressBar,
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from 'pixel-retroui'
import {
  packSearch,
  packVersions,
  packInstall,
  instanceCreate,
  listVersions,
  loaderBuilds,
  type VersionEntry,
  type LoaderBuild,
  type PackHit,
  type PackVersionInfo,
} from '../ipc/commands'
import { onPackInstalled } from '../ipc/events'
import { hasTauri } from '../ipc/tauri'
import { AiBuild } from '../components/AiBuild'
import { InstanceCard, SOURCE_LABEL } from '../components/InstanceCard'
import { useInstances } from '../state/instances'

type View = 'mine' | 'catalog'
type Source = 'modrinth' | 'curseforge' | 'ftb'

const SOURCES: { id: Source; label: string }[] = [
  { id: 'modrinth', label: 'Modrinth' },
  { id: 'curseforge', label: 'CurseForge' },
  { id: 'ftb', label: 'FTB' },
]

type OwnLoader = 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge'

const OWN_LOADERS: { id: OwnLoader; label: string }[] = [
  { id: 'vanilla', label: 'Ванила' },
  { id: 'fabric', label: 'Fabric' },
  { id: 'quilt', label: 'Quilt' },
  { id: 'forge', label: 'Forge' },
  { id: 'neoforge', label: 'NeoForge' },
]

// Сборка «своими руками»: версия игры, опционально загрузчик — остальное само
function BuildOwnCard({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  const [name, setName] = useState('')
  const [mcVersion, setMcVersion] = useState('')
  const [versions, setVersions] = useState<VersionEntry[]>([])
  const [loader, setLoader] = useState<OwnLoader>('vanilla')
  const [builds, setBuilds] = useState<LoaderBuild[]>([])
  const [build, setBuild] = useState('')
  const [busy, setBusy] = useState(false)
  const [pending, setPending] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    void listVersions()
      .then((list) => {
        setVersions(list)
        setMcVersion((cur) => cur || list.find((v) => v.type === 'release')?.id || '')
      })
      .catch(() => {})
  }, [])

  useEffect(() => {
    if (loader === 'vanilla' || !mcVersion) {
      setBuilds([])
      setBuild('')
      return
    }
    let dead = false
    void loaderBuilds(loader, mcVersion)
      .then((list) => {
        if (dead) return
        setBuilds(list)
        setBuild(list.find((b) => b.recommended)?.version ?? list[0]?.version ?? '')
      })
      .catch(() => {
        if (!dead) {
          setBuilds([])
          setBuild('')
        }
      })
    return () => {
      dead = true
    }
  }, [loader, mcVersion])

  // Итог приходит событием pack-installed — и здесь, и в общем списке
  useEffect(() => {
    void onPackInstalled((p) => {
      setPending(false)
      setBusy(false)
      if (p.error) setError(p.error)
    })
  }, [])

  const releases = versions.filter((v) => v.type === 'release')

  const create = async () => {
    if (busy || !mcVersion) return
    setBusy(true)
    setError('')
    try {
      await instanceCreate(name.trim() || 'Моя сборка', mcVersion, loader, build)
      setPending(true)
    } catch (e) {
      setError(String(e))
      setBusy(false)
    }
  }

  return (
    <Card className="mods-installed build-own">
      <div className="mods-head">
        <Button onClick={onToggle}>{open ? 'Свернуть' : 'Собрать свою'}</Button>
        <p className="muted mirrors-note">
          Ванила или загрузчик под выбранную версию — качается и ставится сама.
        </p>
      </div>
      {open && (
        <>
          <div className="play-row">
            <Input
              className="mods-query"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Название сборки"
              maxLength={32}
            />
            <DropdownMenu>
              <DropdownMenuTrigger>{mcVersion || 'Версия…'}</DropdownMenuTrigger>
              <DropdownMenuContent className="version-list">
                {releases.map((v) => (
                  <DropdownMenuItem key={v.id}>
                    <div className="version-item" onClick={() => setMcVersion(v.id)}>
                      {v.id}
                    </div>
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          <div className="tabs source-tabs">
            {OWN_LOADERS.map((l) => (
              <button
                key={l.id}
                className={loader === l.id ? 'tab on' : 'tab'}
                onClick={() => setLoader(l.id)}
              >
                {l.label}
              </button>
            ))}
          </div>
          <div className="play-row">
            {loader !== 'vanilla' &&
              (builds.length > 0 ? (
                <DropdownMenu>
                  <DropdownMenuTrigger>
                    {build ? (builds.find((b) => b.version === build)?.recommended ? `${build} · рекоменд.` : build) : 'Сборка…'}
                  </DropdownMenuTrigger>
                  <DropdownMenuContent className="version-list">
                    {builds.map((b) => (
                      <DropdownMenuItem key={b.version}>
                        <div className="version-item" onClick={() => setBuild(b.version)}>
                          {b.version}
                          {b.recommended ? ' · рекоменд.' : ''}
                          {!b.stable ? ' · тест' : ''}
                        </div>
                      </DropdownMenuItem>
                    ))}
                  </DropdownMenuContent>
                </DropdownMenu>
              ) : (
                <span className="muted loader-hint">Сборок {loader} под {mcVersion} нет</span>
              ))}
            <Button
              bg="#2f7d4f"
              onClick={() => void create()}
              disabled={busy || pending || !mcVersion || (loader !== 'vanilla' && !build)}
            >
              {pending ? 'Ставим…' : 'Создать'}
            </Button>
          </div>
          {pending && <p className="muted">Ставим сборку — прогресс виден на экране «Играть».</p>}
          {error && <p className="error">{error}</p>}
        </>
      )}
    </Card>
  )
}

function fmtDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`
  return String(n)
}

export function Packs() {
  const [view, setView] = useState<View>(() => {
    // Переход из библиотеки («Каталог сборок →») открывает нужный вид
    const saved = sessionStorage.getItem('daylo.packs.view')
    sessionStorage.removeItem('daylo.packs.view')
    return saved === 'catalog' ? 'catalog' : 'mine'
  })
  const [builder, setBuilder] = useState(false)

  const [source, setSource] = useState<Source>('modrinth')
  const [query, setQuery] = useState('')
  const [hits, setHits] = useState<PackHit[]>([])
  const [picked, setPicked] = useState<PackHit | null>(null)
  const [packVs, setPackVs] = useState<PackVersionInfo[]>([])
  const [versions, setVersions] = useState<VersionEntry[]>([])
  const [gv, setGv] = useState(localStorage.getItem('daylo.packs.version') ?? '')
  const [busy, setBusy] = useState('')
  // Ошибки каталога: поиск, выбор, установка
  const [error, setError] = useState('')

  const inst = useInstances()
  const online = hasTauri()

  useEffect(() => {
    useInstances.getState().init()
    void listVersions()
      .then((list) => {
        setVersions(list)
        const stored = localStorage.getItem('daylo.packs.version')
        const fallback = list.find((v) => v.type === 'release')?.id ?? ''
        setGv(stored && list.some((v) => v.id === stored) ? stored : fallback)
      })
      .catch(() => {})
    // Установка из каталога завершилась — показываем «Мои сборки»
    void onPackInstalled((p) => {
      if (!p.error) setView('mine')
    })
  }, [])

  const pickGv = (id: string) => {
    localStorage.setItem('daylo.packs.version', id)
    setGv(id)
  }

  const search = async () => {
    if (!online) return
    setBusy('search')
    setError('')
    setPicked(null)
    setPackVs([])
    try {
      setHits(await packSearch(source, query.trim(), source === 'ftb' ? '' : gv))
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy('')
    }
  }

  const pickPack = async (hit: PackHit) => {
    setPicked(hit)
    setPackVs([])
    if (!online) return
    setBusy(`pack:${hit.id}`)
    setError('')
    try {
      setPackVs(await packVersions(source, hit.id, source === 'ftb' ? '' : gv))
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy('')
    }
  }

  const install = async (pv: PackVersionInfo) => {
    if (!online || !picked || inst.progress) return
    setError('')
    try {
      await packInstall(source, picked.id, pv.version_id, picked.icon_url)
      // Плейсхолдер до первого pack-progress из общего стора
      useInstances.setState({ progress: 'установка сборки', packPct: 0 })
    } catch (e) {
      setError(String(e))
    }
  }

  const releases = versions.filter((v) => v.type === 'release')

  const versionPicker =
    source === 'ftb' ? null : (
      <DropdownMenu>
        <DropdownMenuTrigger>{gv || 'Версия…'}</DropdownMenuTrigger>
        <DropdownMenuContent className="version-list">
          {releases.map((v) => (
            <DropdownMenuItem key={v.id}>
              <div className="version-item" onClick={() => pickGv(v.id)}>
                {v.id}
              </div>
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    )

  return (
    <div className="screen packs-screen">
      <div className="mods-head">
        <h1 className="retro-title">Сборки</h1>
        <div className="tabs">
          <button
            className={view === 'mine' ? 'tab on' : 'tab'}
            onClick={() => setView('mine')}
          >
            Мои сборки{inst.instances.length > 0 ? ` · ${inst.instances.length}` : ''}
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
      {inst.error && <p className="error">{inst.error}</p>}
      {inst.progress && (
        <Card className="pack-progress">
          <p className="muted">{inst.progress}</p>
          <ProgressBar className="pbar-fx" progress={inst.packPct} size="lg" />
        </Card>
      )}
      {inst.stage && !inst.progress && <p className="muted">{inst.stage}</p>}
      {!online && <p className="muted">Ядро не запущено — режим браузера.</p>}

      {view === 'mine' && (
        <>
          <BuildOwnCard open={builder} onToggle={() => setBuilder(!builder)} />
          <AiBuild />
          {inst.instances.length === 0 ? (
            <Card className="mods-installed">
              <div className="mods-empty">
                <div className="mods-empty-icon" />
                <p className="muted">
                  Сборок пока нет.
                  <br />
                  Установи готовую из каталога — Modrinth, CurseForge или FTB.
                </p>
                <Button onClick={() => setView('catalog')}>Открыть каталог</Button>
              </div>
            </Card>
          ) : (
            <div className="mods-grid">
              {inst.instances.map((info) => (
                <InstanceCard
                  key={info.id}
                  info={info}
                  launching={inst.launching === info.id}
                  verifying={inst.verifying === info.id}
                  launchDisabled={inst.launching !== '' || inst.verifying !== ''}
                  verifyDisabled={
                    inst.launching !== '' || inst.verifying !== '' || inst.progress !== ''
                  }
                  onLaunch={() => void inst.launch(info.id)}
                  onVerify={() => void inst.verify(info.id)}
                  onRemove={() => void inst.remove(info)}
                />
              ))}
            </div>
          )}
        </>
      )}

      {view === 'catalog' && (
        <>
          <Card className="mods-search">
            <div className="tabs source-tabs">
              {SOURCES.map((s) => (
                <button
                  key={s.id}
                  className={source === s.id ? 'tab on' : 'tab'}
                  onClick={() => {
                    setSource(s.id)
                    setPicked(null)
                    setPackVs([])
                  }}
                >
                  {s.label}
                </button>
              ))}
            </div>
            <div className="play-row">
              <Input
                className="mods-query"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && void search()}
                placeholder={
                  source === 'ftb' ? 'Поиск в каталоге FTB…' : `Поиск сборок на ${SOURCE_LABEL[source]}…`
                }
              />
              {versionPicker}
              <Button onClick={() => void search()} disabled={!online || busy === 'search'}>
                Найти
              </Button>
            </div>
            {source === 'curseforge' && (
              <p className="muted mirrors-note">
                CurseForge идёт через публичное зеркало; если оно молчит — добавь свой ключ API в
                настройках.
              </p>
            )}
          </Card>

          {picked && (
            <Card className="mods-versions">
              <h2 className="retro-title">Версии: {picked.title}</h2>
              {busy === `pack:${picked.id}` && <p className="muted">Загрузка…</p>}
              {packVs.length === 0 && busy !== `pack:${picked.id}` && (
                <p className="muted">Под эту версию игры подходящих сборок нет.</p>
              )}
              <div className="mods-version-list">
                {packVs.map((pv) => (
                  <div className="mods-version-row" key={pv.version_id}>
                    <div className="mods-version-info">
                      <span className="mods-version-name">{pv.name}</span>
                      <span className="muted">
                        {pv.mc_versions.slice(0, 3).join(', ') || 'версии не указаны'}
                        {pv.loaders.length > 0 ? ` · ${pv.loaders.join(', ')}` : ''}
                      </span>
                    </div>
                    <Button
                      bg="#2f7d4f"
                      onClick={() => void install(pv)}
                      disabled={inst.progress !== ''}
                    >
                      Установить
                    </Button>
                  </div>
                ))}
              </div>
            </Card>
          )}

          <div className="mods-grid">
            {hits.map((h) => (
              <Card
                key={`${h.source}-${h.id}`}
                className={picked?.id === h.id ? 'mod-card on' : 'mod-card'}
              >
                <div className="mod-head" onClick={() => void pickPack(h)}>
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
