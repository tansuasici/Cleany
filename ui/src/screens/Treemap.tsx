import {
  type KeyboardEvent,
  type MouseEvent,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { formatBytes, formatCount, formatShare } from '../lib/format'
import { type NodeEntry, type ScanId, type TreemapCell, type TreemapSlice, getTreemap } from '../lib/ipc'
import { type Box, fit, useBox, useScheme } from '../canvas'
import { useLater } from '../useLater'
import { useT } from '../useT'

/** Levels laid out below the folder on show. */
const DEPTH = 2
/** Children smaller than this (CSS px²) merge into one "others" block. */
const MIN_AREA = 24
const LOADING_DELAY_MS = 250
/** A top block gets its name and size written in when it is at least this big. */
const LABEL_W = 80
const LABEL_H = 48

/*
 * DESIGN_MAP → direction, approved 2026-11-15: blocks are ink at low opacity,
 * one step per level, so nesting reads as darker paper rather than colour.
 * Only the largest block takes the accent.
 */
const TONE = 0.09
const OTHERS_TONE = 0.04
const ACCENT_TONE = 0.2
const HOVER_TONE = 0.08
/** Extra clear pixels on the right and bottom of each top block (3px gap vs 1px inside). */
const TOP_GUTTER = 2

const NO_CELLS: TreemapCell[] = []

/** The last slice asked for; `node` lets a stale size keep drawing while a new one loads. */
type Got = { key: string; node: number; result: TreemapSlice | 'error' }

const keyOf = (scan: ScanId, node: number, box: Box) => `${scan}:${node}:${box.w}x${box.h}`

const opens = (c: TreemapCell | undefined): c is TreemapCell & { id: number } =>
  c !== undefined && c.id !== null && c.kind === 'dir' && c.hasChildren

/** Whole-pixel edges, leaving a 1px gap on the right and bottom between siblings. */
function rect(c: TreemapCell) {
  const x = Math.round(c.x)
  const y = Math.round(c.y)
  return { x, y, w: Math.round(c.x + c.w) - x - 1, h: Math.round(c.y + c.h) - y - 1 }
}

const inside = (c: TreemapCell, px: number, py: number) =>
  px >= c.x && px < c.x + c.w && py >= c.y && py < c.y + c.h

type Dir = 'left' | 'right' | 'up' | 'down'

/** The top block nearest `from` in `dir`, by centre; sideways distance counts double. */
function nearest(top: TreemapCell[], from: number, dir: Dir): number {
  const a = top[from]
  if (!a) return from
  const ax = a.x + a.w / 2
  const ay = a.y + a.h / 2
  let best = from
  let bestScore = Infinity
  top.forEach((b, j) => {
    if (j === from) return
    const dx = b.x + b.w / 2 - ax
    const dy = b.y + b.h / 2 - ay
    const ahead = dir === 'right' ? dx : dir === 'left' ? -dx : dir === 'down' ? dy : -dy
    if (ahead <= 0) return
    const score = ahead + 2 * (dir === 'left' || dir === 'right' ? Math.abs(dy) : Math.abs(dx))
    if (score < bestScore) {
      bestScore = score
      best = j
    }
  })
  return best
}

/**
 * The folder as a squarified treemap on a canvas (#24). The layout comes from
 * `getTreemap`; this only draws it, hit-tests it and moves a selection over
 * its top blocks.
 */
export function Treemap({
  scan,
  node,
  label,
  cameFrom,
  onOpen,
  onUp,
  onExit,
}: {
  scan: ScanId
  node: NodeEntry
  label: string
  cameFrom: number | null
  onOpen: (node: NodeEntry) => void
  onUp: () => void
  onExit: () => void
}) {
  const { t, locale } = useT()
  const panel = useRef<HTMLDivElement>(null)
  const stage = useRef<HTMLDivElement>(null)
  const canvas = useRef<HTMLCanvasElement>(null)
  const box = useBox(stage)
  const scheme = useScheme()
  const [got, setGot] = useState<Got | null>(null)
  const [hover, setHover] = useState<{ cell: number; x: number; y: number } | null>(null)
  const [selected, setSelected] = useState<{ node: number; i: number } | null>(null)

  const key = box && box.w > 0 && box.h > 0 ? keyOf(scan, node.id, box) : null

  useEffect(() => {
    if (!key || !box || got?.key === key) return
    let live = true
    const asked = node.id
    getTreemap(scan, asked, box.w, box.h, DEPTH, MIN_AREA).then(
      (slice) => live && setGot({ key, node: asked, result: slice }),
      () => live && setGot({ key, node: asked, result: 'error' }),
    )
    return () => {
      live = false
    }
    // `got` is read only to skip a slice already in hand (or prefetched by `open`).
  }, [key, box, scan, node.id, got?.key])

  useEffect(() => {
    panel.current?.focus()
  }, [])

  const failed = got?.key === key && got.result === 'error'
  // A slice for this folder at an older size keeps drawing while the new one loads.
  const slice = got && got.node === node.id && got.result !== 'error' ? got.result : null
  const cells = slice?.cells ?? NO_CELLS
  const top = useMemo(() => cells.filter((c) => c.depth === 1), [cells])
  const slow = useLater(slice || failed ? null : `${node.id}`, LOADING_DELAY_MS)

  // A new folder starts on its largest block, or on the one just left by going up.
  const start = Math.max(0, cameFrom === null ? 0 : top.findIndex((c) => c.id === cameFrom))
  const active = selected?.node === node.id ? Math.min(selected.i, Math.max(0, top.length - 1)) : start
  const select = (i: number) => setSelected({ node: node.id, i })
  if (hover && hover.cell >= cells.length) setHover(null)

  useLayoutEffect(() => {
    const el = canvas.current
    if (!el || !box) return
    const dpr = window.devicePixelRatio || 1
    el.width = Math.round(box.w * dpr)
    el.height = Math.round(box.h * dpr)
    const ctx = el.getContext('2d')
    if (!ctx) return
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.clearRect(0, 0, box.w, box.h)
    if (!slice) return

    const css = getComputedStyle(document.documentElement)
    const color = (name: string) => css.getPropertyValue(name).trim()
    const ink = color('--c-ink')
    const ink2 = color('--c-ink-2')
    const accent = color('--c-accent')
    const card = color('--c-card')
    const sans = getComputedStyle(document.body).fontFamily
    const mono = color('--font-mono') || 'monospace'
    const largest = top[0]

    slice.cells.forEach((c, i) => {
      const r = rect(c)
      if (r.w < 1 || r.h < 1) return
      const isLargest = c === largest && c.id !== null
      ctx.fillStyle = isLargest ? accent : ink
      ctx.globalAlpha = isLargest ? ACCENT_TONE : c.id === null ? OTHERS_TONE : TONE
      ctx.fillRect(r.x, r.y, r.w, r.h)
      if (hover?.cell === i) {
        ctx.fillStyle = ink
        ctx.globalAlpha = HOVER_TONE
        ctx.fillRect(r.x, r.y, r.w, r.h)
      }
    })
    ctx.globalAlpha = 1

    // Children fill their parent edge to edge, so widen the gutter between top
    // blocks to keep the first level readable as the frame of the map.
    for (const c of top) {
      const r = rect(c)
      ctx.clearRect(r.x + r.w - 1, r.y, TOP_GUTTER, r.h + TOP_GUTTER)
      ctx.clearRect(r.x, r.y + r.h - 1, r.w + TOP_GUTTER, TOP_GUTTER)
    }

    // Names go on last, on a card-coloured tag, so the blocks below don't bleed through.
    ctx.textBaseline = 'top'
    for (const c of top) {
      const r = rect(c)
      if (r.w < LABEL_W || r.h < LABEL_H) continue
      const max = r.w - 20
      ctx.font = `500 13px ${sans}`
      const name = fit(ctx, c.id === null ? t('browse.others', { n: formatCount(c.others, locale), count: c.others }) : c.name, max)
      const nameW = ctx.measureText(name).width
      ctx.font = `12px ${mono}`
      const size = fit(ctx, formatBytes(c.allocated, locale), max)
      const sizeW = ctx.measureText(size).width
      if (!name) continue
      ctx.fillStyle = card
      ctx.globalAlpha = 0.92
      ctx.beginPath()
      ctx.roundRect(r.x + 4, r.y + 4, Math.ceil(Math.max(nameW, sizeW)) + 12, 40, 4)
      ctx.fill()
      ctx.globalAlpha = 1
      ctx.fillStyle = ink
      ctx.font = `500 13px ${sans}`
      ctx.fillText(name, r.x + 10, r.y + 8)
      ctx.fillStyle = ink2
      ctx.font = `12px ${mono}`
      ctx.fillText(size, r.x + 10, r.y + 26)
    }

    const sel = top[active]
    if (sel) {
      const r = rect(sel)
      ctx.strokeStyle = accent
      ctx.lineWidth = 2
      ctx.strokeRect(r.x + 1, r.y + 1, Math.max(0, r.w - 2), Math.max(0, r.h - 2))
    }
  }, [slice, top, box, active, hover?.cell, scheme, t, locale])

  const at = (e: MouseEvent<HTMLElement>) => {
    const r = e.currentTarget.getBoundingClientRect()
    return { x: e.clientX - r.left, y: e.clientY - r.top }
  }

  const open = useCallback(
    (c: TreemapCell | undefined) => {
      if (!opens(c) || !box) return
      const id = c.id
      // The child's slice is fetched here, so the folder opens drawn and its
      // `NodeEntry` (which `browse` needs) comes with it.
      getTreemap(scan, id, box.w, box.h, DEPTH, MIN_AREA).then(
        (next) => {
          setGot({ key: keyOf(scan, id, box), node: id, result: next })
          setHover(null)
          onOpen(next.node)
        },
        () => setGot({ key: keyOf(scan, node.id, box), node: node.id, result: 'error' }),
      )
    },
    [box, scan, node.id, onOpen],
  )

  const onPointerMove = (e: MouseEvent<HTMLDivElement>) => {
    const p = at(e)
    for (let i = cells.length - 1; i >= 0; i--) {
      if (inside(cells[i], p.x, p.y)) {
        setHover({ cell: i, x: p.x, y: p.y })
        return
      }
    }
    setHover(null)
  }

  const onClick = (e: MouseEvent<HTMLDivElement>) => {
    const p = at(e)
    const i = top.findIndex((c) => inside(c, p.x, p.y))
    if (i < 0) return
    select(i)
    open(top[i])
  }

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return
    const dirs: Record<string, Dir> = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' }
    const dir = dirs[e.key]
    if (dir) select(nearest(top, active, dir))
    else if (e.key === 'Home') select(0)
    else if (e.key === 'End') select(Math.max(0, top.length - 1))
    else if (e.key === 'Enter') open(top[active])
    else if (e.key === 'Backspace') onUp()
    else if (e.key === 'Escape') onExit()
    else return
    e.preventDefault()
  }

  const nameOf = (c: TreemapCell) =>
    c.id === null ? t('browse.others', { n: formatCount(c.others, locale), count: c.others }) : c.name
  const hovered = hover ? cells[hover.cell] : undefined
  const hoveredTop = hover ? top.find((c) => inside(c, hover.x, hover.y)) : undefined
  const chosen = top[active]
  const empty = slice !== null && top.length === 0

  return (
    <div
      ref={panel}
      role="listbox"
      tabIndex={0}
      aria-label={label}
      aria-activedescendant={chosen ? `map-cell-${active}` : undefined}
      onKeyDown={onKeyDown}
      className="mt-4 flex min-h-0 flex-1 flex-col rounded-panel border border-hairline bg-card focus-visible:outline-offset-0"
    >
      <div
        ref={stage}
        onPointerMove={onPointerMove}
        onPointerLeave={() => setHover(null)}
        onClick={onClick}
        className={`relative m-1 min-h-0 flex-1 overflow-hidden ${opens(hoveredTop) ? 'cursor-pointer' : ''}`}
      >
        <canvas
          ref={canvas}
          aria-hidden
          className="absolute top-0 left-0"
          style={box ? { width: box.w, height: box.h } : undefined}
        />

        {failed ? (
          <p role="alert" className="absolute inset-x-0 top-0 flex items-center gap-3 px-3 py-2 text-ink-2">
            {t('summary.childrenError')}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation()
                setGot(null)
              }}
              className="inline-flex min-h-6 items-center rounded-tag px-1 text-ink underline-offset-2 hover:underline"
            >
              {t('volumes.retry')}
            </button>
          </p>
        ) : (
          // One status line for loading and empty, so the empty result is announced too.
          <p role="status" className="absolute inset-x-0 top-0 px-3 py-2 text-ink-2 empty:hidden">
            {slice === null ? slow && t('browse.loading') : empty && t('browse.mapEmpty')}
          </p>
        )}

        {hovered && hover && box && (
          <div
            aria-hidden
            className="pointer-events-none absolute top-0 left-0 max-w-72 rounded-card border border-hairline bg-card px-3 py-2"
            style={{
              transform: `translate(${hover.x > box.w / 2 ? `calc(${hover.x - 12}px - 100%)` : `${hover.x + 12}px`}, ${
                hover.y > box.h / 2 ? `calc(${hover.y - 12}px - 100%)` : `${hover.y + 12}px`
              })`,
            }}
          >
            <bdi className="block truncate text-body font-medium">{nameOf(hovered)}</bdi>
            <span className="flex gap-3 text-secondary text-ink-2">
              <span className="font-mono tabular-nums">{formatBytes(hovered.allocated, locale)}</span>
              <span className="font-mono tabular-nums">{formatShare(hovered.allocated, node.allocated, locale)}</span>
              {hovered.kind === 'dir' && (
                <span className="tabular-nums">{t('browse.fileCount', { n: formatCount(hovered.files, locale), count: hovered.files })}</span>
              )}
            </span>
          </div>
        )}
      </div>

      <div className="sr-only">
        {top.map((c, i) => (
          <div key={c.id ?? `others-${c.parent}`} id={`map-cell-${i}`} role="option" aria-selected={i === active}>
            {`${nameOf(c)}, ${formatBytes(c.allocated, locale)}, ${formatShare(c.allocated, node.allocated, locale)}`}
          </div>
        ))}
      </div>

      {/* The selected block in full; the option already says it to a screen reader. */}
      <p aria-hidden className="flex h-8 shrink-0 items-center gap-3 border-t border-hairline px-4 text-meta text-ink-2">
        {chosen && (
          <>
            <bdi className="min-w-0 truncate" title={nameOf(chosen)}>
              {nameOf(chosen)}
            </bdi>
            <span className="shrink-0 font-mono tabular-nums">{formatBytes(chosen.allocated, locale)}</span>
          </>
        )}
      </p>
    </div>
  )
}
