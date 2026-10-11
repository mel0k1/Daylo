import { Component, type ErrorInfo, type ReactNode } from 'react'

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
        <h1 className="retro-title">Интерфейс упал</h1>
        <p>Произошла ошибка отрисовки:</p>
        <p className="error">{error.message || String(error)}</p>
        <p className="muted">
          Сборки, настройки и миры не пострадали. Перезапустите интерфейс — если ошибка
          повторяется на каждом старте, пришлите автору текст из этого экрана.
        </p>
        <button onClick={() => window.location.reload()}>Перезапустить интерфейс</button>
      </div>
    )
  }
}
