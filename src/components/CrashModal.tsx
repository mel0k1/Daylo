import { Button } from 'pixel-retroui'
import type { CrashInfo } from '../ipc/events'
import { t } from '../i18n'

export function CrashModal({ info, onClose }: { info: CrashInfo; onClose: () => void }) {
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="crash-modal" onClick={(e) => e.stopPropagation()}>
        <h2 className="retro-title">{t('crash.title')}</h2>
        {info.instance && <p className="muted crash-instance">{t('crash.build', { inst: info.instance })}</p>}
        <h3 className="crash-cause">{info.title}</h3>
        <p className="crash-reason">{info.reason}</p>
        {info.advice.length > 0 && (
          <ul className="crash-advice">
            {info.advice.map((a, i) => (
              <li key={i}>{a}</li>
            ))}
          </ul>
        )}
        {info.excerpt.length > 0 && (
          <details className="crash-details">
            <summary>{t('crash.logSummary')}</summary>
            <pre className="crash-log">{info.excerpt.join('\n')}</pre>
          </details>
        )}
        <Button onClick={onClose}>{t('crash.ok')}</Button>
      </div>
    </div>
  )
}
