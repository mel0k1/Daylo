import { useEffect, useMemo, useRef, useState } from 'react'
import { elySkin, skinLoad } from '../ipc/commands'
import { hasTauri } from '../ipc/tauri'
import { useAccount } from '../state/account'

const U = 5 // пикселей на «пиксель» модели

interface Tex {
  url: string
  w: number
}

type Rect = [number, number, number, number]

interface Faces {
  top: Rect
  bottom: Rect
  right: Rect
  front: Rect
  left: Rect
  back: Rect
}

interface PartDef {
  id: string
  dims: [number, number, number]
  faces: Faces
  // Конечности качаются вокруг точки плеча/бедра
  pivot?: [number, number]
  swing?: 'a' | 'b'
  overlay?: boolean
}

const HEAD: Faces = {
  top: [8, 0, 8, 8],
  bottom: [16, 0, 8, 8],
  right: [0, 8, 8, 8],
  front: [8, 8, 8, 8],
  left: [16, 8, 8, 8],
  back: [24, 8, 8, 8],
}

const BODY: Faces = {
  top: [20, 16, 8, 4],
  bottom: [28, 16, 8, 4],
  right: [16, 20, 4, 12],
  front: [20, 20, 8, 12],
  left: [28, 20, 4, 12],
  back: [32, 20, 8, 12],
}

const R_ARM: Faces = {
  top: [44, 16, 4, 4],
  bottom: [48, 16, 4, 4],
  right: [40, 20, 4, 12],
  front: [44, 20, 4, 12],
  left: [48, 20, 4, 12],
  back: [52, 20, 4, 12],
}

const L_ARM: Faces = {
  top: [36, 48, 4, 4],
  bottom: [40, 48, 4, 4],
  right: [32, 52, 4, 12],
  front: [36, 52, 4, 12],
  left: [40, 52, 4, 12],
  back: [44, 52, 4, 12],
}

const R_LEG: Faces = {
  top: [4, 16, 4, 4],
  bottom: [8, 16, 4, 4],
  right: [0, 20, 4, 12],
  front: [4, 20, 4, 12],
  left: [8, 20, 4, 12],
  back: [12, 20, 4, 12],
}

const L_LEG: Faces = {
  top: [20, 48, 4, 4],
  bottom: [24, 48, 4, 4],
  right: [16, 52, 4, 12],
  front: [20, 52, 4, 12],
  left: [24, 52, 4, 12],
  back: [28, 52, 4, 12],
}

// Слои: та же геометрия со сдвигом по X, натягиваются с надувом
const shift = (f: Faces, dx: number, dy: number): Faces => ({
  top: [f.top[0] + dx, f.top[1] + dy, f.top[2], f.top[3]],
  bottom: [f.bottom[0] + dx, f.bottom[1] + dy, f.bottom[2], f.bottom[3]],
  right: [f.right[0] + dx, f.right[1] + dy, f.right[2], f.right[3]],
  front: [f.front[0] + dx, f.front[1] + dy, f.front[2], f.front[3]],
  left: [f.left[0] + dx, f.left[1] + dy, f.left[2], f.left[3]],
  back: [f.back[0] + dx, f.back[1] + dy, f.back[2], f.back[3]],
})

// Координаты центров в «пикселях» модели: x 0..16 слева направо, y 0..32 сверху вниз
const PARTS: PartDef[] = [
  { id: 'head', dims: [8, 8, 8], faces: HEAD },
  { id: 'body', dims: [8, 12, 4], faces: BODY },
  { id: 'arm-r', dims: [4, 12, 4], faces: R_ARM, pivot: [2, 8], swing: 'a' },
  { id: 'arm-l', dims: [4, 12, 4], faces: L_ARM, pivot: [14, 8], swing: 'b' },
  { id: 'leg-r', dims: [4, 12, 4], faces: R_LEG, pivot: [6, 20], swing: 'b' },
  { id: 'leg-l', dims: [4, 12, 4], faces: L_LEG, pivot: [10, 20], swing: 'a' },
  { id: 'hat', dims: [8, 8, 8], faces: shift(HEAD, 32, 0), overlay: true },
  { id: 'jacket', dims: [8, 12, 4], faces: shift(BODY, 0, 16), overlay: true },
  { id: 'sleeve-r', dims: [4, 12, 4], faces: shift(R_ARM, 0, 16), pivot: [2, 8], swing: 'a', overlay: true },
  { id: 'sleeve-l', dims: [4, 12, 4], faces: shift(L_ARM, 16, 0), pivot: [14, 8], swing: 'b', overlay: true },
  { id: 'pants-r', dims: [4, 12, 4], faces: shift(R_LEG, 0, 16), pivot: [6, 20], swing: 'b', overlay: true },
  { id: 'pants-l', dims: [4, 12, 4], faces: shift(L_LEG, -16, 0), pivot: [10, 20], swing: 'a', overlay: true },
]

function loadImg(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.crossOrigin = 'anonymous'
    img.onload = () => resolve(img)
    img.onerror = () => reject(new Error('скин не загрузился'))
    img.src = src
  })
}

// Приводим любую текстуру к квадрату 64x64: старые скины 64x32 получают
// левые конечности зеркальными правыми, HD-скины отдаются с исходным разрешением
async function toTex(src: string): Promise<Tex> {
  const img = await loadImg(src)
  const w = img.width
  if (w !== img.height && w !== img.height * 2) throw new Error('это не скин Minecraft')
  try {
    const c = document.createElement('canvas')
    c.width = w
    c.height = w
    const ctx = c.getContext('2d')
    if (!ctx) throw new Error('нет canvas')
    ctx.imageSmoothingEnabled = false
    ctx.drawImage(img, 0, 0)
    if (w === img.height * 2) {
      for (const [sx, sy, dx] of [
        [40, 16, 32], // рука
        [0, 16, 16], // нога
      ] as const) {
        ctx.save()
        ctx.translate(dx + 16, 48)
        ctx.scale(-1, 1)
        ctx.drawImage(img, sx, sy, 16, 16, 0, 0, 16, 16)
        ctx.restore()
      }
    }
    return { url: c.toDataURL(), w }
  } catch {
    // Картинка с чужого домена может «пачкать» canvas — рисуем как есть
    return { url: src, w }
  }
}

function faceEls(faces: Faces, dims: [number, number, number], tex: Tex) {
  const f = tex.w / 64
  const [w, h, d] = dims
  const mk = (rect: Rect, cw: number, ch: number, transform: string, cls: string, key: string) => (
    <div
      key={key}
      className={`face ${cls}`}
      style={{
        width: cw * U,
        height: ch * U,
        left: (-cw * U) / 2,
        top: (-ch * U) / 2,
        backgroundImage: `url(${tex.url})`,
        backgroundSize: `${64 * U * f}px ${64 * U * f}px`,
        backgroundPosition: `${-rect[0] * U * f}px ${-rect[1] * U * f}px`,
        transform,
      }}
    />
  )
  return [
    mk(faces.front, w, h, `translateZ(${(d * U) / 2}px)`, 'f-front', 'front'),
    mk(faces.back, w, h, `rotateY(180deg) translateZ(${(d * U) / 2}px)`, 'f-back', 'back'),
    mk(faces.right, d, h, `rotateY(-90deg) translateZ(${(w * U) / 2}px)`, 'f-side', 'right'),
    mk(faces.left, d, h, `rotateY(90deg) translateZ(${(w * U) / 2}px)`, 'f-side', 'left'),
    mk(faces.top, w, d, `rotateX(90deg) translateZ(${(h * U) / 2}px)`, 'f-top', 'top'),
    mk(faces.bottom, w, d, `rotateX(-90deg) translateZ(${(h * U) / 2}px)`, 'f-bottom', 'bottom'),
  ]
}

const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v))

function Scene({ tex }: { tex: Tex | null }) {
  const tiltRef = useRef<HTMLDivElement>(null)
  const spinRef = useRef<HTMLDivElement>(null)
  const rot = useRef({ x: -6, y: 28 })
  const drag = useRef<{ id: number; x: number; y: number } | null>(null)
  const last = useRef(0)

  // Плавное авто-вращение, пока игрок не крутил модель руками
  useEffect(() => {
    let raf = 0
    const tick = () => {
      const r = rot.current
      if (!drag.current && performance.now() - last.current > 2200) r.y += 0.35
      if (spinRef.current) spinRef.current.style.transform = `rotateY(${r.y}deg)`
      if (tiltRef.current) tiltRef.current.style.transform = `rotateX(${r.x}deg)`
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [])

  const model = useMemo(() => {
    if (!tex) return null
    const at = (x: number, y: number) =>
      `translate3d(${(x - 8) * U}px, ${(y - 16) * U}px, 0px)`
    return PARTS.map((p) => {
      const scale = p.overlay ? ' scale3d(1.125, 1.125, 1.125)' : ''
      if (p.pivot) {
        const [hx, hy] = p.pivot
        return (
          <div key={p.id} className="part" style={{ transform: at(hx, hy) }}>
            <div className={`limb swing-${p.swing === 'a' ? 'a' : 'b'}`}>
              <div className="part" style={{ transform: `translate3d(0px, ${(p.dims[1] / 2) * U}px, 0px)${scale}` }}>
                {faceEls(p.faces, p.dims, tex)}
              </div>
            </div>
          </div>
        )
      }
      // голова — y 4, тело — y 14
      const cy = p.id === 'head' || p.id === 'hat' ? 4 : 14
      return (
        <div key={p.id} className="part" style={{ transform: `${at(8, cy)}${scale}` }}>
          {faceEls(p.faces, p.dims, tex)}
        </div>
      )
    })
  }, [tex])

  const onDown = (e: React.PointerEvent<HTMLDivElement>) => {
    drag.current = { id: e.pointerId, x: e.clientX, y: e.clientY }
    e.currentTarget.setPointerCapture(e.pointerId)
    last.current = performance.now()
  }

  const onMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const d = drag.current
    if (!d || d.id !== e.pointerId) return
    rot.current.y += (e.clientX - d.x) * 0.55
    rot.current.x = clamp(rot.current.x - (e.clientY - d.y) * 0.35, -42, 42)
    d.x = e.clientX
    d.y = e.clientY
    last.current = performance.now()
  }

  const onUp = () => {
    drag.current = null
    last.current = performance.now()
  }

  return (
    <div className={`skin3d${tex ? '' : ' dim'}`} onPointerDown={onDown} onPointerMove={onMove} onPointerUp={onUp} onPointerCancel={onUp}>
      <div className="skin3d-tilt" ref={tiltRef}>
        <div className="skin3d-spin" ref={spinRef}>
          {model}
        </div>
      </div>
    </div>
  )
}

interface Props {
  nick: string
  refreshKey: number
}

export function SkinView({ nick, refreshKey }: Props) {
  const [tex, setTex] = useState<Tex | null>(null)

  useEffect(() => {
    let dead = false
    setTex(null)
    void (async () => {
      const name = nick.trim()
      // Свой файл приоритетнее: им игрок переопределяет и ely, и серверный скин
      if (hasTauri() && name) {
        const b64 = await skinLoad(name).catch(() => null)
        if (b64) {
          await toTex(`data:image/png;base64,${b64}`)
            .then((t) => !dead && setTex(t))
            .catch(() => {})
          return
        }
        const acc = useAccount.getState().info
        if (acc?.mode === 'ely' && acc.name === name) {
          const s = await elySkin(name).catch(() => null)
          if (s) {
            await toTex(`data:image/png;base64,${s}`)
              .then((t) => !dead && setTex(t))
              .catch(() => {})
            return
          }
        }
      }
      // Публичный сервис как предпросмотр; неизвестный ник покажет стива
      if (name) {
        await toTex(`https://minotar.net/skin/${encodeURIComponent(name)}`)
          .then((t) => !dead && setTex(t))
          .catch(() => {})
      }
    })()
    return () => {
      dead = true
    }
  }, [nick, refreshKey])

  return (
    <div className="skin-view">
      <Scene tex={tex} />
      {!tex && <span className="muted skin-hint">скин не выбран</span>}
    </div>
  )
}
