import { type KeyboardEvent, useEffect, useRef, useState } from 'react'
import { ChevronIcon, EmptyLine, Segmented, SkeletonRows } from '../components/ui'
import { formatBytes, truncateMiddle } from '../lib/format'
import { type LargestEntry, type LargestKind, type NodeEntry, type ScanId, getLargest } from '../lib/ipc'
import { useLater } from '../useLater'
import { useT } from '../useT'
import { type ActionTarget, type MenuAt, menuPoint } from './useBrowseActions'

/** How many rows the list shows (docs/PLAN.md §M5). */
export const LIMIT = 100
/** DESIGN_MAP → Spacing: a list row is 32 tall, as in the table. */
const ROW = 32
/** Names longer than this are cut in the middle; the full name is in the footer and the tooltip. */
const NAME_CHARS = 64
/** Locations longer than this are cut in the middle. */
const DIR_CHARS = 30
/** The loading line waits this long, so a quick folder change doesn't flash it. */
const LOADING_DELAY_MS = 250

const KINDS = ['files', 'dirs'] as const
const KIND_TEXT = { files: 'browse.largestFiles', dirs: 'browse.largestDirs' } as const

type Rows = { key: string; rows: LargestEntry[] | null; failed: boolean }

const opens = (e: NodeEntry | undefined): e is NodeEntry => e !== undefined && e.kind === 'dir' && e.hasChildren

/**
 * The largest files, or the folders holding the most files directly,
 * anywhere below the folder on show. Selection and keys work as in the table.
 */
export function Largest({
  scan,
  node,
  revision,
  label,
  kind,
  onKind,
  onOpen,
  onUp,
  onExit,
  onSelect,
  onMenu,
  hint,
}: {
  scan: ScanId
  node: NodeEntry
  /** The tree's revision; a newer one reads the list again. */
  revision: number
  label: string
  kind: LargestKind
  onKind: (kind: LargestKind) => void
  onOpen: (node: NodeEntry) => void
  onUp: () => void
  onExit: () => void
  onSelect: (target: ActionTarget | null) => void
  /** Opens the item menu for the item just selected. */
  onMenu: (at: MenuAt) => void
  /** The short key hint shown at the right of the footer. */
  hint: string
}) {
  const { t, locale } = useT()
  const key = `${scan}:${node.id}:${kind}`
  const [state, setState] = useState<Rows>({ key, rows: null, failed: false })
  const [attempt, setAttempt] = useState(0)
  const [active, setActive] = useState(0)
  const grid = useRef<HTMLDivElement>(null)
  const viewport = useRef<HTMLDivElement>(null)

  // A new folder or kind starts empty at the top; a newer revision keeps the
  // old rows on show until the new ones arrive.
  const [shown, setShown] = useState(key)
  if (shown !== key) {
    setShown(key)
    setState({ key, rows: null, failed: false })
    setActive(0)
  }

  useEffect(() => {
    let live = true
    getLargest(scan, node.id, kind, LIMIT).then(
      (rows) => {
        if (live) setState({ key, rows, failed: false })
      },
      () => {
        if (live) setState((s) => ({ ...s, key, failed: true }))
      },
    )
    return () => {
      live = false
    }
  }, [key, scan, node.id, kind, revision, attempt])

  useEffect(() => {
    grid.current?.focus()
  }, [])

  const rows = state.key === key ? state.rows : null
  const count = rows?.length ?? 0
  const slow = useLater(rows === null ? key : null, LOADING_DELAY_MS)
  // A refresh can shorten the list under the selection.
  const at = Math.min(active, Math.max(0, count - 1))
  const selected = rows?.[at]?.entry

  const selectedId = selected?.id
  const selectedName = selected?.name
  useEffect(() => {
    onSelect(selectedId === undefined || selectedName === undefined ? null : { id: selectedId, name: selectedName })
  }, [selectedId, selectedName, onSelect])

  // Keep the selected row in view, whatever moved it.
  useEffect(() => {
    const el = viewport.current
    if (!el) return
    if (at * ROW < el.scrollTop) el.scrollTop = at * ROW
    else if ((at + 1) * ROW > el.scrollTop + el.clientHeight) el.scrollTop = (at + 1) * ROW - el.clientHeight
  }, [at])

  const move = (i: number) => {
    if (count > 0) setActive(Math.min(count - 1, Math.max(0, i)))
  }
  const page = () => Math.max(1, Math.floor((viewport.current?.clientHeight ?? 0) / ROW) - 1)

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return
    switch (e.key) {
      case 'ArrowDown':
        move(at + 1)
        break
      case 'ArrowUp':
        move(at - 1)
        break
      case 'PageDown':
        move(at + page())
        break
      case 'PageUp':
        move(at - page())
        break
      case 'Home':
        move(0)
        break
      case 'End':
        move(count - 1)
        break
      case 'Enter':
      case 'ArrowRight':
        if (opens(selected)) onOpen(selected)
        break
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

  const dirs = kind === 'dirs'
  // Below md only name and size fit; location and total drop out.
  const cols = `grid grid-cols-[minmax(0,1fr)_6.5rem] gap-x-4 px-3 ${
    dirs
      ? 'md:grid-cols-[minmax(0,1fr)_minmax(0,14rem)_6.5rem_6.5rem]'
      : 'md:grid-cols-[minmax(0,1fr)_minmax(0,14rem)_6.5rem]'
  }`
  const wide = 'hidden md:block'

  return (
    <div className="mt-4 flex min-h-0 flex-1 flex-col">
      <Segmented
        label={t('browse.largestKind')}
        options={KINDS.map((k) => ({ key: k, text: t(KIND_TEXT[k]) }))}
        value={kind}
        onChange={onKind}
      />

      <div
        ref={grid}
        role="grid"
        tabIndex={0}
        aria-label={label}
        aria-rowcount={count + 1}
        aria-activedescendant={selected ? `largest-row-${at}` : undefined}
        onKeyDown={onKeyDown}
        onContextMenu={(e) => {
          if (e.target === e.currentTarget && selected) onMenu(menuPoint(e))
        }}
        className="group mt-2 flex min-h-0 flex-1 flex-col rounded-panel border border-card-edge bg-card shadow-card focus-visible:outline-none"
      >
        <div role="row" aria-rowindex={1} className={`${cols} h-8 shrink-0 items-center border-b border-hairline text-secondary text-ink-2`}>
          <span role="columnheader">{t('browse.name')}</span>
          <span role="columnheader" className={wide}>
            {t('browse.largestWhere')}
          </span>
          <span role="columnheader" aria-sort="descending" className="text-right">
            {dirs ? t('browse.largestOwn') : t('browse.size')}
          </span>
          {dirs && (
            <span role="columnheader" className={`${wide} text-right`}>
              {t('browse.largestTotal')}
            </span>
          )}
        </div>

        <div ref={viewport} role="rowgroup" className="relative min-h-0 flex-1 overflow-auto px-1 py-1 [scrollbar-gutter:stable]">
          {state.failed && rows === null ? (
            <p role="alert" className="flex items-center gap-3 px-3 py-2 text-ink-2">
              {t('summary.childrenError')}
              <button
                type="button"
                onClick={() => {
                  setState((s) => ({ ...s, failed: false }))
                  setAttempt((n) => n + 1)
                }}
                className="inline-flex min-h-6 items-center rounded-tag px-1 text-ink underline-offset-2 hover:underline"
              >
                {t('volumes.retry')}
              </button>
            </p>
          ) : rows === null ? (
            slow && <SkeletonRows label={t('browse.loading')} />
          ) : count === 0 ? (
            <EmptyLine text={t('browse.largestEmpty')} action={t('browse.goUp')} onAction={onUp} />
          ) : (
            rows.map((r, i) => {
              const e = r.entry
              return (
                <div
                  key={e.id}
                  id={`largest-row-${i}`}
                  role="row"
                  aria-rowindex={i + 2}
                  aria-selected={i === at}
                  onClick={() => {
                    setActive(i)
                    if (opens(e)) onOpen(e)
                  }}
                  onContextMenu={(ev) => {
                    ev.preventDefault()
                    setActive(i)
                    onMenu(menuPoint(ev))
                  }}
                  className={`${cols} h-8 cursor-pointer items-center rounded-control text-body ${
                    // Keyboard focus sits on the grid; the ring goes on the active row, where the eye is.
                    i === at
                      ? 'bg-raised'
                      : 'hover:bg-raised/50'
                  }`}
                >
                  <span role="gridcell" className="flex min-w-0 items-center gap-1">
                    <bdi className="truncate" title={e.name}>
                      {truncateMiddle(e.name, NAME_CHARS)}
                    </bdi>
                    {opens(e) && (
                      <span aria-hidden className="shrink-0 text-ink-3">
                        <ChevronIcon />
                      </span>
                    )}
                  </span>
                  <span role="gridcell" className={`${wide} min-w-0 truncate text-secondary text-ink-2`}>
                    {r.dir ? (
                      <bdi title={r.dir}>{truncateMiddle(r.dir, DIR_CHARS)}</bdi>
                    ) : (
                      <span className="text-ink-3">{t('browse.largestHere')}</span>
                    )}
                  </span>
                  <span role="gridcell" className="text-right tabular-nums">
                    {formatBytes(r.own, locale)}
                  </span>
                  {dirs && (
                    <span role="gridcell" className={`${wide} text-right text-secondary text-ink-3 tabular-nums`}>
                      {formatBytes(e.allocated, locale)}
                    </span>
                  )}
                </div>
              )
            })
          )}
        </div>

        {/* The selected name in full; the row already says it to a screen reader. */}
        <div className="flex h-8 shrink-0 items-center gap-3 px-4 text-meta text-ink-2">
          {selected && (
            <bdi aria-hidden className="min-w-0 truncate" title={selected.name}>
              {selected.name}
            </bdi>
          )}
          <span className="ml-auto shrink-0 text-ink-3 max-md:hidden">{hint}</span>
        </div>
      </div>
    </div>
  )
}
