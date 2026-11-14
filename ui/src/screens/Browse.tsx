import { type KeyboardEvent, useCallback, useEffect, useRef, useState } from 'react'
import { ChevronIcon, LeaderRow } from '../components/ui'
import { formatBytes, formatCount, formatDate, formatShare, truncateMiddle } from '../lib/format'
import {
  type NodeEntry,
  type ScanId,
  type SortKey,
  type SortOrder,
  getChildren,
} from '../lib/ipc'
import { type BrowseScreen, useStore } from '../store'
import { useT } from '../useT'
import { useTrail } from '../useTrail'

/** DESIGN_MAP → Spacing: a tree table row is 32 tall. */
const ROW = 32
/** Rows fetched per `getChildren` call. */
const PAGE = 200
/** Rows rendered beyond each edge of the viewport. */
const OVERSCAN = 10

type Sort = { key: SortKey; order: SortOrder }

/** Sizes, counts and dates start largest/newest first; names start A→Z. */
const firstOrder = (key: SortKey): SortOrder => (key === 'name' ? 'asc' : 'desc')

/** Names longer than this are cut in the middle; the full name is in the footer and the tooltip. */
const NAME_CHARS = 64
/** The loading line waits this long, so a quick folder change doesn't flash it. */
const LOADING_DELAY_MS = 250

const opens = (row: NodeEntry | undefined): row is NodeEntry =>
  row !== undefined && row.kind === 'dir' && row.hasChildren

/** The tree table: one folder's children, sortable, one click to go inside (ADR-0004). */
export function BrowseView({ screen }: { screen: BrowseScreen }) {
  const { t, locale } = useT()
  const browse = useStore((s) => s.browse)
  const showSummary = useStore((s) => s.showSummary)
  const { done, node } = screen
  const scan = done.summary.scan
  const [sort, setSort] = useState<Sort>({ key: 'allocated', order: 'desc' })
  // The folder just left by going up, so the table can select it again.
  const [cameFrom, setCameFrom] = useState<number | null>(null)
  const trail = useTrail(scan, node)

  const name = node.id === done.summary.root.id ? done.target.name : node.name
  const parent = trail && trail.length > 1 ? trail[trail.length - 2] : undefined

  const open = (target: NodeEntry) => {
    setCameFrom(null)
    browse(target)
  }
  const up = () => {
    if (!parent) return
    setCameFrom(node.id)
    browse(parent)
  }
  const onSort = (key: SortKey) =>
    setSort((s) => (s.key === key ? { key, order: s.order === 'asc' ? 'desc' : 'asc' } : { key, order: firstOrder(key) }))

  return (
    <section aria-labelledby="browse-title" className="mx-auto flex h-full w-full max-w-240 flex-col px-8 pt-8 pb-4">
      <h1 id="browse-title" className="text-title font-medium">
        <LeaderRow name={name} title={name} value={formatBytes(node.allocated, locale)} />
      </h1>

      <Table
        scan={scan}
        node={node}
        label={t('browse.tableLabel', { name })}
        sort={sort}
        cameFrom={cameFrom}
        onSort={onSort}
        onOpen={open}
        onUp={up}
        onExit={showSummary}
      />

      <p className="mt-3 text-meta text-ink-3">{t('browse.hint')}</p>
    </section>
  )
}

interface Pages {
  /** Which folder and sort these pages belong to. */
  key: string
  pages: Record<number, NodeEntry[]>
  /** Child count, known once the first page arrives. */
  total: number | null
  failed: boolean
}

const emptyPages = (key: string): Pages => ({ key, pages: {}, total: null, failed: false })

/**
 * Children of `node` in `sort` order, fetched a page at a time. A new folder
 * or sort starts empty; replies for an older key are dropped.
 */
function usePages(scan: ScanId, node: number, sort: Sort) {
  const key = `${scan}:${node}:${sort.key}:${sort.order}`
  const [state, setState] = useState(() => emptyPages(key))
  if (state.key !== key) setState(emptyPages(key))
  const current = state.key === key ? state : emptyPages(key)
  // Pages requested for the current key; a failed one is removed for retry.
  // Scoped to the key, so coming back to a folder fetches it again.
  const requested = useRef({ key, pages: new Set<number>() })

  const load = useCallback(
    (page: number) => {
      if (requested.current.key !== key) requested.current = { key, pages: new Set() }
      const asked = requested.current.pages
      if (asked.has(page)) return
      asked.add(page)
      getChildren(scan, node, sort.key, sort.order, page * PAGE, PAGE).then(
        (res) =>
          setState((s) =>
            s.key === key ? { ...s, total: res.total, pages: { ...s.pages, [page]: res.items } } : s,
          ),
        () => {
          asked.delete(page)
          setState((s) => (s.key === key ? { ...s, failed: true } : s))
        },
      )
    },
    [key, scan, node, sort.key, sort.order],
  )

  const retry = () => setState((s) => ({ ...s, failed: false }))
  const row = (i: number): NodeEntry | undefined => current.pages[Math.floor(i / PAGE)]?.[i % PAGE]

  return { pages: current, load, retry, row }
}

function Table({
  scan,
  node,
  label,
  sort,
  cameFrom,
  onSort,
  onOpen,
  onUp,
  onExit,
}: {
  scan: ScanId
  node: NodeEntry
  label: string
  sort: Sort
  cameFrom: number | null
  onSort: (key: SortKey) => void
  onOpen: (node: NodeEntry) => void
  onUp: () => void
  onExit: () => void
}) {
  const { t, locale } = useT()
  const { pages, load, retry, row } = usePages(scan, node.id, sort)
  const viewport = useRef<HTMLDivElement>(null)
  const grid = useRef<HTMLDivElement>(null)
  const [scrollTop, setScrollTop] = useState(0)
  const [height, setHeight] = useState(0)
  const [active, setActive] = useState(0)
  // Set once the folder just left has been selected in this list.
  const [placed, setPlaced] = useState(false)

  // A new folder or sort starts at the top with the first row selected.
  const listKey = `${node.id}:${sort.key}:${sort.order}`
  const [shown, setShown] = useState(listKey)
  if (shown !== listKey) {
    setShown(listKey)
    setActive(0)
    setScrollTop(0)
    setPlaced(false)
  }
  useEffect(() => {
    if (viewport.current) viewport.current.scrollTop = 0
  }, [listKey])

  useEffect(() => {
    const el = viewport.current
    if (!el) return
    const observer = new ResizeObserver(() => setHeight(el.clientHeight))
    observer.observe(el)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    grid.current?.focus()
  }, [])

  const count = pages.total ?? 0
  const slow = useLater(pages.total === null ? listKey : null, LOADING_DELAY_MS)
  const selectedRow = row(active)
  const first = Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN)
  const last = Math.min(count, Math.ceil((scrollTop + height) / ROW) + OVERSCAN)

  const failed = pages.failed
  useEffect(() => {
    if (failed) return
    load(0)
    if (last === 0) return
    for (let p = Math.floor(first / PAGE); p <= Math.floor((last - 1) / PAGE); p++) load(p)
  }, [first, last, load, failed])

  const reveal = useCallback((i: number) => {
    const el = viewport.current
    if (!el) return
    if (i * ROW < el.scrollTop) el.scrollTop = i * ROW
    else if ((i + 1) * ROW > el.scrollTop + el.clientHeight) el.scrollTop = (i + 1) * ROW - el.clientHeight
  }, [])

  // After going up, select the folder that was just left once its page is in.
  if (cameFrom !== null && !placed) {
    for (const [p, items] of Object.entries(pages.pages)) {
      const i = items.findIndex((item) => item.id === cameFrom)
      if (i >= 0) {
        setPlaced(true)
        setActive(Number(p) * PAGE + i)
        break
      }
    }
  }

  // Keep the selected row in view, whatever moved it.
  useEffect(() => reveal(active), [active, reveal])

  const move = (i: number) => {
    if (count > 0) setActive(Math.min(count - 1, Math.max(0, i)))
  }
  const page = Math.max(1, Math.floor(height / ROW) - 1)

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return
    switch (e.key) {
      case 'ArrowDown':
        move(active + 1)
        break
      case 'ArrowUp':
        move(active - 1)
        break
      case 'PageDown':
        move(active + page)
        break
      case 'PageUp':
        move(active - page)
        break
      case 'Home':
        move(0)
        break
      case 'End':
        move(count - 1)
        break
      case 'Enter':
      case 'ArrowRight': {
        const r = row(active)
        if (opens(r)) onOpen(r)
        break
      }
      case 'Backspace':
      case 'ArrowLeft':
        onUp()
        break
      case 'Escape':
        onExit()
        break
      default:
        return
    }
    e.preventDefault()
  }

  const columns: { key: SortKey; label: string; align: 'left' | 'right' }[] = [
    { key: 'name', label: t('browse.name'), align: 'left' },
    { key: 'allocated', label: t('browse.size'), align: 'right' },
    { key: 'allocated', label: t('browse.share'), align: 'right' },
    { key: 'files', label: t('browse.files'), align: 'right' },
    { key: 'modified', label: t('browse.modified'), align: 'right' },
  ]
  // Below md only name and size fit; share, files and date drop out.
  const cols =
    'grid grid-cols-[minmax(0,1fr)_6.5rem] gap-x-4 px-3 md:grid-cols-[minmax(0,1fr)_6.5rem_4.5rem_6rem_8.5rem]'
  const wide = 'hidden md:block'

  const rows = Array.from({ length: Math.max(0, last - first) }, (_, k) => {
    const i = first + k
    const r = row(i)
    const selected = i === active
    return (
      <div
        key={i}
        id={`browse-row-${i}`}
        role="row"
        aria-rowindex={i + 2}
        aria-selected={selected}
        onClick={() => {
          setActive(i)
          if (opens(r)) onOpen(r)
        }}
        className={`${cols} absolute inset-x-0 top-0 h-8 items-center rounded-control text-body ${
          opens(r) ? 'cursor-pointer' : ''
        } ${selected ? 'bg-raised shadow-[inset_2px_0_0_var(--c-accent)]' : 'hover:bg-raised'}`}
        style={{ transform: `translateY(${i * ROW}px)` }}
      >
        {r ? (
          <>
            <span role="gridcell" className="flex min-w-0 items-center gap-1">
              <bdi className="truncate" title={r.name}>
                {truncateMiddle(r.name, NAME_CHARS)}
              </bdi>
              {opens(r) && (
                <span aria-hidden className="shrink-0 text-ink-3">
                  <ChevronIcon />
                </span>
              )}
            </span>
            <span role="gridcell" className="text-right font-mono tabular-nums">
              {formatBytes(r.allocated, locale)}
            </span>
            <span role="gridcell" className={`${wide} text-right font-mono text-secondary text-ink-2 tabular-nums`}>
              {formatShare(r.allocated, node.allocated, locale)}
            </span>
            <span role="gridcell" className={`${wide} text-right font-mono text-secondary text-ink-2 tabular-nums`}>
              {r.kind === 'dir' ? formatCount(r.files, locale) : ''}
            </span>
            <span role="gridcell" className={`${wide} text-right text-secondary text-ink-2 tabular-nums`}>
              {formatDate(r.modified, locale)}
            </span>
          </>
        ) : (
          <span role="gridcell" className="text-ink-2">
            …
          </span>
        )}
      </div>
    )
  })

  return (
    <div
      ref={grid}
      role="grid"
      tabIndex={0}
      aria-label={label}
      aria-rowcount={count + 1}
      aria-activedescendant={count > 0 && row(active) ? `browse-row-${active}` : undefined}
      onKeyDown={onKeyDown}
      className="mt-4 flex min-h-0 flex-1 flex-col rounded-panel border border-hairline bg-card focus-visible:outline-offset-0"
    >
      <div role="row" aria-rowindex={1} className={`${cols} h-8 shrink-0 items-center border-b border-hairline text-secondary text-ink-2`}>
        {columns.map((c, i) => {
          // Size and share sort the same way; only the size header shows it.
          const sorted = sort.key === c.key && !(c.key === 'allocated' && i === 2)
          return (
            <span
              key={i}
              role="columnheader"
              aria-sort={sorted ? (sort.order === 'asc' ? 'ascending' : 'descending') : undefined}
              className={`${c.align === 'right' ? 'text-right' : ''} ${i >= 2 ? wide : ''}`}
            >
              {i === 2 ? (
                c.label
              ) : (
                <button
                  type="button"
                  onClick={() => onSort(c.key)}
                  className={`-mx-1 inline-flex min-h-6 items-center gap-1 rounded-tag px-1 hover:bg-raised hover:text-ink ${
                    sorted ? 'text-ink' : ''
                  }`}
                >
                  {c.label}
                  <span aria-hidden className={`w-2 font-mono ${sorted ? '' : 'invisible'}`}>
                    {sort.order === 'asc' ? '↑' : '↓'}
                  </span>
                </button>
              )}
            </span>
          )
        })}
      </div>

      <div
        ref={viewport}
        role="rowgroup"
        onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
        className="relative min-h-0 flex-1 overflow-auto px-1 py-1 [scrollbar-gutter:stable]"
      >
        {pages.failed ? (
          <p className="flex items-center gap-3 px-3 py-2 text-ink-2">
            {t('summary.childrenError')}
            <button
              type="button"
              onClick={retry}
              className="rounded-tag px-1 text-ink underline-offset-2 hover:underline"
            >
              {t('volumes.retry')}
            </button>
          </p>
        ) : pages.total === null ? (
          <p role="status" className="px-3 py-2 text-ink-2">
            {slow && t('browse.loading')}
          </p>
        ) : count === 0 ? (
          <p className="px-3 py-2 text-ink-2">{t('browse.empty')}</p>
        ) : (
          <div className="relative" style={{ height: count * ROW }}>
            {rows}
          </div>
        )}
      </div>

      {/* The selected name in full; the row already says it to a screen reader. */}
      <p aria-hidden className="flex h-8 shrink-0 items-center border-t border-hairline px-4 text-meta text-ink-2">
        {selectedRow && (
          <bdi className="min-w-0 truncate" title={selectedRow.name}>
            {selectedRow.name}
          </bdi>
        )}
      </p>
    </div>
  )
}

/** True once `key` has stayed the same, and not null, for `ms`. */
function useLater(key: string | null, ms: number): boolean {
  const [due, setDue] = useState<string | null>(null)
  useEffect(() => {
    if (key === null) return
    const id = window.setTimeout(() => setDue(key), ms)
    return () => window.clearTimeout(id)
  }, [key, ms])
  return key !== null && due === key
}
