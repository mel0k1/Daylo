import { useEffect, useState } from 'react'
import { Button, Card, Input, DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from 'pixel-retroui'
import {
  listVersions,
  getInstanceConfig,
  saveInstanceConfig,
  getLauncherSettings,
  setMirrors,
  type VersionEntry,
} from '../ipc/commands'
import { useAccount } from '../state/account'
import { hasTauri } from '../ipc/tauri'

// Вход через Ely.by: код устройства, браузер, ожидание подтверждения
function AccountCard() {
  const { info, login, error, init, startLogin, cancelLogin, logout } = useAccount()
  const [copied, setCopied] = useState(false)

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

  return (
    <Card className="play-card">
      <h2 className="retro-title">Аккаунт</h2>
      {info?.mode === 'ely' ? (
        <div className="acc-row">
          <div className="acc-info">
            <span>{info.name}</span>
            <span className="muted acc-uuid">{info.uuid}</span>
          </div>
          <Button bg="#a03030" onClick={() => void logout()}>
            Выйти
          </Button>
        </div>
      ) : login ? (
        <div className="ely-login">
          <p>Введите код на странице входа Ely.by:</p>
          <div className="user-code">{login.userCode}</div>
          <div className="play-row">
            <Button onClick={() => void copyCode()}>{copied ? 'Скопировано' : 'Скопировать код'}</Button>
            <a className="ely-link" href={login.verificationUri} target="_blank" rel="noreferrer">
              {login.verificationUri}
            </a>
          </div>
          <p className="muted">Ждём подтверждение…</p>
          <Button bg="#a03030" onClick={cancelLogin}>
            Отмена
          </Button>
        </div>
      ) : (
        <div className="ely-login">
          <p className="muted">
            Оффлайн-режим без авторизации. Аккаунт Ely.by даст скины и вход на серверы с Ely.by.
          </p>
          <Button bg="#2f7d4f" onClick={() => void startLogin()}>
            Войти через Ely.by
          </Button>
        </div>
      )}
      {error && <p className="error">{error}</p>}
    </Card>
  )
}

// Зеркало BMCLAPI ускоряет библиотеки и ассеты из России
function MirrorsCard() {
  const [on, setOn] = useState(true)
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    if (!hasTauri()) return
    void getLauncherSettings()
      .then((s) => setOn(s.use_mirrors))
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

  return (
    <Card className="play-card">
      <h2 className="retro-title">Загрузки</h2>
      <label className="check-row">
        <input type="checkbox" checked={on} onChange={() => void toggle()} disabled={!hasTauri()} />
        <span>Зеркало BMCLAPI для библиотек, ассетов и загрузчиков</span>
      </label>
      <p className="muted mirrors-note">
        Оригинальные серверы Mojang и maven остаются запасным путём: если зеркало молчит, качаем
        напрямую. {saved && 'Сохранено.'}
      </p>
    </Card>
  )
}

export function Settings() {
  const [versions, setVersions] = useState<VersionEntry[]>([])
  const [version, setVersion] = useState(localStorage.getItem('daylo.cfg.version') ?? '')
  const [ram, setRam] = useState('2048')
  const [flags, setFlags] = useState('')
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState('')

  const online = hasTauri()

  useEffect(() => {
    if (!online) return
    void listVersions()
      .then((list) => {
        setVersions(list)
        const stored = localStorage.getItem('daylo.cfg.version')
        const fallback = list.find((v) => v.type === 'release')?.id ?? ''
        const id = stored && list.some((v) => v.id === stored) ? stored : fallback
        setVersion(id)
        loadConfig(id)
      })
      .catch((e) => setError(String(e)))
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
      setError('память — число от 512 до 32768 МБ')
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

  const releases = versions.filter((v) => v.type === 'release')

  return (
    <div className="screen">
      <h1 className="retro-title">Настройки</h1>

      {online && <AccountCard />}

      <Card className="play-card">
        <div className="play-row">
          <DropdownMenu>
            <DropdownMenuTrigger>{version || 'Сборка…'}</DropdownMenuTrigger>
            <DropdownMenuContent className="version-list">
              {releases.map((v) => (
                <DropdownMenuItem key={v.id}>
                  <div className="version-item" onClick={() => pickVersion(v.id)}>
                    {v.id}
                  </div>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        <label className="field">
          <span>Память (МБ), по умолчанию 2048</span>
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
          <span>Дополнительные JVM-флаги, по одному в строке</span>
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

        {!online && <p className="muted">Ядро не запущено — режим браузера.</p>}
        {error && <p className="error">{error}</p>}
        {saved && <p className="muted">Сохранено.</p>}

        <div className="play-row">
          <Button onClick={() => void save()} disabled={!online || !version}>
            Сохранить
          </Button>
        </div>
      </Card>

      {online && <MirrorsCard />}
    </div>
  )
}
