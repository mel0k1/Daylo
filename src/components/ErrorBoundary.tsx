import { Component, type ErrorInfo, type ReactNode } from 'react'
import { t } from '../i18n'

interface Props {
  children: ReactNode
}

interface State {
  error: Error | null
}

// Стена между падением рендера и белым экраном: показываем читаемый экран
// с сообщением и кнопкой перезапуска интерфейса. Данные (настройки, сборки,
// миры) живут в файлах на диске и от этого сбоя не страдают
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('Ошибка интерфейса:', error, info.componentStack)
  }

  render() {
    const { error } = this.state
    if (!error) return this.props.children
    return (
      <div className="screen">
        <h1 className="retro-title">{t('eb.title')}</h1>
        <p>{t('eb.renderError')}</p>
        <p className="error">{error.message || String(error)}</p>
        <p className="muted">{t('eb.note')}</p>
        <button onClick={() => window.location.reload()}>{t('eb.restart')}</button>
      </div>
    )
  }
}
