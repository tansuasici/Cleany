import { type KeyboardEvent, useCallback, useEffect, useEffectEvent, useRef, useState } from 'react'
import { Button, ChevronIcon, LeaderRow, ShareBar } from '../components/ui'
import { formatBytes, formatCount, formatDate, formatShare, truncateMiddle } from '../lib/format'
import {
  type NodeEntry,
  type ScanId,
  type SortKey,
  type SortOrder,
  type TrashReport,
  getAncestors,
  getChildren,
  deleteNodes,
  trashNodes,
} from '../lib/ipc'
import { type BrowseScreen, useStore } from '../store'
import { BasketBar } from './Basket'
import { ActionBar, BasketTrashDialog, type Removal, type TrashNote, TrashDialog } from './BrowseActions'
import { useOs } from '../useOs'
import { type ActionTarget, actionKeys, deleteKey, trashKey, useBrowseActions } from './useBrowseActions'
import { Sunburst } from './Sunburst'
import { Treemap } from './Treemap'
import { useT } from '../useT'
import { useLater } from '../useLater'
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

const VIEWS = ['table', 'map', 'rings'] as const
type View = (typeof VIEWS)[number]
const VIEW_KEY = 'cleany.browseView'
const VIEW_TEXT = { table: 'browse.table', map: 'browse.map', rings: 'browse.rings' } as const
const HINT_TEXT = { table: 'browse.hint', map: 'browse.mapHint', rings: 'browse.ringsHint' } as const

function initialView(): View {
  try {
    const saved = localStorage.getItem(VIEW_KEY)
    return VIEWS.find((v) => v === saved) ?? 'table'
  } catch {
    // Storage can be unavailable; the table is the default.
    return 'table'
  }
}

/**
 * ⌘[ / ⌘] on macOS (Ctrl elsewhere) and Alt+← / Alt+→ everywhere move back
 * and forward in the folder history. Reads the key's place (`code`), so it
 * works on layouts where [ is not a key of its own. Bound in the capture
 * phase so the views never read Alt+← as ← (up a folder).
 */
function historyKeys(mac: boolean, go: (step: -1 | 1) => void) {
  return (e: KeyboardEvent<HTMLElement>) => {
    const el = e.target as HTMLElement
    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable) return
    if (el.closest('dialog')) return
    if (e.shiftKey) return
    const mod = mac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey
    const alt = e.altKey && !e.metaKey && !e.ctrlKey
    let step: -1 | 1
    if (mod && !e.altKey && e.code === 'BracketLeft') step = -1
    else if (mod && !e.altKey && e.code === 'BracketRight') step = 1
    else if (alt && e.key === 'ArrowLeft') step = -1
    else if (alt && e.key === 'ArrowRight') step = 1
    else return
    e.preventDefault()
    e.stopPropagation()
    if (!e.repeat) go(step)
  }
}

/** The trash question on show: one item, or the whole basket. */
type Asking = { kind: 'one'; target: ActionTarget; removal: Removal } | { kind: 'basket'; removal: Removal }

const NO_ITEMS: readonly number[] = []

const opens = (row: NodeEntry | undefined): row is NodeEntry =>
  row !== undefined && row.kind === 'dir' && row.hasChildren

/** The tree table: one folder's children, sortable, one click to go inside (ADR-0004). */
export function BrowseView({ screen }: { screen: BrowseScreen }) {
  const { t, locale } = useT()
  const browse = useStore((s) => s.browse)
  const showSummary = useStore((s) => s.showSummary)
  const trashed = useStore((s) => s.trashed)
  const basket = useStore((s) => s.basket)
  const permanentDelete = useStore((s) => s.permanentDelete)
  const addToBasket = useStore((s) => s.addToBasket)
  const removeFromBasket = useStore((s) => s.removeFromBasket)
  const clearBasket = useStore((s) => s.clearBasket)
  const refreshBasket = useStore((s) => s.refreshBasket)
  const history = useStore((s) => s.history)
  const go = useStore((s) => s.go)
  const { done, node } = screen
  const scan = done.summary.scan
  const [sort, setSort] = useState<Sort>({ key: 'allocated', order: 'desc' })
  // The folder just left by going up, so the table can select it again.
  const [cameFrom, setCameFrom] = useState<number | null>(null)
  const trail = useTrail(scan, node)
  const [view, setView] = useState<View>(initialView)
  const choose = (next: View) => {
    setView(next)
    try {
      localStorage.setItem(VIEW_KEY, next)
    } catch {
      // Not remembered; the choice still holds for this session.
    }
  }

  const name = node.id === done.summary.root.id ? done.target.name : node.name
  const parent = trail && trail.length > 1 ? trail[trail.length - 2] : undefined
  // The view's selection; with none (or "others"), the actions act on this folder.
  const [picked, setPicked] = useState<ActionTarget | null>(null)
  const target = picked ?? { id: node.id, name }
  const actions = useBrowseActions(scan, target)
  const os = useOs()

  // Moving to the trash: what is being asked about, and what the last move did.
  const [asking, setAsking] = useState<Asking | null>(null)
  const [working, setWorking] = useState(false)
  const [note, setNote] = useState<TrashNote | null>(null)
  // Bumped after a move, so the view on show reads the tree again.
  const [rev, setRev] = useState(0)
  const canTrash = target.id !== done.summary.root.id
  const askTrash = () => {
    if (canTrash && !asking) setAsking({ kind: 'one', target, removal: 'trash' })
  }
  const askDelete = () => {
    if (permanentDelete && canTrash && !asking) setAsking({ kind: 'one', target, removal: 'delete' })
  }

  const basketItems = basket?.scan === scan ? basket.items : null
  const basketIds = basketItems?.map((b) => b.id) ?? NO_ITEMS
  const inBasket = basketIds.includes(target.id)
  const toggleBasket = async () => {
    if (!canTrash) return
    const at = target
    if (inBasket) {
      removeFromBasket(at.id)
      setNote({ status: t('basket.removed', { name: at.name }), alert: '' })
      return
    }
    let chain: NodeEntry[]
    try {
      chain = await getAncestors(scan, at.id)
    } catch {
      setNote({ status: '', alert: t('basket.addError', { name: at.name }) })
      return
    }
    const entry = chain.at(-1)
    if (!entry) return
    const { id, kind, allocated, files } = entry
    const result = addToBasket(scan, {
      id,
      name: at.name,
      kind,
      allocated,
      files,
      chain: chain.slice(0, -1).map((c) => c.id),
    })
    const status =
      result.kind === 'covered'
        ? t('basket.covered', { name: at.name, by: result.by })
        : result.replaced > 0
          ? t('basket.addedFolder', {
              name: at.name,
              n: formatCount(result.replaced, locale),
              count: result.replaced,
            })
          : t('basket.added', { name: at.name })
    setNote({ status, alert: '' })
  }
  const askBasket = () => {
    if (basketItems && !asking) setAsking({ kind: 'basket', removal: 'trash' })
  }
  const askBasketDelete = () => {
    if (permanentDelete && basketItems && !asking) setAsking({ kind: 'basket', removal: 'delete' })
  }

  const describe = (report: TrashReport, removal: Removal): TrashNote => {
    const many = removal === 'trash' ? 'basket.done' : 'delete.basketDone'
    const one = removal === 'trash' ? 'trash.done' : 'delete.done'
    const alerts: string[] = []
    let status = ''
    for (const item of report.items) {
      const name = item.name
      switch (item.outcome.kind) {
        case 'trashed':
        case 'deleted':
          status =
            report.trashed > 1
              ? t(many, {
                  n: formatCount(report.trashed, locale),
                  count: report.trashed,
                  size: formatBytes(report.freed.allocated, locale),
                })
              : t(one, { name, size: formatBytes(report.freed.allocated, locale) })
          break
        case 'gone':
          alerts.push(t('trash.gone', { name }))
          break
        case 'protected':
          alerts.push(t(removal === 'trash' ? 'trash.protected' : 'delete.protected', { name }))
          break
        case 'failed':
          alerts.push(
            t(removal === 'trash' ? 'trash.failed' : 'delete.failed', { name, reason: item.outcome.message }),
          )
          break
      }
    }
    return { status, alert: alerts.join(' ') }
  }

  /**
   * Back to the folder on show (with its new size), or the nearest one above
   * it that is still in the tree. Returns `false` when it had to leave it.
   */
  const settle = async (): Promise<boolean> => {
    const chain = [node, ...(trail ?? []).slice(0, -1).reverse()]
    for (const folder of chain) {
      try {
        const list = await getAncestors(scan, folder.id)
        const fresh = list[list.length - 1]
        if (!fresh) continue
        if (fresh.id !== node.id) setCameFrom(null)
        browse(fresh)
        return fresh.id === node.id
      } catch {
        // Taken out of the tree; try the folder above.
      }
    }
    showSummary()
    return false
  }

  // Disk changes were patched into the tree: the views read it again on their
  // own (`revision`); the folder on show may have changed size or be gone.
  const revision = done.revision
  const followed = useRef(revision)
  const follow = useEffectEvent(() => {
    void settle().then((stayed) => {
      if (!stayed) setNote({ status: t('browse.folderGone'), alert: '' })
    })
  })
  useEffect(() => {
    if (followed.current === revision) return
    followed.current = revision
    follow()
  }, [revision])

  const confirmTrash = async () => {
    if (!asking) return
    const ids = asking.kind === 'one' ? [asking.target.id] : (basketItems ?? []).map((b) => b.id)
    if (ids.length === 0) return
    const { removal } = asking
    setWorking(true)
    let report: TrashReport
    try {
      report = await (removal === 'trash' ? trashNodes : deleteNodes)(scan, ids)
    } catch {
      setNote({ status: '', alert: t(removal === 'trash' ? 'trash.error' : 'delete.error') })
      return
    } finally {
      // Closed before the view is rebuilt, so focus can go back to it.
      setWorking(false)
      setAsking(null)
    }
    setNote(describe(report, removal))
    if (report.root) trashed(scan, report.root)
    if (report.items.some((i) => i.outcome.kind === 'trashed' || i.outcome.kind === 'deleted' || i.outcome.kind === 'gone')) {
      // Drops what went and what was inside it; protected and failed items stay.
      await refreshBasket(scan)
      setPicked(null)
      await settle()
      setRev((r) => r + 1)
    }
  }

  const open = (target: NodeEntry) => {
    setCameFrom(null)
    setNote(null)
    browse(target)
  }
  const up = () => {
    if (!parent) return
    setCameFrom(node.id)
    setNote(null)
    browse(parent)
  }
  const ownHistory = history?.scan === scan ? history : null
  const canBack = ownHistory !== null && ownHistory.at > 0
  const canForward = ownHistory !== null && ownHistory.at < ownHistory.entries.length - 1
  const step = (by: -1 | 1) => {
    if (by < 0 ? !canBack : !canForward) return
    // Set first, so the folder that comes into view selects the one left.
    setCameFrom(node.id)
    void go(by).then((moved) => {
      if (moved) setNote(null)
    })
  }
  const mac = os === 'macos'
  const navKeys = {
    back: { label: mac ? '⌘[' : 'Alt+←', aria: mac ? 'Meta+[ Alt+ArrowLeft' : 'Control+[ Alt+ArrowLeft' },
    forward: { label: mac ? '⌘]' : 'Alt+→', aria: mac ? 'Meta+] Alt+ArrowRight' : 'Control+] Alt+ArrowRight' },
  }
  // `px-0!`: Button's own `px-3` comes later in the stylesheet and would win,
  // leaving the icon 6px. `aria-disabled`, not `disabled`: a focused button that turns disabled
  // drops focus to the page, and the section's keys stop working.
  const navLook =
    'w-8 justify-center px-0! aria-disabled:cursor-default aria-disabled:opacity-50 aria-disabled:hover:bg-card'

  const onSort = (key: SortKey) =>
    setSort((s) => (s.key === key ? { key, order: s.order === 'asc' ? 'desc' : 'asc' } : { key, order: firstOrder(key) }))

  return (
    <section
      aria-labelledby="browse-title"
      onKeyDown={actionKeys(actions, () => void toggleBasket())}
      onKeyDownCapture={(e) => {
        trashKey(mac, askTrash)(e)
        if (permanentDelete) deleteKey(mac, askDelete)(e)
        historyKeys(mac, step)(e)
      }}
      className="mx-auto flex h-full w-full max-w-240 flex-col px-8 pt-8 pb-4">
      <div className="flex min-w-0 items-center gap-3">
        <div role="group" aria-label={t('browse.history')} className="flex shrink-0 gap-1">
          <Button
            aria-label={t('browse.back')}
            aria-keyshortcuts={navKeys.back.aria}
            aria-disabled={!canBack}
            title={`${t('browse.back')} (${navKeys.back.label})`}
            onClick={() => step(-1)}
            className={navLook}
          >
            <span className="flex shrink-0 -scale-x-100">
              <ChevronIcon />
            </span>
          </Button>
          <Button
            aria-label={t('browse.forward')}
            aria-keyshortcuts={navKeys.forward.aria}
            aria-disabled={!canForward}
            title={`${t('browse.forward')} (${navKeys.forward.label})`}
            onClick={() => step(1)}
            className={navLook}
          >
            <span className="flex shrink-0">
              <ChevronIcon />
            </span>
          </Button>
        </div>
        <h1 id="browse-title" className="min-w-0 flex-1 text-title font-medium">
          <LeaderRow name={name} title={name} value={formatBytes(node.allocated, locale)} />
        </h1>
      </div>

      <div role="group" aria-label={t('browse.view')} className="mt-4 flex gap-1 self-start rounded-control border border-hairline bg-card p-0.5">
        {VIEWS.map((v) => (
          <button
            key={v}
            type="button"
            aria-pressed={view === v}
            onClick={() => choose(v)}
            className={`h-7 rounded-tag px-3 text-secondary transition-[background-color,color] duration-150 ease-out ${
              view === v ? 'bg-raised text-ink' : 'text-ink-2 hover:text-ink'
            }`}
          >
            {t(VIEW_TEXT[v])}
          </button>
        ))}
      </div>

      {view === 'table' ? (
        <Table
          key={rev}
          scan={scan}
          node={node}
          revision={done.revision}
          label={t('browse.tableLabel', { name })}
          sort={sort}
          cameFrom={cameFrom}
          onSort={onSort}
          onOpen={open}
          onUp={up}
          onExit={showSummary}
          onSelect={setPicked}
          basketIds={basketIds}
        />
      ) : view === 'map' ? (
        <Treemap
          key={rev}
          scan={scan}
          node={node}
          revision={done.revision}
          label={t('browse.mapLabel', { name })}
          cameFrom={cameFrom}
          onOpen={open}
          onUp={up}
          onExit={showSummary}
          onSelect={setPicked}
        />
      ) : (
        <Sunburst
          key={rev}
          scan={scan}
          node={node}
          revision={done.revision}
          name={name}
          label={t('browse.ringsLabel', { name })}
          canGoUp={parent !== undefined}
          cameFrom={cameFrom}
          onOpen={open}
          onUp={up}
          onExit={showSummary}
          onSelect={setPicked}
        />
      )}

      <div className="mt-3 flex flex-col gap-2">
        <ActionBar
          target={target}
          actions={actions}
          os={os}
          canTrash={canTrash}
          onTrash={askTrash}
          inBasket={inBasket}
          onBasket={() => void toggleBasket()}
          permanent={permanentDelete}
          onDelete={askDelete}
          note={note}
        />
        {basketItems && (
          <BasketBar
            basket={{ scan, items: basketItems }}
            onTrash={askBasket}
            onDelete={permanentDelete ? askBasketDelete : undefined}
            onRemove={removeFromBasket}
            onClear={clearBasket}
          />
        )}
        <p className="text-meta text-ink-3">{t(HINT_TEXT[view])}</p>
      </div>

      {asking?.kind === 'one' && (
        <TrashDialog
          removal={asking.removal}
          scan={scan}
          target={asking.target}
          working={working}
          onCancel={() => setAsking(null)}
          onConfirm={() => void confirmTrash()}
        />
      )}
      {asking?.kind === 'basket' && basketItems && (
        <BasketTrashDialog
          removal={asking.removal}
          items={basketItems}
          working={working}
          onCancel={() => setAsking(null)}
          onConfirm={() => void confirmTrash()}
        />
      )}
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
function usePages(scan: ScanId, node: number, sort: Sort, revision: number) {
  const key = `${scan}:${node}:${sort.key}:${sort.order}`
  const [state, setState] = useState(() => emptyPages(key))
  if (state.key !== key) setState(emptyPages(key))
  const current = state.key === key ? state : emptyPages(key)
  // Pages requested for the current key and tree revision; a failed one is
  // removed for retry. Scoped to the key, so coming back to a folder fetches
  // it again; a newer revision fetches the pages again while the old rows
  // stay on show.
  const requested = useRef({ key, revision, pages: new Set<number>() })

  const load = useCallback(
    (page: number) => {
      if (requested.current.key !== key || requested.current.revision !== revision) {
        requested.current = { key, revision, pages: new Set() }
      }
      const round = requested.current
      const asked = round.pages
      if (asked.has(page)) return
      asked.add(page)
      // A reply for an older folder, sort or revision is dropped.
      const fresh = () => requested.current === round
      getChildren(scan, node, sort.key, sort.order, page * PAGE, PAGE).then(
        (res) => {
          if (!fresh()) return
          setState((s) =>
            s.key === key ? { ...s, total: res.total, pages: { ...s.pages, [page]: res.items } } : s,
          )
        },
        () => {
          asked.delete(page)
          if (fresh()) setState((s) => (s.key === key ? { ...s, failed: true } : s))
        },
      )
    },
    [key, revision, scan, node, sort.key, sort.order],
  )

  const retry = () => setState((s) => ({ ...s, failed: false }))
  const row = (i: number): NodeEntry | undefined => current.pages[Math.floor(i / PAGE)]?.[i % PAGE]

  return { pages: current, load, retry, row }
}

function Table({
  scan,
  node,
  revision,
  label,
  sort,
  cameFrom,
  onSort,
  onOpen,
  onUp,
  onExit,
  onSelect,
  basketIds,
}: {
  scan: ScanId
  node: NodeEntry
  /** The tree's revision; a newer one reads the rows again. */
  revision: number
  label: string
  sort: Sort
  cameFrom: number | null
  onSort: (key: SortKey) => void
  onOpen: (node: NodeEntry) => void
  onUp: () => void
  onExit: () => void
  onSelect: (target: ActionTarget | null) => void
  /** Rows in the basket get a mark before the name. */
  basketIds: readonly number[]
}) {
  const { t, locale } = useT()
  const { pages, load, retry, row } = usePages(scan, node.id, sort, revision)
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
  const selectedId = selectedRow?.id
  const selectedName = selectedRow?.name
  useEffect(() => {
    onSelect(selectedId === undefined || selectedName === undefined ? null : { id: selectedId, name: selectedName })
  }, [selectedId, selectedName, onSelect])
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
    'grid grid-cols-[minmax(0,1fr)_6.5rem] gap-x-4 px-3 md:grid-cols-[minmax(0,1fr)_6.5rem_8rem_6rem_8.5rem]'
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
        className={`${cols} absolute inset-x-0 top-0 h-8 cursor-pointer items-center rounded-control text-body ${
          // Keyboard focus sits on the grid; the ring goes on the active row, where the eye is.
          selected
            ? 'bg-raised group-focus-visible:outline-2 group-focus-visible:-outline-offset-2 group-focus-visible:outline-accent'
            : 'hover:bg-raised/50'
        }`}
        style={{ transform: `translateY(${i * ROW}px)` }}
      >
        {r ? (
          <>
            <span role="gridcell" className="flex min-w-0 items-center gap-1">
              {basketIds.includes(r.id) && (
                <span className="mr-1 size-1.5 shrink-0 rounded-full bg-ink-2">
                  <span className="sr-only">{t('basket.mark')}</span>
                </span>
              )}
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
            <span role="gridcell" className={`${wide} text-secondary text-ink-2 tabular-nums`}>
              <span className="flex items-center justify-end gap-2">
                <span className="w-16">
                  <ShareBar ratio={node.allocated > 0 ? r.allocated / node.allocated : 0} />
                </span>
                <span className="w-10 text-right">{formatShare(r.allocated, node.allocated, locale)}</span>
              </span>
            </span>
            <span role="gridcell" className={`${wide} text-right text-secondary text-ink-2 tabular-nums`}>
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
      className="group mt-4 flex min-h-0 flex-1 flex-col rounded-panel border border-hairline bg-card focus-visible:border-accent focus-visible:outline-none"
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
          <p role="alert" className="flex items-center gap-3 px-3 py-2 text-ink-2">
            {t('summary.childrenError')}
            <button
              type="button"
              onClick={retry}
              className="inline-flex min-h-6 items-center rounded-tag px-1 text-ink underline-offset-2 hover:underline"
            >
              {t('volumes.retry')}
            </button>
          </p>
        ) : pages.total === null || count === 0 ? (
          // One status line for loading and empty, so the empty result is announced too.
          <p role="status" className="px-3 py-2 text-ink-2">
            {pages.total === null ? slow && t('browse.loading') : t('browse.empty')}
          </p>
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
