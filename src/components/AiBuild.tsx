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
import {
  aiBuildPlan,
  instanceCreate,
  listVersions,
  loaderBuilds,
  modrinthInstall,
  type AiPick,
  type VersionEntry,
  type LoaderBuild,
} from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'

type OwnLoader = 'fabric' | 'quilt' | 'forge' | 'neoforge'

const AI_LOADERS: { id: OwnLoader; label: string }[] = [
  { id: 'fabric', label: 'Fabric' },
  { id: 'quilt', label: 'Quilt' },
  { id: 'forge', label: 'Forge' },
  { id: 'neoforge', label: 'NeoForge' },
]

// ИИ-сборщик: описание словами -> план модов -> готовая сборка.
// План строит бесплатный безключевой LLM-шлюз, моды ищутся в Modrinth.
export function AiBuild() {
  const [open, setOpen] = useState(false)
  const [prompt, setPrompt] = useState('')
  const [name, setName] = useState('')
  const [mcVersion, setMcVersion] = useState('')
  const [versions, setVersions] = useState<VersionEntry[]>([])
  const [loader, setLoader] = useState<OwnLoader>('fabric')
  const [builds, setBuilds] = useState<LoaderBuild[]>([])
  const [build, setBuild] = useState('')
  const [plan, setPlan] = useState<AiPick[] | null>(null)
  const [skipped, setSkipped] = useState<Set<string>>(new Set())
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')

  const online = hasTauri()

  useEffect(() => {
    if (!online) return
    void listVersions()
      .then((list) => {
        setVersions(list)
        setMcVersion((cur) => cur || list.find((v) => v.type === 'release')?.id || '')
      })
      .catch(() => {})
  }, [online])

  useEffect(() => {
    if (!mcVersion) {
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

  const releases = versions.filter((v) => v.type === 'release')

  const togglePick = (projectId: string) => {
    setSkipped((cur) => {
      const next = new Set(cur)
      if (next.has(projectId)) next.delete(projectId)
      else next.add(projectId)
      return next
    })
  }

  const buildPlan = async () => {
    if (busy || !mcVersion) return
    setBusy('plan')
    setError('')
    setPlan(null)
    try {
      setPlan(await aiBuildPlan(prompt.trim(), mcVersion, loader))
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy('')
    }
  }

  // Создаём сборку и ставим выбранные моды по одному; сбой одного мода
  // не роняет остальные — в конце честная сводка
  const install = async () => {
    if (busy || !plan || !mcVersion || !build) return
    setBusy('install')
    setError('')
    const picks = plan.filter((p) => !skipped.has(p.project_id))
    try {
      const id = await instanceCreate(name.trim() || 'ИИ-сборка', mcVersion, loader, build)
      let ok = 0
      const failed: string[] = []
      for (const p of picks) {
        try {
          await modrinthInstall(id, p.version_id)
          ok += 1
        } catch {
          failed.push(p.title)
        }
      }
      if (failed.length > 0) {
        setError(`Не поставились: ${failed.join(', ')}`)
      }
      setPlan(null)
      setPrompt('')
      if (ok > 0 && failed.length === 0) setOpen(false)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy('')
    }
  }

  return (
    <Card className="mods-installed ai-build">
      <div className="mods-head">
        <Button onClick={() => setOpen(!open)}>{open ? 'Свернуть' : 'ИИ-сборка'}</Button>
        <p className="muted mirrors-note">
          Опиши сборку словами — моды подберёт нейросеть и поставит в новую сборку. Бесплатно.
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
            {AI_LOADERS.map((l) => (
              <button
                key={l.id}
                className={loader === l.id ? 'tab on' : 'tab'}
                onClick={() => setLoader(l.id)}
              >
                {l.label}
              </button>
            ))}
          </div>
          <textarea
            className="ai-prompt"
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="Например: лёгкая техно-сборка с автоматизацией и хранением энергии, немного магии"
            rows={3}
            maxLength={500}
          />
          <div className="play-row">
            {builds.length > 0 && (
              <DropdownMenu>
                <DropdownMenuTrigger>
                  {build
                    ? builds.find((b) => b.version === build)?.recommended
                      ? `${build} · рекоменд.`
                      : build
                    : 'Сборка…'}
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
            )}
            <Button
              onClick={() => void buildPlan()}
              disabled={!!busy || !mcVersion || prompt.trim().length < 3}
            >
              {busy === 'plan' ? 'Думаем…' : 'Собрать план'}
            </Button>
          </div>

          {plan && plan.length > 0 && (
            <div className="ai-plan">
              <p className="muted ai-plan-note">
                План: {plan.length} модов. Сними галочку с ненужных и собирай.
              </p>
              {plan.map((p) => (
                <label key={p.project_id} className="ai-pick">
                  <input
                    type="checkbox"
                    checked={!skipped.has(p.project_id)}
                    onChange={() => togglePick(p.project_id)}
                  />
                  <img
                    className="mod-icon"
                    src={p.icon_url}
                    alt=""
                    onError={(e) => (e.currentTarget.style.visibility = 'hidden')}
                  />
                  <div className="ai-pick-info">
                    <span className="mod-title">{p.title}</span>
                    <span className="muted ai-pick-desc">{p.description}</span>
                  </div>
                </label>
              ))}
              <div className="play-row">
                <Button bg="#2f7d4f" onClick={() => void install()} disabled={!!busy}>
                  {busy === 'install'
                    ? 'Ставим…'
                    : `Создать сборку (${plan.length - skipped.size} модов)`}
                </Button>
              </div>
            </div>
          )}
          {plan && plan.length === 0 && (
            <p className="muted">Ничего не нашлось — попробуй переформулировать.</p>
          )}
          {error && <p className="error">{error}</p>}
        </>
      )}
    </Card>
  )
}
