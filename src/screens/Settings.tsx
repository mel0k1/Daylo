import { useEffect, useState } from 'react'
import { Button, Card, Input, DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from 'pixel-retroui'
import { listVersions, getInstanceConfig, saveInstanceConfig, type VersionEntry } from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'

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
    </div>
  )
}
