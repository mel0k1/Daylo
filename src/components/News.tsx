import { useEffect, useState } from 'react'
import { Card } from 'pixel-retroui'

interface Release {
  tag_name: string
  name: string
  body: string | null
  published_at: string
}

const URL = 'https://api.github.com/repos/mel0k1/Daylo/releases/latest'

function fmtDate(iso: string): string {
  const d = new Date(iso)
  return isNaN(d.getTime()) ? '' : d.toLocaleDateString('ru-RU', { day: 'numeric', month: 'long' })
}

// Лобби-новости в духе Millida: что нового в самом лаунчере — из GitHub Releases
export function News() {
  const [rel, setRel] = useState<Release | null>(null)

  useEffect(() => {
    let dead = false
    void fetch(URL, { headers: { Accept: 'application/vnd.github+json' } })
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
      .then((j: Release) => {
        if (!dead && j.tag_name) setRel(j)
      })
      .catch(() => {})
    return () => {
      dead = true
    }
  }, [])

  if (!rel) return null

  // Обрезаем markdown-разметку и держим короткую сводку
  const lines = (rel.body ?? '')
    .split('\n')
    .map((l) => l.replace(/[#*`>-]/g, '').trim())
    .filter((l) => l.length > 0)
    .slice(0, 6)

  return (
    <Card className="news-card">
      <div className="news-head">
        <span className="news-tag">{rel.tag_name}</span>
        <span className="news-date">{rel.published_at ? fmtDate(rel.published_at) : ''}</span>
      </div>
      {rel.name && <div className="news-name">{rel.name}</div>}
      {lines.length > 0 && <div className="news-body">{lines.join('\n')}</div>}
      <a
        className="link-btn"
        href="https://github.com/mel0k1/Daylo/releases"
        target="_blank"
        rel="noreferrer"
      >
        Все релизы
      </a>
    </Card>
  )
}
