import { useEffect, useRef, useState } from 'react'
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
import { useGame } from '../state/game'
import { useT, type TKey } from '../i18n'
import { useAccount } from '../state/account'
import { useInstances } from '../state/instances'
import { useUi } from '../state/ui'
import { hasTauri } from '../ipc/tauri'
import {
  installLoader,
  loaderBuilds,
  skinSave,
  skinDelete,
  skinLoad,
  elyUploadSkin,
  type LoaderBuild,
} from '../ipc/commands'
import { SkinView } from '../components/SkinView'
import { Servers } from '../components/Servers'
import { News } from '../components/News'
import { InstanceCard } from '../components/InstanceCard'

type LoaderTab = 'fabric' | 'quilt' | 'forge' | 'neoforge'

const LOADERS: { id: LoaderTab; label: string }[] = [
  { id: 'fabric', label: 'Fabric' },
  { id: 'quilt', label: 'Quilt' },
  { id: 'forge', label: 'Forge' },
  { id: 'neoforge', label: 'NeoForge' },
]

const TYPE_KEY: Record<string, TKey> = {
  release: 'type.release',
  snapshot: 'type.snapshot',
  old_beta: 'type.old_beta',
  old_alpha: 'type.old_alpha',
}

export function Play() {
  const {
    versions,
    version,
    nick,
    busy,
    stage,
    progress,
    playing,
    logs,
    error,
    init,
    setVersion,
    setNick,
    play,
    stop,
  } = useGame()
  const inst = useInstances()
  const logRef = useRef<HTMLPreElement>(null)
  const [loaderOpen, setLoaderOpen] = useState(false)
  const [tab, setTab] = useState<LoaderTab>('fabric')
  const [builds, setBuilds] = useState<LoaderBuild[]>([])
  const [build, setBuild] = useState('')
  const [buildsBusy, setBuildsBusy] = useState(false)
  const [installing, setInstalling] = useState(false)
  const [skinKey, setSkinKey] = useState(0)
  const [skinError, setSkinError] = useState('')
  // Под аккаунтом (ely, msa или оффлайн-профилем) ник задаёт аккаунт, поле не редактируется
  const accName = useAccount((s) => s.info?.name ?? '')
  const accMode = accName !== ''
  const accEly = useAccount((s) => s.info?.mode === 'ely')

  useEffect(() => {
    void init()
  }, [init])

  // Слушатели событий сборок и первый список инстансов — в общем сторе
  useEffect(() => {
    useInstances.getState().init()
  }, [])

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight })
  }, [logs])

  const releases = versions.filter((v) => v.type === 'release')
  const profiles = versions.filter((v) => v.loader !== '')
  // Сборки загрузчика ставятся под ванильную версию: текущую или свежий релиз
  const mcBase = releases.some((v) => v.id === version) ? version : (releases[0]?.id ?? '')
  const pick = builds.find((b) => b.version === build)

  useEffect(() => {
    if (!loaderOpen || !hasTauri() || !mcBase) {
      setBuilds([])
      setBuild('')
      return
    }
    let dead = false
    setBuildsBusy(true)
    void loaderBuilds(tab, mcBase)
      .then((list) => {
        if (dead) return
        setBuilds(list)
        setBuild(list.find((b) => b.recommended)?.version ?? list[0]?.version ?? '')
      })
      .catch(() => {
        if (!dead) setBuilds([])
      })
      .finally(() => !dead && setBuildsBusy(false))
    return () => {
      dead = true
    }
  }, [tab, mcBase, loaderOpen])

  // Установка загрузчика заканчивается событием готово/ошибки
  useEffect(() => {
    if (installing && (stage === 'готово' || error)) setInstalling(false)
  }, [stage, error, installing])

  const installBuild = async () => {
    if (!mcBase || !build) return
    setInstalling(true)
    setSkinError('')
    try {
      await installLoader(tab, mcBase, build)
    } catch (e) {
      setInstalling(false)
      setSkinError(String(e))
    }
  }

  const onSkinFile = async (file: File | undefined) => {
    const name = nick.trim()
    if (!file || !name) return
    setSkinError('')
    try {
      const b64 = await new Promise<string>((resolve, reject) => {
        const r = new FileReader()
        r.onload = () => resolve(String(r.result).split(',')[1] ?? '')
        r.onerror = () => reject(new Error(t('error.readFile')))
        r.readAsDataURL(file)
      })
      await skinSave(name, b64)
      setSkinKey((k) => k + 1)
    } catch (e) {
      setSkinError(String(e))
    }
  }

  const removeSkin = async () => {
    const name = nick.trim()
    if (!name) return
    setSkinError('')
    try {
      await skinDelete(name)
      setSkinKey((k) => k + 1)
    } catch (e) {
      setSkinError(String(e))
    }
  }

  // Выбранный PNG уезжает в аккаунт Ely.by — скин появится и на серверах
  const [elyModel, setElyModel] = useState<'classic' | 'slim'>('classic')
  const [elyBusy, setElyBusy] = useState(false)
  const [elySaved, setElySaved] = useState(false)
  const uploadEly = async () => {
    const name = accMode ? accName : nick.trim()
    if (!name) return
    setElyBusy(true)
    setSkinError('')
    setElySaved(false)
    try {
      const b64 = await skinLoad(name)
      if (!b64) throw new Error(t('error.skinFile'))
      await elyUploadSkin(elyModel, b64)
      setElySaved(true)
      setSkinKey((k) => k + 1)
    } catch (e) {
      setSkinError(String(e))
    } finally {
      setElyBusy(false)
    }
  }

  // Скачанные версии: ванила сверху (свежие раньше), профили загрузчиков в конце
  const installed = versions
    .filter((v) => v.installed)
    .sort((a, b) => {
      if (a.loader !== b.loader) return a.loader === '' ? -1 : 1
      return b.release_time.localeCompare(a.release_time)
    })
  const pickerList = [...releases, ...profiles]
  const t = useT()
  const versionLabel = version && versions.some((v) => v.id === version) ? version : t('placeholder.version')

  const packs = inst.instances.filter((i) => i.kind === 'modpack')
  const own = inst.instances.filter((i) => i.kind !== 'modpack')
  const launchBusy = inst.launching !== '' || inst.verifying !== ''
  const verifyBusy = launchBusy || inst.progress !== ''
  const gameCardBusy = busy || playing || installing

  const playVersion = (id: string) => {
    if (gameCardBusy) return
    setVersion(id)
    void play()
  }

  const goPacks = (view: 'mine' | 'catalog') => {
    sessionStorage.setItem('daylo.packs.view', view)
    useUi.getState().setScreen('packs')
  }

  const statusVisible =
    busy ||
    installing ||
    stage !== '' ||
    error !== '' ||
    inst.progress !== '' ||
    inst.stage !== '' ||
    inst.error !== '' ||
    !hasTauri()

  const instanceGrid = (list: typeof inst.instances) =>
    list.map((info) => (
      <InstanceCard
        key={info.id}
        info={info}
        launching={inst.launching === info.id}
        verifying={inst.verifying === info.id}
        launchDisabled={launchBusy}
        verifyDisabled={verifyBusy}
        onLaunch={() => void inst.launch(info.id)}
        onVerify={() => void inst.verify(info.id)}
        onRemove={() => void inst.remove(info)}
      />
    ))

  return (
    <div className="home-screen">
      <Card className="topbar">
        {accMode ? (
          <span className="nick ely-nick" title={t('play.accountAuth')}>
            {accName}
          </span>
        ) : (
          <Input
            className="nick"
            value={nick}
            onChange={(e) => setNick(e.target.value)}
            placeholder={t('play.nick')}
            maxLength={16}
          />
        )}
        <DropdownMenu>
          <DropdownMenuTrigger>{versionLabel}</DropdownMenuTrigger>
          <DropdownMenuContent className="version-list">
            {pickerList.map((v) => (
              <DropdownMenuItem key={v.id}>
                <div className="version-item" onClick={() => setVersion(v.id)}>
                  {v.id}
                  {v.installed ? t('suffix.installed') : ''}
                </div>
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
        {playing ? (
          <Button bg="#a03030" onClick={() => void stop()}>
            {t('action.stop')}
          </Button>
        ) : (
          <Button
            className="btn-cta"
            bg="#2f7d4f"
            onClick={() => void play()}
            disabled={gameCardBusy || !version}
          >
            {busy ? t('busy.preparing') : t('action.play')}
          </Button>
        )}
      </Card>

      {statusVisible && (
        <div className="home-status">
          {(busy || installing) && <ProgressBar className="pbar-fx" progress={progress} size="lg" />}
          {inst.progress && (
            <Card className="pack-progress">
              <p className="muted">{inst.progress}</p>
              <ProgressBar className="pbar-fx" progress={inst.packPct} size="lg" />
            </Card>
          )}
          {stage && <p className="muted">{stage}</p>}
          {inst.stage && <p className="muted">{inst.stage}</p>}
          {error && <p className="error">{error}</p>}
          {inst.error && <p className="error">{inst.error}</p>}
          {!hasTauri() && <p className="muted">{t('core.offline')}</p>}
        </div>
      )}

      <div className="home-body">
        <div className="library">
          <section className="lib-section">
            <div className="lib-head">
              <h2 className="retro-title">{t('lib.packs')}</h2>
              {packs.length > 0 && <span className="muted">{packs.length}</span>}
              <button className="link-btn lib-action" onClick={() => goPacks('catalog')}>
                {t('lib.packsAction')}
              </button>
            </div>
            {packs.length === 0 ? (
              <div className="muted lib-empty">
                <span className="slot-cross" aria-hidden="true" />
                <span>{t('lib.packsEmpty')}</span>
              </div>
            ) : (
              <div className="mods-grid">{instanceGrid(packs)}</div>
            )}
          </section>

          <section className="lib-section">
            <div className="lib-head">
              <h2 className="retro-title">{t('lib.own')}</h2>
              {own.length > 0 && <span className="muted">{own.length}</span>}
              <button className="link-btn lib-action" onClick={() => goPacks('mine')}>
                {t('lib.ownAction')}
              </button>
            </div>
            {own.length === 0 ? (
              <div className="muted lib-empty">
                <span className="slot-cross" aria-hidden="true" />
                <span>{t('lib.ownEmpty')}</span>
              </div>
            ) : (
              <div className="mods-grid">{instanceGrid(own)}</div>
            )}
          </section>

          <section className="lib-section">
            <div className="lib-head">
              <h2 className="retro-title">{t('lib.versions')}</h2>
              {installed.length > 0 && <span className="muted">{installed.length}</span>}
              <button className="link-btn lib-action" onClick={() => setLoaderOpen(!loaderOpen)}>
                {loaderOpen ? t('lib.hideLoader') : t('lib.installLoader')}
              </button>
            </div>
            {installed.length === 0 ? (
              <div className="muted lib-empty">
                <span className="slot-cross" aria-hidden="true" />
                <span>{t('lib.versionsEmpty')}</span>
              </div>
            ) : (
              <div className="ver-grid">
                {installed.map((v) => (
                  <Card
                    key={v.id}
                    className={v.id === version ? 'mod-card ver-card on' : 'mod-card ver-card'}
                  >
                    <div className="ver-head" onClick={() => setVersion(v.id)} title={t('play.selectVersion')}>
                      <div className="ver-name">{v.id}</div>
                      <span className="pack-badge">{v.loader || t(TYPE_KEY[v.type] ?? v.type)}</span>
                    </div>
                    <div className="mods-row-actions">
                      <Button bg="#2f7d4f" onClick={() => playVersion(v.id)} disabled={gameCardBusy}>
                        {v.id === version && busy ? t('busy.preparing') : t('action.play')}
                      </Button>
                      <Button
                        bg="#a03030"
                        onClick={() => void inst.removeVersion(v.id)}
                        disabled={gameCardBusy}
                      >
                        {t('action.delete')}
                      </Button>
                    </div>
                  </Card>
                ))}
              </div>
            )}
            {loaderOpen && (
              <div className="loader-install">
                <div className="tabs source-tabs">
                  {LOADERS.map((t) => (
                    <button
                      key={t.id}
                      className={tab === t.id ? 'tab on' : 'tab'}
                      onClick={() => setTab(t.id)}
                    >
                      {t.label}
                    </button>
                  ))}
                </div>
                <div className="play-row">
                  {builds.length > 0 ? (
                    <>
                      <span className="muted loader-hint">
                        {t('loader.hint', { loader: tab, mc: mcBase })}
                      </span>
                      <DropdownMenu>
                        <DropdownMenuTrigger>
                          {pick
                            ? pick.recommended
                              ? `${build}${t('suffix.rec')}`
                              : build
                            : t('placeholder.build')}
                        </DropdownMenuTrigger>
                        <DropdownMenuContent className="version-list">
                          {builds.map((b) => (
                            <DropdownMenuItem key={b.version}>
                              <div className="version-item" onClick={() => setBuild(b.version)}>
                                {b.version}
                                {b.recommended ? t('suffix.rec') : ''}
                                {!b.stable ? t('suffix.test') : ''}
                              </div>
                            </DropdownMenuItem>
                          ))}
                        </DropdownMenuContent>
                      </DropdownMenu>
                      <Button
                        bg="#2f7d4f"
                        onClick={() => void installBuild()}
                        disabled={installing || buildsBusy}
                      >
                        {installing ? t('busy.installing') : t('action.install')}
                      </Button>
                    </>
                  ) : (
                    <span className="muted loader-hint">
                      {buildsBusy
                        ? t('loader.checking', { loader: tab })
                        : t('loader.none', { loader: tab, mc: mcBase })}
                    </span>
                  )}
                </div>
              </div>
            )}
          </section>

          {logs.length > 0 && (
            <pre className="log" ref={logRef}>
              {logs.join('\n')}
            </pre>
          )}
        </div>

        <div className="side-column">
          <Card className="skin-card">
            <h2 className="retro-title">{t('skin.title')}</h2>
            <SkinView nick={accMode ? accName : nick} refreshKey={skinKey} />
            <label className="skin-buttons">
              <input
                type="file"
                accept="image/png"
                className="skin-file"
                onChange={(e) => void onSkinFile(e.target.files?.[0])}
              />
              <span>{t('skin.file')}</span>
            </label>
            <Button
              bg="#a03030"
              onClick={() => void removeSkin()}
              disabled={!accMode && !nick.trim()}
            >
              {t('action.remove')}
            </Button>
            {accEly && (
              <div className="skin-ely">
                <div className="play-row">
                  <DropdownMenu>
                    <DropdownMenuTrigger>
                      {elyModel === 'slim' ? 'slim · 3px' : 'classic · 4px'}
                    </DropdownMenuTrigger>
                    <DropdownMenuContent className="version-list">
                      <DropdownMenuItem>
                        <div className="version-item" onClick={() => setElyModel('classic')}>
                          classic · 4px
                        </div>
                      </DropdownMenuItem>
                      <DropdownMenuItem>
                        <div className="version-item" onClick={() => setElyModel('slim')}>
                          slim · 3px
                        </div>
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                  <Button bg="#2f7d4f" onClick={() => void uploadEly()} disabled={elyBusy}>
                    {elyBusy ? t('busy.uploading') : t('skin.toEly')}
                  </Button>
                </div>
                {elySaved && <p className="muted">{t('skin.saved')}</p>}
              </div>
            )}
            {skinError && <p className="error">{skinError}</p>}
            <p className="muted skin-note">{t('skin.note')}</p>
          </Card>
          <Servers version={version} />
          <News />
        </div>
      </div>
    </div>
  )
}
