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
