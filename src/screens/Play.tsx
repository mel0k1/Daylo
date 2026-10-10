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
import { useAccount } from '../state/account'
import { hasTauri } from '../ipc/tauri'
import { installLoader, loaderBuilds, skinSave, skinDelete, type LoaderBuild } from '../ipc/commands'
import { SkinView } from '../components/SkinView'
import { Servers } from '../components/Servers'
import { News } from '../components/News'

type Tab = 'vanilla' | 'fabric' | 'forge' | 'neoforge'

const TABS: { id: Tab; label: string }[] = [
  { id: 'vanilla', label: 'Ванила' },
  { id: 'fabric', label: 'Fabric' },
  { id: 'forge', label: 'Forge' },
  { id: 'neoforge', label: 'NeoForge' },
]

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
  const logRef = useRef<HTMLPreElement>(null)
  const [tab, setTab] = useState<Tab>('vanilla')
  const [builds, setBuilds] = useState<LoaderBuild[]>([])
  const [build, setBuild] = useState('')
  const [buildsBusy, setBuildsBusy] = useState(false)
  const [installing, setInstalling] = useState(false)
  const [skinKey, setSkinKey] = useState(0)
  const [skinError, setSkinError] = useState('')
  // Под аккаунтом Ely.by или Microsoft ник задаёт сервер авторизации, поле не редактируется
  const accName = useAccount((s) => (s.info && s.info.mode !== 'offline' ? s.info.name : ''))
  const accMode = accName !== ''

  useEffect(() => {
    void init()
  }, [init])

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight })
  }, [logs])

  const releases = versions.filter((v) => v.type === 'release')
  const selected = versions.find((v) => v.id === version)
  const profiles = versions.filter((v) => v.type === tab)
  // Сборки загрузчика ставятся под ванильную версию: текущую или свежий релиз
  const mcBase = releases.some((v) => v.id === version) ? version : (releases[0]?.id ?? '')
  const pick = builds.find((b) => b.version === build)

  useEffect(() => {
    if (tab === 'vanilla' || !hasTauri() || !mcBase) {
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
  }, [tab, mcBase])

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
        r.onerror = () => reject(new Error('не удалось прочитать файл'))
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

  const list = tab === 'vanilla' ? releases : profiles

  return (
    <div className="play-layout">
      <div className="screen play-screen">
        <Card className="play-card">
          <h1 className="retro-title">Играть</h1>

          <div className="tabs">
            {TABS.map((t) => (
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
            {accMode ? (
              <span className="nick ely-nick" title="Аккаунт авторизован">{accName}</span>
            ) : (
              <Input
                className="nick"
                value={nick}
                onChange={(e) => setNick(e.target.value)}
                placeholder="Ник"
                maxLength={16}
              />
            )}
            <DropdownMenu>
              <DropdownMenuTrigger>
                {selected && list.some((v) => v.id === version) ? version : 'Версия…'}
              </DropdownMenuTrigger>
              <DropdownMenuContent className="version-list">
                {list.map((v) => (
                  <DropdownMenuItem key={v.id}>
                    <div className="version-item" onClick={() => setVersion(v.id)}>
                      {v.id}
                    </div>
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          {tab !== 'vanilla' && (
            <div className="play-row">
              <span className="muted loader-hint">
                {buildsBusy
                  ? `Сборки ${tab}…`
                  : builds.length > 0
                    ? `${tab} для ${mcBase}`
                    : `Сборок ${tab} под ${mcBase} нет`}
              </span>
              {builds.length > 0 && (
                <>
                  <DropdownMenu>
                    <DropdownMenuTrigger>
                      {pick ? (pick.recommended ? `${build} · рекоменд.` : build) : 'Сборка…'}
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
                  <Button bg="#2f7d4f" onClick={() => void installBuild()} disabled={installing || buildsBusy}>
                    {installing ? 'Ставим…' : 'Установить'}
                  </Button>
                </>
              )}
            </div>
          )}

          {(busy || installing) && <ProgressBar progress={progress} size="lg" />}
          {stage && <p className="muted">{stage}</p>}
          {error && <p className="error">{error}</p>}
          {!hasTauri() && <p className="muted">Ядро не запущено — режим браузера.</p>}

          <div className="play-row">
            {playing ? (
              <Button onClick={() => void stop()} bg="#a03030">
                Остановить
              </Button>
            ) : (
              <Button onClick={() => void play()} disabled={busy || installing || !version}>
                {busy ? 'Готовим…' : 'Играть'}
              </Button>
            )}
          </div>
        </Card>

        {logs.length > 0 && (
          <pre className="log" ref={logRef}>
            {logs.join('\n')}
          </pre>
        )}
      </div>

      <div className="side-column">
        <Card className="skin-card">
          <h2 className="retro-title">Скин</h2>
          <SkinView nick={accMode ? accName : nick} refreshKey={skinKey} />
          <label className="skin-buttons">
            <input
              type="file"
              accept="image/png"
              className="skin-file"
              onChange={(e) => void onSkinFile(e.target.files?.[0])}
            />
            <span>Скин PNG</span>
          </label>
          <Button
            bg="#a03030"
            onClick={() => void removeSkin()}
            disabled={!accMode && !nick.trim()}
          >
            Убрать
          </Button>
          {skinError && <p className="error">{skinError}</p>}
          <p className="muted skin-note">
            Скин применяется через CustomSkinLoader в сборках с загрузчиком; с аккаунтом Ely.by —
            через authlib-injector.
          </p>
        </Card>
        <Servers version={version} />
        <News />
      </div>
    </div>
  )
}
