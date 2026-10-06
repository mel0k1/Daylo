import { useEffect, useRef, useState } from 'react'

const S = 6

// Рисует фронтальный вид скина: голова, тело, руки, ноги + слой шляпы
function draw(canvas: HTMLCanvasElement, img: HTMLImageElement) {
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.imageSmoothingEnabled = false
  ctx.clearRect(0, 0, canvas.width, canvas.height)
  const legacy = img.height === 32
  const face = (sx: number, sy: number, sw: number, sh: number, dx: number, dy: number) =>
    ctx.drawImage(img, sx, sy, sw, sh, dx * S, dy * S, sw * S, sh * S)

  face(8, 8, 8, 8, 4, 0) // голова
  face(40, 8, 8, 8, 4, 0) // шляпа поверх
  face(20, 20, 8, 12, 4, 8) // тело
  face(44, 20, 4, 12, 0, 8) // правая рука
  face(4, 20, 4, 12, 4, 20) // правая нога

  if (!legacy) {
    face(36, 52, 4, 12, 12, 8) // левая рука
    face(20, 52, 4, 12, 8, 20) // левая нога
  } else {
    // Старые скины: левые конечности — отражение правых
    for (const [sx, dx] of [
      [44, 12],
      [4, 8],
    ] as const) {
      ctx.save()
      ctx.translate((dx + 4) * S, 0)
      ctx.scale(-1, 1)
      ctx.drawImage(img, sx, 20, 4, 12, 0, 8 * S, 4 * S, 12 * S)
      ctx.restore()
    }
  }
}

interface Props {
  nick: string
  refreshKey: number
}

export function SkinView({ nick, refreshKey }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [ok, setOk] = useState(false)

  useEffect(() => {
    let dead = false
    setOk(false)
    const render = (src: string) => {
      const img = new Image()
      img.onload = () => {
        if (dead || !canvasRef.current) return
        draw(canvasRef.current, img)
        setOk(true)
      }
      img.src = src
    }
    void (async () => {
      const { skinLoad, } = await import('../ipc/commands')
      const { hasTauri } = await import('../ipc/tauri')
      const name = nick.trim()
      if (hasTauri() && name) {
        const b64 = await skinLoad(name).catch(() => null)
        if (b64) {
          render(`data:image/png;base64,${b64}`)
          return
        }
      }
      // Публичный сервис как предпросмотр; неизвестный ник покажет стива
      if (name) render(`https://minotar.net/skin/${encodeURIComponent(name)}`)
    })()
    return () => {
      dead = true
    }
  }, [nick, refreshKey])

  return (
    <div className="skin-view">
      <canvas ref={canvasRef} width={16 * S} height={32 * S} className={ok ? '' : 'dim'} />
      {!ok && <span className="muted skin-hint">скин не выбран</span>}
    </div>
  )
}
