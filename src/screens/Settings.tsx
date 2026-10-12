import { useEffect, useState } from 'react'
import { Button, Card, Input, DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from 'pixel-retroui'
import {
  listVersions,
  instanceList,
  getInstanceConfig,
  saveInstanceConfig,
  getLauncherSettings,
  setMirrors,
  setCfKey,
  setGameWindow,
  setGameFullscreen,
  setBackups,
  packExport,
  packImport,
  type InstanceInfo,
} from '../ipc/commands'
import { onPackProgress, onPackImported, type PackProgress } from '../ipc/events'
import { useAccount } from '../state/account'
import { useGame, NICK_RE } from '../state/game'
import { useTheme, type ThemeName } from '../state/theme'
import { useLang, useT, type Lang, type TKey } from '../i18n'
import { useUi } from '../state/ui'
import { hasTauri } from '../ipc/tauri'

// Подписи провайдеров в карточке аккаунта
const MODE_KEY: Record<string, TKey> = {
  msa: 'acc.provider.msa',
  ely: 'acc.provider.ely',
  offline: 'acc.mode.offline',
}

// Аккаунтов может быть несколько (ely, msa, оффлайн-профили):
// список с переключением кликом, активный подсвечен
function AccountCard() {
  const t = useT()
  const { info, accounts, login, error, init, startLogin, cancelLogin, switchTo, logout, addOffline } =
    useAccount()
  const [copied, setCopied] = useState(false)
  const [nick, setNick] = useState('')

  useEffect(() => {
    void init()
  }, [init])

  const copyCode = async () => {
    if (!login) return
    try {
      await navigator.clipboard.writeText(login.userCode)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      // в старых webview буфер может быть недоступен — код и так виден
    }
  }

  // Оффлайн-профиль по нику: uuid выводится как в ваниле, повторное
  // добавление того же ника обновляет существующую запись
  const addOfflineNick = async () => {
    const n = nick.trim()
    if (!NICK_RE.test(n)) return
    if (await addOffline(n)) setNick('')
  }

  const providerLabel = login?.provider === 'msa' ? 'Microsoft' : 'Ely.by'

  return (
    <Card className="play-card">
      <h2 className="retro-title">{t('acc.title')}</h2>
      {info?.name ? (
        <div className="acc-row">
          <div className="acc-info">
            <span>{info.name}</span>
            <span className="muted acc-uuid">
              {t(MODE_KEY[info.mode] ?? info.mode)} · {info.uuid}
            </span>
          </div>
          <Button bg="#a03030" onClick={() => void logout(info.uuid)}>
            {t('acc.logout')}
          </Button>
        </div>
      ) : (
        !login && (
          <p className="muted">{t('acc.offlineNote')}</p>
        )
      )}
      {login ? (
        <div className="ely-login">
          <p>{t('acc.enterCode', { provider: providerLabel })}</p>
          <div className="user-code">{login.userCode}</div>
          <div className="play-row">
            <Button onClick={() => void copyCode()}>{copied ? t('btn.copied') : t('btn.copyCode')}</Button>
            <a className="ely-link" href={login.verificationUri} target="_blank" rel="noreferrer">
              {login.verificationUri}
            </a>
          </div>
          <p className="muted">{t('acc.waiting')}</p>
          <Button bg="#a03030" onClick={cancelLogin}>
            {t('action.cancel')}
          </Button>
        </div>
      ) : (
        <div className="play-row">
          <Button bg="#2f7d4f" onClick={() => void startLogin('ely')}>
            {t('acc.loginEly')}
          </Button>
          <Button onClick={() => void startLogin('msa')}>{t('acc.loginMsa')}</Button>
        </div>
      )}
      {accounts.length > 0 && (
        <div className="acc-list">
          {accounts.map((a) => (
            <div key={a.uuid} className={a.active ? 'acc-item on' : 'acc-item'}>
              <button
                className="acc-pick"
                onClick={() => !a.active && void switchTo(a.uuid)}
                title={a.active ? t('acc.active') : t('acc.makeActive')}
              >
                <span>{a.name}</span>
                <span className="muted">{t(MODE_KEY[a.mode] ?? a.mode)}</span>
              </button>
              {!a.active && (
                <button
                  className="acc-drop"
                  onClick={() => void logout(a.uuid)}
                  title={t('acc.remove')}
                >
                  ×
                </button>
              )}
            </div>
          ))}
        </div>
      )}
      <div className="play-row">
        <Input
          value={nick}
          maxLength={16}
          placeholder={t('acc.nickPlaceholder')}
          onChange={(e) => setNick(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && void addOfflineNick()}
        />
        <Button
          bg="#2f7d4f"
          onClick={() => void addOfflineNick()}
          disabled={!NICK_RE.test(nick.trim())}
        >
          {t('action.add')}
        </Button>
      </div>
      {error && <p className="error">{error}</p>}
    </Card>
  )
}

// Зеркало BMCLAPI ускоряет библиотеки и ассеты из России;
// ключ CurseForge включает официальный API вместо зеркала
function MirrorsCard() {
  const t = useT()
  const [on, setOn] = useState(true)
  const [saved, setSaved] = useState(false)
  const [cfKey, setCfKeyState] = useState('')
  const [cfSaved, setCfSaved] = useState(false)

  useEffect(() => {
    if (!hasTauri()) return
    void getLauncherSettings()
      .then((s) => {
        setOn(s.use_mirrors)
        setCfKeyState(s.cf_api_key ?? '')
      })
      .catch(() => {})
  }, [])

  const toggle = async () => {
    const next = !on
    setOn(next)
    try {
      await setMirrors(next)
      setSaved(true)
      setTimeout(() => setSaved(false), 1500)
    } catch {
      setOn(!next)
    }
  }

  const saveKey = async () => {
    try {
      await setCfKey(cfKey)
      setCfSaved(true)
      setTimeout(() => setCfSaved(false), 1500)
    } catch {
      // ключ опционален: сбой сохранения не критичен
    }
  }

  return (
    <Card className="play-card">
      <h2 className="retro-title">{t('mirrors.title')}</h2>
      <label className="check-row">
        <input type="checkbox" checked={on} onChange={() => void toggle()} disabled={!hasTauri()} />
        <span>{t('mirrors.bmclapi')}</span>
      </label>
      <p className="muted mirrors-note">
        {t('mirrors.fallback')} {saved && t('status.saved')}
      </p>
      <div className="play-row">
        <Input
          className="mods-query"
          value={cfKey}
          onChange={(e) => setCfKeyState(e.target.value)}
          placeholder={t('mirrors.cfKey')}
        />
        <Button onClick={() => void saveKey()} disabled={!hasTauri()}>
          {cfSaved ? t('btn.saved') : t('btn.apply')}
        </Button>
      </div>
      <p className="muted mirrors-note">{t('mirrors.cfNote')}</p>
    </Card>
  )
}

// Карточка обновления лаунчера: сводка релиза, тихая установка, ручная проверка
function UpdateCard() {
  const t = useT()
  const update = useUi((s) => s.update)
  const progress = useUi((s) => s.progress)
  const updateError = useUi((s) => s.updateError)
  const checkNow = useUi((s) => s.updateCheckNow)
  const apply = useUi((s) => s.updateApply)
  const [checking, setChecking] = useState(false)

  const check = async () => {
    setChecking(true)
    await checkNow()
    setChecking(false)
  }

  const pct =
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.received / progress.total) * 100))
      : 0

  // Markdown из release notes в плоский текст на три строчки
  const summary = (update?.notes ?? '')
    .replace(/[#*`>\-\[\]]/g, '')
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
    .slice(0, 3)
    .join(' · ')

  return (
    <Card className="play-card">
      <h2 className="retro-title">{t('update.title')}</h2>
      {update ? (
        <>
          <p>
            {t('update.available', { v: update.version })}
            <span className="muted"> · {t('units.mb', { n: (update.size / 1048576).toFixed(1) })}</span>
          </p>
          {summary && <p className="muted mirrors-note">{summary}</p>}
          {progress && (
            <div className="update-banner">
              <span className="muted">
                {progress.done ? t('update.installing') : t('update.progress', { pct })}
              </span>
              <div className="update-bar">
                <div
                  className="update-bar-fill"
                  style={{ width: `${progress.done ? 100 : pct}%` }}
                />
              </div>
            </div>
          )}
          {!progress && (
            <Button bg="#2f7d4f" onClick={() => void apply()}>
              {t('update.applyAndRestart')}
            </Button>
          )}
        </>
      ) : (
        <p className="muted mirrors-note">{t('update.fresh')}</p>
      )}
      <div className="play-row">
        <Button onClick={() => void check()} disabled={checking || !!progress}>
          {checking ? t('busy.checking') : t('action.checkNow')}
        </Button>
      </div>
      {updateError && <p className="error">{updateError}</p>}
    </Card>
  )
}

// Окно запускаемой игры: размер и полноэкранный режим, общие для всех сборок.
// Значения пишет бэкенд (clamp_window), фронту остаётся валидация для UX
function WindowCard() {
  const t = useT()
  const [width, setWidth] = useState('1280')
  const [height, setHeight] = useState('720')
  const [fullscreen, setFullscreen] = useState(false)
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!hasTauri()) return
    void getLauncherSettings()
      .then((s) => {
        setWidth(String(s.game_width))
        setHeight(String(s.game_height))
        setFullscreen(s.game_fullscreen)
      })
      .catch(() => {})
  }, [])

  const save = async () => {
    const w = Number.parseInt(width, 10)
    const h = Number.parseInt(height, 10)
    if (!Number.isFinite(w) || w < 320 || w > 7680) {
      setError(t('error.width'))
      return
    }
    if (!Number.isFinite(h) || h < 240 || h > 4320) {
      setError(t('error.height'))
      return
    }
    setError('')
    try {
      await setGameWindow(w, h)
      await setGameFullscreen(fullscreen)
      setSaved(true)
      setTimeout(() => setSaved(false), 1500)
    } catch (e) {
      setError(String(e))
    }
  }

  return (
    <Card className="play-card">
      <h2 className="retro-title">{t('window.title')}</h2>
      <div className="play-row">
        <label className="field">
          <span>{t('window.width')}</span>
          <Input
            className="ram"
            value={width}
            onChange={(e) => {
              setWidth(e.target.value.replace(/\D/g, ''))
              setSaved(false)
            }}
            inputMode="numeric"
            maxLength={4}
          />
        </label>
        <label className="field">
          <span>{t('window.height')}</span>
          <Input
            className="ram"
            value={height}
            onChange={(e) => {
              setHeight(e.target.value.replace(/\D/g, ''))
              setSaved(false)
            }}
            inputMode="numeric"
            maxLength={4}
          />
        </label>
      </div>
      <label className="check-row">
        <input
          type="checkbox"
          checked={fullscreen}
          onChange={(e) => {
            setFullscreen(e.target.checked)
            setSaved(false)
          }}
        />
        <span>{t('window.fullscreen')}</span>
      </label>
      <p className="muted mirrors-note">{t('window.note')}</p>
      {error && <p className="error">{error}</p>}
      {saved && <p className="muted">{t('status.saved')}</p>}
      <div className="play-row">
        <Button onClick={() => void save()}>{t('action.save')}</Button>
      </div>
    </Card>
  )
}

// Бэкапы миров: автобэкап перед запуском и сколько копий хранить на мир
function BackupCard() {
  const t = useT()
  const [on, setOn] = useState(true)
  const [keep, setKeep] = useState('5')
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!hasTauri()) return
    void getLauncherSettings()
      .then((s) => {
        setOn(s.backup_on_launch)
        setKeep(String(s.backup_keep))
      })
      .catch(() => {})
  }, [])

  const save = async () => {
    const k = Number.parseInt(keep, 10)
    if (!Number.isFinite(k) || k < 1 || k > 30) {
      setError(t('error.keep'))
      return
    }
    setError('')
    try {
      await setBackups(on, k)
      setSaved(true)
      setTimeout(() => setSaved(false), 1500)
    } catch (e) {
      setError(String(e))
    }
  }

  return (
    <Card className="play-card">
      <h2 className="retro-title">{t('backups.title')}</h2>
      <label className="check-row">
        <input
          type="checkbox"
          checked={on}
          onChange={(e) => {
            setOn(e.target.checked)
            setSaved(false)
          }}
        />
        <span>{t('backups.on')}</span>
      </label>
      <div className="play-row">
        <label className="field">
          <span>{t('backups.keep')}</span>
          <Input
            className="ram"
            value={keep}
            onChange={(e) => {
              setKeep(e.target.value.replace(/\D/g, ''))
              setSaved(false)
            }}
            inputMode="numeric"
            maxLength={2}
          />
        </label>
        <Button onClick={() => void save()}>{t('action.save')}</Button>
      </div>
      <p className="muted mirrors-note">{t('backups.note')}</p>
      {error && <p className="error">{error}</p>}
      {saved && <p className="muted">{t('status.saved')}</p>}
    </Card>
  )
}

// Экспорт текущей сборки в файл и импорт файла-сборки
function PacksCard() {
  const t = useT()
  const [busy, setBusy] = useState('')
  const [stage, setStage] = useState('')
  const [error, setError] = useState('')
  const refresh = useGame((s) => s.refresh)
  const versions = useGame((s) => s.versions)
  const current = localStorage.getItem('daylo.cfg.version') ?? versions.find((v) => v.type === 'release')?.id ?? ''

  useEffect(() => {
    if (!hasTauri()) return
    void onPackProgress((p: PackProgress) => setStage(p.error ? '' : `${p.stage} ${p.done}/${p.total}`))
    void onPackImported((p: { id: string; error: string | null }) => {
      setBusy('')
      setStage('')
      void refresh()
      if (p.error) setError(p.error)
    })
  }, [refresh])

  const doExport = async () => {
    if (!current || busy) return
    setBusy('export')
    setError('')
    try {
      const path = await packExport(current)
      if (path) setStage(t('packsCard.savedPath', { path }))
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy('')
    }
  }

  const doImport = async () => {
    if (busy) return
    setBusy('import')
    setStage('')
    setError('')
    try {
      const id = await packImport()
      if (id) setStage(t('packsCard.installingFile', { id }))
      else setBusy('')
    } catch (e) {
      setError(String(e))
      setBusy('')
      setStage('')
    }
  }

  return (
    <Card className="play-card">
      <h2 className="retro-title">{t('packs.title')}</h2>
      <p className="muted mirrors-note">{t('packsCard.note')}</p>
      <div className="play-row">
        <Button onClick={() => void doExport()} disabled={busy !== '' || !current}>
          {busy === 'export' ? t('busy.exporting') : t('packsCard.export')}
        </Button>
        <Button bg="#2f7d4f" onClick={() => void doImport()} disabled={busy !== ''}>
          {busy === 'import' ? t('busy.importing') : t('packsCard.import')}
        </Button>
      </div>
      {stage && <p className="muted">{stage}</p>}
      {error && <p className="error">{error}</p>}
    </Card>
  )
}

// Сборка в дропдауне настроек: id папки + подпись для человека
interface Choice {
  id: string
  label: string
}

// У модпаков и своих сборок показываем имя и загрузчик, у ванилы — версию
const choiceLabel = (i: InstanceInfo): string => {
  if (i.kind !== 'vanilla' && i.loader) return `${i.name} · ${i.loader}`
  if (i.kind !== 'vanilla') return i.name
  return i.id
}

// Тема оформления: чистый фронт, применяется сразу, работает без ядра
const THEME_KEY: Record<ThemeName, TKey> = {
  dark: 'theme.dark',
  light: 'theme.light',
  plains: 'theme.plains',
  nether: 'theme.nether',
}

function ThemeCard() {
  const t = useT()
  const theme = useTheme((s) => s.theme)
  const setTheme = useTheme((s) => s.set)
  return (
    <Card className="play-card">
      <h2 className="retro-title">{t('theme.title')}</h2>
      <div className="tabs theme-tabs">
        {(Object.keys(THEME_KEY) as ThemeName[]).map((tn) => (
          <button
            key={tn}
            className={theme === tn ? 'tab on' : 'tab'}
            onClick={() => setTheme(tn)}
            title={t(THEME_KEY[tn])}
          >
            {/* Превью-миниатюра: небо с частицей, срез травы и земля в цветах темы */}
            <span className={`tprev tprev-${tn}`} aria-hidden="true" />
            {t(THEME_KEY[tn])}
          </button>
        ))}
      </div>
      <p className="muted">{t('theme.note')}</p>
    </Card>
  )
}

// Язык интерфейса: RU/EN, работает без ядра, запоминается в localStorage
function LangCard() {
  const t = useT()
  const lang = useLang((s) => s.lang)
  const setLang = useLang((s) => s.set)
  const langs: { id: Lang; label: string }[] = [
    { id: 'ru', label: 'Русский' },
    { id: 'en', label: 'English' },
  ]
  return (
    <Card className="play-card">
      <h2 className="retro-title">{t('lang.title')}</h2>
      <div className="tabs">
        {langs.map((l) => (
          <button
            key={l.id}
            className={lang === l.id ? 'tab on' : 'tab'}
            onClick={() => setLang(l.id)}
          >
            {l.label}
          </button>
        ))}
      </div>
    </Card>
  )
}

export function Settings() {
  const t = useT()
  const [choices, setChoices] = useState<Choice[]>([])
  const [version, setVersion] = useState(localStorage.getItem('daylo.cfg.version') ?? '')
  const [ram, setRam] = useState('2048')
  const [flags, setFlags] = useState('')
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState('')

  const online = hasTauri()

  useEffect(() => {
    if (!online) return
    void (async () => {
      try {
        // Установленные сборки (модпаки, свои) первыми — их настраивают чаще;
        // релизы Mojang остаются для преднастройки памяти до первой установки
        const [inst, list] = await Promise.all([
          instanceList().catch(() => [] as InstanceInfo[]),
          listVersions(),
        ])
        const built: Choice[] = [
          ...inst.map((i) => ({ id: i.id, label: choiceLabel(i) })),
          ...list
            .filter((v) => v.type === 'release' && !inst.some((i) => i.id === v.id))
            .map((v) => ({ id: v.id, label: v.id })),
        ]
        setChoices(built)
        const stored = localStorage.getItem('daylo.cfg.version')
        const id =
          stored && built.some((c) => c.id === stored) ? stored : (built[0]?.id ?? '')
        setVersion(id)
        loadConfig(id)
      } catch (e) {
        setError(String(e))
      }
    })()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const loadConfig = (id: string) => {
    if (!online || !id) return
    void getInstanceConfig(id)
      .then((cfg) => {
        setRam(String(cfg.ram_mb))
        setFlags(cfg.jvm_args.join('\n'))
        setSaved(false)
      })
      .catch((e) => setError(String(e)))
  }

  const pickVersion = (id: string) => {
    localStorage.setItem('daylo.cfg.version', id)
    setVersion(id)
    loadConfig(id)
  }

  const save = async () => {
    if (!online || !version) return
    setError('')
    const ramMb = Number.parseInt(ram, 10)
    if (!Number.isFinite(ramMb) || ramMb < 512 || ramMb > 32768) {
      setError(t('error.ram'))
      return
    }
    try {
      await saveInstanceConfig(
        version,
        ramMb,
        flags
          .split('\n')
          .map((s) => s.trim())
          .filter(Boolean),
      )
      setSaved(true)
    } catch (e) {
      setError(String(e))
    }
  }

  const currentLabel = choices.find((c) => c.id === version)?.label ?? version

  return (
    <div className="screen">
      <h1 className="retro-title">{t('settings.title')}</h1>

      {online && <AccountCard />}

      <Card className="play-card">
        <div className="play-row">
          <DropdownMenu>
            <DropdownMenuTrigger>{currentLabel || t('placeholder.build')}</DropdownMenuTrigger>
            <DropdownMenuContent className="version-list">
              {choices.map((c) => (
                <DropdownMenuItem key={c.id}>
                  <div className="version-item" onClick={() => pickVersion(c.id)}>
                    {c.label}
                  </div>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        <label className="field">
          <span>{t('settings.memory')}</span>
          <Input
            className="ram"
            value={ram}
            onChange={(e) => {
              setRam(e.target.value.replace(/\D/g, ''))
              setSaved(false)
            }}
            inputMode="numeric"
            maxLength={6}
          />
        </label>

        <label className="field">
          <span>{t('settings.flags')}</span>
          <textarea
            className="flags"
            value={flags}
            onChange={(e) => {
              setFlags(e.target.value)
              setSaved(false)
            }}
            placeholder={'-XX:+UseG1GC\n-XX:MaxGCPauseMillis=50'}
            rows={5}
          />
        </label>

        {!online && <p className="muted">{t('core.offline')}</p>}
        {error && <p className="error">{error}</p>}
        {saved && <p className="muted">{t('status.saved')}</p>}

        <div className="play-row">
          <Button onClick={() => void save()} disabled={!online || !version}>
            {t('action.save')}
          </Button>
        </div>
      </Card>

      {online && <WindowCard />}

      <ThemeCard />

      <LangCard />

      {online && <BackupCard />}

      {online && <MirrorsCard />}

      {online && <UpdateCard />}

      {online && <PacksCard />}
    </div>
  )
}
