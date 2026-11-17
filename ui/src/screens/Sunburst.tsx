import { type KeyboardEvent, type MouseEvent, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { formatBytes, formatCount, formatShare } from '../lib/format'
import { type NodeEntry, type RingArc, type ScanId, type SunburstSlice, getSunburst } from '../lib/ipc'
import { type Box, fit, useBox, useScheme, hatch, rankTone, readRamp } from '../canvas'
import { type ActionTarget, type MenuAt, menuPoint } from './useBrowseActions'
import { useLater } from '../useLater'
import { useT } from '../useT'

/** Rings drawn around the centre. */
const DEPTH = 3
/** Children narrower than this (in turns, ~1.4°) merge into one "others" slice. */
const MIN_ANGLE = 0.004
const LOADING_DELAY_MS = 250
/** Space kept clear around the outer ring, in CSS px. */
const MARGIN = 8
/** The centre takes this share of the radius. */
const HOLE = 0.3
/** A first-ring slice gets its name and size written in when it is at least this wide and deep. */
const LABEL_ARC = 88
const LABEL_RING = 40

/*
 * DESIGN_MAP → Viz ramp, approved 2026-11-16, as on the map: first-ring slices
 * are ink at an opacity set by their size rank (`--viz-*`); each ring further
 * out keeps its first-ring slice's tone, fading by `RING_FADE`, so a branch
 * reads as one piece. Only the largest slice takes the accent; "others" is hatched.
 */
const RING_FADE = 0.7
const ACCENT_TONE = 0.2
const HOVER_TONE = 0.08
const HATCH_TONE = 0.18
/** The folder's size in the centre: the view's key figure. */
const CENTRE_FIGURE = 28

const NO_ARCS: RingArc[] = []
const TAU = Math.PI * 2

/** The last slice asked for; `node` lets the open folder keep drawing while a retry loads. */
type Got = { key: string; node: number; result: SunburstSlice | 'error' }

/** Where a point lands: the centre, an arc (by index), or nothing. */
type Hit = { at: 'centre' } | { at: 'arc'; i: number } | null

const opens = (a: RingArc | undefined): a is RingArc & { id: number } =>
  a !== undefined && a.id !== null && a.kind === 'dir' && a.hasChildren

/** Centre and radii for `box`: the hole, then `DEPTH` rings of equal width. */
function geometry(box: Box) {
  const outer = Math.max(0, Math.min(box.w, box.h) / 2 - MARGIN)
  const hole = outer * HOLE
  return { cx: box.w / 2, cy: box.h / 2, hole, ring: (outer - hole) / DEPTH, outer }
}

/** Turns clockwise from 12 o'clock to canvas radians. */
const rad = (turn: number) => turn * TAU - Math.PI / 2

/**
 * The folder as a sunburst on a canvas (#25): its children on the first ring,
 * theirs outside, each slice's angle its share of the parent. The layout comes
 * from `getSunburst`; this only draws it, hit-tests it and moves a selection
 * around the first ring.
 */
export function Sunburst({
  scan,
  node,
  revision,
  name,
  label,
  canGoUp,
  cameFrom,
  onOpen,
  onUp,
  onExit,
  onSelect,
  onMenu,
  hint,
}: {
  scan: ScanId
  node: NodeEntry
  /** The tree's revision; a newer one reads the rings again, the old ones drawing meanwhile. */
  revision: number
  /** The folder's display name, written in the centre. */
  name: string
  label: string
  canGoUp: boolean
  cameFrom: number | null
  onOpen: (node: NodeEntry) => void
  onUp: () => void
  onExit: () => void
  /** The selected block's node, or `null` for "others" and while loading. */
  onSelect: (target: ActionTarget | null) => void
  /** Opens the item menu for the item just selected. */
  onMenu: (at: MenuAt) => void
  /** The short key hint shown at the right of the footer. */
  hint: string
}) {
  const { t, locale } = useT()
  const panel = useRef<HTMLDivElement>(null)
  const stage = useRef<HTMLDivElement>(null)
  const canvas = useRef<HTMLCanvasElement>(null)
  const box = useBox(stage)
  const scheme = useScheme()
  const [got, setGot] = useState<Got | null>(null)
  const [hover, setHover] = useState<{ hit: NonNullable<Hit>; x: number; y: number } | null>(null)
  const [selected, setSelected] = useState<{ node: number; i: number } | null>(null)

  // Angles don't depend on the box, so a resize only redraws.
  const key = `${scan}:${node.id}:${revision}`

  useEffect(() => {
    if (got?.key === key) return
    let live = true
    const asked = node.id
    getSunburst(scan, asked, DEPTH, MIN_ANGLE).then(
      (slice) => live && setGot({ key, node: asked, result: slice }),
      () => live && setGot({ key, node: asked, result: 'error' }),
    )
    return () => {
      live = false
    }
    // `got` is read only to skip a slice already in hand (or prefetched by `open`).
  }, [key, scan, node.id, got?.key])

  useEffect(() => {
    panel.current?.focus()
  }, [])

  const failed = got?.key === key && got.result === 'error'
  const slice = got && got.node === node.id && got.result !== 'error' ? got.result : null
  const arcs = slice?.arcs ?? NO_ARCS
  const top = useMemo(() => arcs.filter((a) => a.depth === 1), [arcs])
  const byId = useMemo(() => new Map(arcs.flatMap((a) => (a.id === null ? [] : [[a.id, a] as const]))), [arcs])
  const slow = useLater(slice || failed ? null : `${node.id}`, LOADING_DELAY_MS)

  // A new folder starts on its largest slice, or on the one just left by going up.
  const start = Math.max(0, cameFrom === null ? 0 : top.findIndex((a) => a.id === cameFrom))
  const active = selected?.node === node.id ? Math.min(selected.i, Math.max(0, top.length - 1)) : start
  const select = (i: number) => setSelected({ node: node.id, i })
  if (hover?.hit.at === 'arc' && hover.hit.i >= arcs.length) setHover(null)

  /** The first-ring slice an arc lies in. */
  const rootOf = useCallback(
    (a: RingArc): RingArc => {
      let cur = a
      while (cur.depth > 1) {
        const up = byId.get(cur.parent)
        if (!up) break
        cur = up
      }
      return cur
    },
    [byId],
  )

  /** The arc itself if it opens, else the nearest folder around it that does. */
  const target = useCallback(
    (a: RingArc | undefined): RingArc | undefined => {
      let cur = a
      while (cur && !opens(cur)) cur = cur.depth > 1 ? byId.get(cur.parent) : undefined
      return cur
    },
    [byId],
  )

  const hit = useCallback(
    (px: number, py: number): Hit => {
      if (!box) return null
      const g = geometry(box)
      const dx = px - g.cx
      const dy = py - g.cy
      const d = Math.hypot(dx, dy)
      if (d < g.hole) return { at: 'centre' }
      if (d >= g.outer || g.ring <= 0) return null
      const depth = Math.floor((d - g.hole) / g.ring) + 1
      const turn = (Math.atan2(dy, dx) / TAU + 1.25) % 1
      const i = arcs.findIndex((a) => a.depth === depth && turn >= a.start && turn < a.end)
      return i < 0 ? null : { at: 'arc', i }
    },
    [box, arcs],
  )

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
    const color = (v: string) => css.getPropertyValue(v).trim()
    const ink = color('--c-ink')
    const ink2 = color('--c-ink-2')
    const accent = color('--c-accent')
    const card = color('--c-card')
    const sans = getComputedStyle(document.body).fontFamily
    const mono = color('--font-mono') || 'monospace'
    const g = geometry(box)
    if (g.ring <= 0) return
    const ramp = readRamp(css)
    const largest = top[0]
    /** The first-ring slice an arc lies under, by angle. */
    const branch = (a: RingArc) => (a.depth === 1 ? top.indexOf(a) : top.findIndex((b) => a.start >= b.start && a.start < b.end))

    const path = (a: RingArc) => {
      const inner = g.hole + (a.depth - 1) * g.ring
      ctx.beginPath()
      ctx.arc(g.cx, g.cy, inner + g.ring, rad(a.start), rad(a.end))
      ctx.arc(g.cx, g.cy, inner, rad(a.end), rad(a.start), true)
      ctx.closePath()
    }

    arcs.forEach((a, i) => {
      path(a)
      const isLargest = a === largest && a.id !== null
      const root = branch(a)
      const base = a.id === null || top[root]?.id === null ? ramp.rest : rankTone(ramp, Math.max(0, root))
      ctx.fillStyle = isLargest ? accent : ink
      ctx.globalAlpha = isLargest ? ACCENT_TONE : base * RING_FADE ** (a.depth - 1)
      ctx.fill()
      if (a.id === null && a.depth === 1) {
        hatch(ctx, ink, HATCH_TONE, g.cx - g.outer, g.cy - g.outer, g.outer * 2, g.outer * 2)
        path(a) // hatch ends its own path; the hover fill and edge need the slice again
      }
      if (hover?.hit.at === 'arc' && hover.hit.i === i) {
        ctx.fillStyle = ink
        ctx.globalAlpha = HOVER_TONE
        ctx.fill()
      }
      // A card-coloured edge parts each slice from its neighbours.
      ctx.globalAlpha = 1
      ctx.strokeStyle = card
      ctx.lineWidth = 1
      ctx.stroke()
    })

    if (hover?.hit.at === 'centre' && canGoUp) {
      ctx.beginPath()
      ctx.arc(g.cx, g.cy, g.hole - 1, 0, TAU)
      ctx.fillStyle = ink
      ctx.globalAlpha = HOVER_TONE
      ctx.fill()
      ctx.globalAlpha = 1
    }

    // The folder itself in the centre.
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    const room = g.hole * 1.6
    ctx.fillStyle = ink2
    ctx.font = `500 13px ${sans}`
    ctx.fillText(fit(ctx, name, room), g.cx, g.cy - 16)
    // Number in mono, unit in smaller sans, as `Size` sets it in the DOM.
    const total = formatBytes(node.allocated, locale)
    const cut = total.lastIndexOf(' ')
    const num = cut < 0 ? total : total.slice(0, cut)
    const unit = cut < 0 ? '' : total.slice(cut)
    const figure = `500 ${CENTRE_FIGURE}px ${mono}`
    const unitFont = `${Math.round(CENTRE_FIGURE * 0.6)}px ${sans}`
    ctx.font = figure
    const numW = ctx.measureText(num).width
    ctx.font = unitFont
    const unitW = ctx.measureText(unit).width
    if (numW + unitW <= room) {
      ctx.textAlign = 'left'
      const x0 = g.cx - (numW + unitW) / 2
      ctx.fillStyle = ink
      ctx.font = figure
      ctx.fillText(num, x0, g.cy + 8)
      ctx.fillStyle = ink2
      ctx.font = unitFont
      ctx.fillText(unit, x0 + numW, g.cy + 10)
      ctx.textAlign = 'center'
    } else {
      ctx.fillStyle = ink
      ctx.font = `12px ${mono}`
      ctx.fillText(fit(ctx, total, room), g.cx, g.cy + 9)
    }

    // Names of the wide first-ring slices, in ink at the slice's middle.
    if (g.ring >= LABEL_RING) {
      for (const a of top) {
        const mid = g.hole + g.ring / 2
        if ((a.end - a.start) * TAU * mid < LABEL_ARC) continue
        const turn = rad((a.start + a.end) / 2)
        const x = g.cx + Math.cos(turn) * mid
        const y = g.cy + Math.sin(turn) * mid
        const max = LABEL_ARC + 32
        ctx.font = `500 13px ${sans}`
        const text = fit(ctx, a.id === null ? t('browse.others', { n: formatCount(a.others, locale), count: a.others }) : a.name, max)
        if (!text) continue
        ctx.font = `12px ${mono}`
        const size = fit(ctx, formatBytes(a.allocated, locale), max)
        ctx.fillStyle = ink
        ctx.font = `500 13px ${sans}`
        ctx.fillText(text, x, y - 8)
        ctx.font = `12px ${mono}`
        ctx.fillText(size, x, y + 10)
      }
    }

    const sel = top[active]
    if (sel) {
      path(sel)
      ctx.strokeStyle = accent
      ctx.lineWidth = 2
      ctx.stroke()
    }
  }, [slice, arcs, top, box, active, hover, canGoUp, name, node.allocated, scheme, t, locale])

  const at = (e: MouseEvent<HTMLElement>) => {
    const r = e.currentTarget.getBoundingClientRect()
    return { x: e.clientX - r.left, y: e.clientY - r.top }
  }

  const open = useCallback(
    (a: RingArc | undefined) => {
      if (!opens(a)) return
      const id = a.id
      // The child's slice is fetched here, so the folder opens drawn and its
      // `NodeEntry` (which `browse` needs) comes with it.
      getSunburst(scan, id, DEPTH, MIN_ANGLE).then(
        (next) => {
          setGot({ key: `${scan}:${id}:${revision}`, node: id, result: next })
          setHover(null)
          onOpen(next.node)
        },
        () => setGot({ key: `${scan}:${node.id}:${revision}`, node: node.id, result: 'error' }),
      )
    },
    [scan, node.id, revision, onOpen],
  )

  const onPointerMove = (e: MouseEvent<HTMLDivElement>) => {
    const p = at(e)
    const h = hit(p.x, p.y)
    setHover(h ? { hit: h, x: p.x, y: p.y } : null)
  }

  const onClick = (e: MouseEvent<HTMLDivElement>) => {
    const p = at(e)
    const h = hit(p.x, p.y)
    if (!h) return
    if (h.at === 'centre') {
      if (canGoUp) onUp()
      return
    }
    const a = arcs[h.i]
    const i = top.indexOf(rootOf(a))
    if (i >= 0) select(i)
    open(target(a))
  }

  // The menu acts on a first-ring item, the one the row of actions shows;
  // outer rings and "other items" have none.
  const onContextMenu = (e: MouseEvent<HTMLDivElement>) => {
    const p = at(e)
    const h = hit(p.x, p.y)
    if (h?.at !== 'arc') return
    const a = arcs[h.i]
    const i = top.indexOf(a)
    if (i < 0 || a.id === null) return
    select(i)
    onMenu(menuPoint(e))
  }

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return
    const n = top.length
    if (n > 0 && (e.key === 'ArrowRight' || e.key === 'ArrowDown')) select((active + 1) % n)
    else if (n > 0 && (e.key === 'ArrowLeft' || e.key === 'ArrowUp')) select((active - 1 + n) % n)
    else if (e.key === 'Home') select(0)
    else if (e.key === 'End') select(Math.max(0, n - 1))
    else if (e.key === 'Enter') open(top[active])
    else if (e.key === 'Backspace') onUp()
    else if (e.key === 'Escape') onExit()
    else return
    e.preventDefault()
  }

  const nameOf = (a: RingArc) =>
    a.id === null ? t('browse.others', { n: formatCount(a.others, locale), count: a.others }) : a.name
  const hovered = hover?.hit.at === 'arc' ? arcs[hover.hit.i] : undefined
  const pointer = hover?.hit.at === 'centre' ? canGoUp : opens(target(hovered))
  const chosen = top[active]
  const chosenId = chosen?.id ?? null
  const chosenName = chosen?.name
  useEffect(() => {
    onSelect(chosenId === null || chosenName === undefined ? null : { id: chosenId, name: chosenName })
  }, [chosenId, chosenName, onSelect])
  const empty = slice !== null && top.length === 0

  return (
    <div
      ref={panel}
      role="listbox"
      tabIndex={0}
      aria-label={label}
      aria-activedescendant={chosen ? `ring-arc-${active}` : undefined}
      onKeyDown={onKeyDown}
      className="mt-4 flex min-h-0 flex-1 flex-col rounded-panel border border-card-edge bg-card shadow-card focus-visible:border-accent focus-visible:outline-none"
    >
      <div
        ref={stage}
        onPointerMove={onPointerMove}
        onPointerLeave={() => setHover(null)}
        onClick={onClick}
        onContextMenu={onContextMenu}
        className={`relative m-2 min-h-0 flex-1 overflow-hidden rounded-control ${pointer ? 'cursor-pointer' : ''}`}
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
            {slice === null ? slow && t('browse.loading') : empty && t('browse.ringsEmpty')}
          </p>
        )}

        {hovered && hover && box && (
          <div
            aria-hidden
            className="pointer-events-none absolute top-0 left-0 max-w-72 rounded-card border border-hairline bg-card px-3 py-2 shadow-float"
            style={{
              transform: `translate(${hover.x > box.w / 2 ? `calc(${hover.x - 12}px - 100%)` : `${hover.x + 12}px`}, ${
                hover.y > box.h / 2 ? `calc(${hover.y - 12}px - 100%)` : `${hover.y + 12}px`
              })`,
            }}
          >
            <bdi className="block truncate text-body font-medium">{nameOf(hovered)}</bdi>
            <span className="flex gap-3 text-secondary text-ink-2">
              <span className="tabular-nums">{formatBytes(hovered.allocated, locale)}</span>
              <span className="tabular-nums">{formatShare(hovered.allocated, node.allocated, locale)}</span>
              {hovered.kind === 'dir' && (
                <span className="tabular-nums">{t('browse.fileCount', { n: formatCount(hovered.files, locale), count: hovered.files })}</span>
              )}
            </span>
          </div>
        )}
      </div>

      <div className="sr-only">
        {top.map((a, i) => (
          <div key={a.id ?? `others-${a.parent}`} id={`ring-arc-${i}`} role="option" aria-selected={i === active}>
            {`${nameOf(a)}, ${formatBytes(a.allocated, locale)}, ${formatShare(a.allocated, node.allocated, locale)}`}
          </div>
        ))}
      </div>

      {/* The selected slice in full; the option already says it to a screen reader. */}
      <div className="flex h-8 shrink-0 items-center gap-3 px-4 text-meta text-ink-2">
        <span aria-hidden className="flex min-w-0 items-center gap-3">
          {chosen && (
            <>
              <bdi className="min-w-0 truncate" title={nameOf(chosen)}>
                {nameOf(chosen)}
              </bdi>
              <span className="shrink-0 tabular-nums">{formatBytes(chosen.allocated, locale)}</span>
            </>
          )}
        </span>
        <span className="ml-auto shrink-0 text-ink-3 max-md:hidden">{hint}</span>
      </div>
    </div>
  )
}
