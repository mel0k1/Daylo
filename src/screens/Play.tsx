import { useEffect, useRef } from 'react'
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
import { hasTauri } from '../ipc/tauri'

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

  useEffect(() => {
    void init()
  }, [init])

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight })
  }, [logs])

  const releases = versions.filter((v) => v.type === 'release')
  const selected = versions.find((v) => v.id === version)

  return (
    <div className="screen">
      <Card className="play-card">
        <h1 className="retro-title">Играть</h1>

        <div className="play-row">
          <Input
            className="nick"
            value={nick}
            onChange={(e) => setNick(e.target.value)}
            placeholder="Ник"
            maxLength={16}
          />
          <DropdownMenu>
            <DropdownMenuTrigger>{selected ? selected.id : 'Версия…'}</DropdownMenuTrigger>
            <DropdownMenuContent className="version-list">
              {releases.map((v) => (
                <DropdownMenuItem key={v.id}>
                  <div className="version-item" onClick={() => setVersion(v.id)}>
                    {v.id}
                  </div>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        {busy && <ProgressBar progress={progress} size="lg" />}
        {stage && <p className="muted">{stage}</p>}
        {error && <p className="error">{error}</p>}
        {!hasTauri() && <p className="muted">Ядро не запущено — режим браузера.</p>}

        <div className="play-row">
          {playing ? (
            <Button onClick={() => void stop()} bg="#a03030">
              Остановить
            </Button>
          ) : (
            <Button onClick={() => void play()} disabled={busy || !version}>
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
  )
}
