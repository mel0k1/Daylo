import { useCallback, useEffect, useState } from 'react'
import { Button, Card, Input } from 'pixel-retroui'
import {
  pingServer,
  serversAdd,
  serversList,
  serversRemove,
  gameServers,
  type ServerStatus,
  type GameServer,
} from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'

const offlineStub = (s: string): ServerStatus => ({
  online: false,
  ms: 0,
  motd: '',
  version: '',
  players_online: 0,
  players_max: 0,
  favicon: null,
  error: `${s}: нет связи`,
})

export function Servers({ version }: { version: string }) {
  const [servers, setServers] = useState<string[]>([])
  const [addr, setAddr] = useState('')
  const [statuses, setStatuses] = useState<Record<string, ServerStatus>>({})
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [fromGame, setFromGame] = useState<GameServer[]>([])

  const pingAll = useCallback(async (list: string[]) => {
    setBusy(true)
    const pairs = await Promise.all(
      list.map(async (s) => {
        const st = await pingServer(s).catch(() => offlineStub(s))
        return [s, st] as const
      }),
    )
    setStatuses(Object.fromEntries(pairs))
    setBusy(false)
  }, [])

  useEffect(() => {
    if (!hasTauri()) return
    void serversList()
      .then(setServers)
      .catch(() => {})
  }, [])

  useEffect(() => {
    if (servers.length > 0) void pingAll(servers)
  }, [servers, pingAll])

  // servers.dat меняется после каждого выхода из игры — перечитываем
  useEffect(() => {
    if (!hasTauri() || !version) {
      setFromGame([])
      return
    }
    void gameServers(version)
      .then(setFromGame)
      .catch(() => setFromGame([]))
  }, [version])

  const add = async () => {
    const s = addr.trim()
    if (!s) return
    setError('')
    try {
      await serversAdd(s)
      setAddr('')
      setServers(await serversList())
    } catch (e) {
      setError(String(e))
    }
  }

  const remove = async (s: string) => {
    setError('')
    try {
      await serversRemove(s)
      setServers(await serversList())
    } catch (e) {
      setError(String(e))
    }
  }

  const track = async (s: string) => {
    setError('')
    try {
      await serversAdd(s)
      setServers(await serversList())
    } catch (e) {
      setError(String(e))
    }
  }

  if (!hasTauri()) return null

  return (
    <Card className="servers-card">
      <h2 className="retro-title">Серверы</h2>
      <div className="srv-add">
        <Input
          className="srv-input"
          value={addr}
          onChange={(e) => setAddr(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void add()
          }}
          placeholder="mc.example.ru:25565"
          maxLength={255}
        />
        <Button onClick={() => void add()}>+</Button>
      </div>
      {error && <p className="error">{error}</p>}
      <div className="srv-list">
        {servers.length === 0 && (
          <p className="muted srv-note">Добавьте адрес сервера — здесь появится его статус.</p>
        )}
        {servers.map((s) => {
          const st = statuses[s]
          return (
            <div key={s} className="srv-row">
              {st?.favicon ? (
                <img className="srv-icon" src={st.favicon} alt="" />
              ) : (
                <div className="srv-icon empty" />
              )}
              <div className="srv-info">
                <span className="srv-addr">{s}</span>
                {st ? (
                  st.online ? (
                    <span className="muted srv-meta">
                      {st.motd || 'онлайн'} · {st.players_online}/{st.players_max} · {st.ms} мс
                    </span>
                  ) : (
                    <span className="srv-meta srv-down">недоступен</span>
                  )
                ) : (
                  <span className="muted srv-meta">проверяем…</span>
                )}
              </div>
              <button
                className="srv-del"
                onClick={() => void remove(s)}
                title="Убрать из списка"
              >
                ×
              </button>
            </div>
          )
        })}
      </div>
      {fromGame.length > 0 && (
        <div className="srv-game">
          <span className="muted srv-game-title">Из игры ({version})</span>
          <div className="srv-list">
            {fromGame.map((s) => (
              <div key={s.address} className="srv-row">
                {s.icon ? (
                  <img className="srv-icon" src={s.icon} alt="" />
                ) : (
                  <div className="srv-icon empty" />
                )}
                <div className="srv-info">
                  <span className="srv-addr">{s.name || s.address}</span>
                  <span className="muted srv-meta">{s.address}</span>
                </div>
                <button
                  className="srv-del"
                  title="Добавить в наблюдаемые"
                  onClick={() => void track(s.address)}
                >
                  +
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
      {servers.length > 0 && (
        <Button bg="#2f7d4f" disabled={busy} onClick={() => void pingAll(servers)}>
          {busy ? 'Пингуем…' : 'Обновить'}
        </Button>
      )}
    </Card>
  )
}
