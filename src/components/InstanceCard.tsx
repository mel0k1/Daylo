import { Button, Card } from 'pixel-retroui'
import type { InstanceInfo } from '../ipc/commands'
import { useT, type TKey } from '../i18n'

const SOURCE_KEY: Record<string, TKey> = {
  modrinth: 'source.modrinth',
  curseforge: 'source.curseforge',
  ftb: 'source.ftb',
  custom: 'source.custom',
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
  const t = useT()
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
          {info.source in SOURCE_KEY
            ? t(SOURCE_KEY[info.source])
            : info.kind === 'modpack'
              ? t('badge.modpack')
              : t('type.release')}
        </span>
        <span className="pack-badge">{t('units.mb', { n: info.ram_mb })}</span>
        {info.mods_count > 0 && (
          <span className="pack-badge">{t('badge.modsCount', { n: info.mods_count })}</span>
        )}
      </div>
      <div className="mods-row-actions">
        <Button bg="#2f7d4f" onClick={onLaunch} disabled={launchDisabled}>
          {launching ? t('busy.preparing') : t('action.play')}
        </Button>
        <Button onClick={onVerify} disabled={verifyDisabled}>
          {verifying ? t('busy.checking') : t('action.verify')}
        </Button>
        <Button bg="#a03030" onClick={onRemove}>
          {t('action.delete')}
        </Button>
      </div>
    </Card>
  )
}
