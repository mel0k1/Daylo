import { Button, Card } from 'pixel-retroui'
import type { InstanceInfo } from '../ipc/commands'

export const SOURCE_LABEL: Record<string, string> = {
  modrinth: 'Modrinth',
  curseforge: 'CurseForge',
  ftb: 'FTB',
  custom: 'Своя',
}

// Карточка установленной сборки: одна на экран «Играть» (библиотека)
// и на экран «Сборки»
export function InstanceCard({
  info,
  launching,
  verifying,
  launchDisabled,
  verifyDisabled,
  onLaunch,
  onVerify,
  onRemove,
}: {
  info: InstanceInfo
  launching: boolean
  verifying: boolean
  launchDisabled: boolean
  verifyDisabled: boolean
  onLaunch: () => void
  onVerify: () => void
  onRemove: () => void
}) {
  return (
    <Card className="mod-card pack-card">
      <div className="mod-head">
        {info.icon_url ? (
          <img
            className="mod-icon"
            src={info.icon_url}
            alt=""
            onError={(e) => (e.currentTarget.style.display = 'none')}
          />
        ) : (
          <div className="mod-icon empty" />
        )}
        <div>
          <div className="mod-title">{info.name}</div>
          <div className="muted">
            {info.mc_version}
            {info.loader ? ` · ${info.loader}` : ''}
            {info.pack_version ? ` · ${info.pack_version}` : ''}
          </div>
        </div>
      </div>
      <div className="pack-badges">
        <span className="pack-badge">
          {SOURCE_LABEL[info.source] ?? (info.kind === 'modpack' ? 'сборка' : 'ванила')}
        </span>
        <span className="pack-badge">{info.ram_mb} МБ</span>
        {info.mods_count > 0 && <span className="pack-badge">модов: {info.mods_count}</span>}
      </div>
      <div className="mods-row-actions">
        <Button bg="#2f7d4f" onClick={onLaunch} disabled={launchDisabled}>
          {launching ? 'Готовим…' : 'Играть'}
        </Button>
        <Button onClick={onVerify} disabled={verifyDisabled}>
          {verifying ? 'Проверяем…' : 'Проверить'}
        </Button>
        <Button bg="#a03030" onClick={onRemove}>
          Удалить
        </Button>
      </div>
    </Card>
  )
}
