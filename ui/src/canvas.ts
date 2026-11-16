import { type RefObject, useEffect, useState } from 'react'

/** Shared by the canvas views on Browse (map and rings). */

/** A resize settles for this long before a canvas view is laid out again. */
const RESIZE_MS = 120

export type Box = { w: number; h: number }

/** `text` cut at the end to fit `max` px, with an ellipsis. */
export function fit(ctx: CanvasRenderingContext2D, text: string, max: number): string {
  if (ctx.measureText(text).width <= max) return text
  let lo = 0
  let hi = text.length
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2)
    if (ctx.measureText(`${text.slice(0, mid)}…`).width <= max) lo = mid
    else hi = mid - 1
  }
  return lo > 0 ? `${text.slice(0, lo)}…` : ''
}

/** Bumps when the system light/dark setting changes, so the canvas repaints in the new tokens. */
export function useScheme(): number {
  const [n, setN] = useState(0)
  useEffect(() => {
    const query = window.matchMedia('(prefers-color-scheme: dark)')
    const bump = () => setN((v) => v + 1)
    query.addEventListener('change', bump)
    return () => query.removeEventListener('change', bump)
  }, [])
  return n
}

/** The size of `el` in whole CSS pixels; changes wait for `RESIZE_MS` of quiet. */
export function useBox(el: RefObject<HTMLDivElement | null>): Box | null {
  const [box, setBox] = useState<Box | null>(null)
  useEffect(() => {
    const node = el.current
    if (!node) return
    let timer = 0
    const measure = () => {
      const w = Math.floor(node.clientWidth)
      const h = Math.floor(node.clientHeight)
      setBox((b) => (b && b.w === w && b.h === h ? b : { w, h }))
    }
    measure()
    const observer = new ResizeObserver(() => {
      window.clearTimeout(timer)
      timer = window.setTimeout(measure, RESIZE_MS)
    })
    observer.observe(node)
    return () => {
      observer.disconnect()
      window.clearTimeout(timer)
    }
  }, [el])
  return box
}

/** Ink opacities by size rank, read from the `--viz-*` tokens (DESIGN_MAP → Viz ramp). */
export type Ramp = { ranks: number[]; rest: number }

/** The ramp for the current theme; `css` is the root's computed style. */
export function readRamp(css: CSSStyleDeclaration): Ramp {
  const num = (name: string, fallback: number) => {
    const v = parseFloat(css.getPropertyValue(name))
    return Number.isFinite(v) ? v : fallback
  }
  return {
    ranks: [num('--viz-1', 0.3), num('--viz-2', 0.22), num('--viz-3', 0.16), num('--viz-4', 0.1), num('--viz-5', 0.06)],
    rest: num('--viz-rest', 0.04),
  }
}

/** Opacity for the block ranked `i` (0 = largest); past the ramp, the last step. */
export const rankTone = (ramp: Ramp, i: number) => ramp.ranks[Math.min(i, ramp.ranks.length - 1)]

/** Gap between hatch lines on the "others" block, in CSS px. */
const HATCH = 6

/**
 * Diagonal hatch over the current path, so "others" reads as a remainder and
 * not as one more item. The caller builds the path; the clip is restored after.
 */
export function hatch(ctx: CanvasRenderingContext2D, ink: string, alpha: number, x: number, y: number, w: number, h: number) {
  ctx.save()
  ctx.clip()
  ctx.strokeStyle = ink
  ctx.globalAlpha = alpha
  ctx.lineWidth = 1
  ctx.beginPath()
  for (let d = -h; d < w; d += HATCH) {
    ctx.moveTo(x + d, y + h)
    ctx.lineTo(x + d + h, y)
  }
  ctx.stroke()
  ctx.restore()
}
