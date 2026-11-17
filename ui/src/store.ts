import { create } from 'zustand'
import {
  type DiskAccess,
  type NodeEntry,
  type NodeKind,
  type ScanEvent,
  type ScanId,
  type ScanSummary,
  type VolumeEntry,
  attachScan,
  cancelScan,
  getAncestors,
  getChildren,
  getDiskAccess,
  listVolumes,
  startScan,
} from './lib/ipc'
import { type Locale, detectLocale } from './lib/i18n'

/** What is being scanned. */
export interface Target {
  path: string
  name: string
  /** The volume's used and free bytes when the target is a whole volume. */
  used?: number
  free?: number
}

export interface Progress {
  dirs: number
  files: number
  allocated: number
  elapsedMs: number
}

/** The largest items of a finished scan, as the döküm lists them. */
export type TopItems =
  | { status: 'loading' }
  | { status: 'ready'; items: NodeEntry[]; total: number }
  | { status: 'error' }

/** A finished scan; it stays openable until the next scan starts. */
export type DoneScreen = {
  kind: 'done'
  target: Target
  summary: ScanSummary
  top: TopItems
  /** Bumped each time disk changes are patched into the tree; views read it again. */
  revision: number
  /** Too much changed on disk to keep up; the result is shown as it was. */
  outdated: boolean
}

const finished = (target: Target, summary: ScanSummary): DoneScreen => ({
  kind: 'done',
  target,
  summary,
  top: { status: 'loading' },
  revision: 0,
  outdated: false,
})

/** The tree table, open at `node` (a folder) of a finished scan. */
export type BrowseScreen = { kind: 'browse'; done: DoneScreen; node: NodeEntry }

/** The screen on show; each variant carries exactly the data it needs. */
export type Screen =
  | { kind: 'volumes' }
  | { kind: 'scanning'; target: Target; scan: ScanId | null; progress: Progress | null }
  | DoneScreen
  | BrowseScreen
  | { kind: 'cancelled'; target: Target }
  | { kind: 'failed'; target: Target; message: string }

export type Volumes =
  | { status: 'loading' }
  | { status: 'ready'; list: VolumeEntry[] }
  | { status: 'error'; message: string }

/** The last finished scan of a volume, for splitting its fullness bar. */
export interface VolumeBreakdown {
  items: { name: string; allocated: number }[]
  allocated: number
  /** When the scan finished, in ms since the epoch. */
  at: number
}

/** An item put aside to move to the trash together with others. */
export interface BasketItem {
  id: number
  name: string
  kind: NodeKind
  allocated: number
  files: number
  /** Ids of the folders above it, root first; finds overlaps between items. */
  chain: number[]
}

/** The selection basket; it belongs to one scan, so a new scan clears it. */
export interface Basket {
  scan: ScanId
  items: BasketItem[]
}

/**
 * Folders opened in the tree table of one scan, oldest first; `at` is the one
 * on show. Back and forward move `at`, as in a browser.
 */
export interface History {
  scan: ScanId
  entries: NodeEntry[]
  at: number
}

/** What adding to the basket did. */
export type BasketAdd =
  | { kind: 'added'; replaced: number }
  /** A folder already in the basket holds the item. */
  | { kind: 'covered'; by: string }

/** How many items the döküm lists. */
export const TOP_ITEMS = 1000

interface State {
  locale: Locale
  /** macOS Full Disk Access; `null` until the first check answers. */
  access: DiskAccess | null
  /** The user put the Full Disk Access panel aside ("Şimdilik geç"). */
  accessDismissed: boolean
  /** Settings allow deleting without the trash; off until the user turns it on. */
  permanentDelete: boolean
  volumes: Volumes
  screen: Screen
  /** Keyed by mount point. */
  breakdowns: Record<string, VolumeBreakdown>
  /**
   * The last finished scan. Starting a scan clears it, because the backend
   * keeps one tree and a new scan makes the old ids stale.
   */
  last: DoneScreen | null
  /** Döküm folders opened in place, by node id; kept while moving to the table and back. */
  opened: number[]
  setLocale: (locale: Locale) => void
  /** Re-checks Full Disk Access; run at start and whenever the window regains focus. */
  checkAccess: () => Promise<void>
  dismissAccess: () => void
  setPermanentDelete: (on: boolean) => void
  loadVolumes: () => Promise<void>
  scan: (target: Target) => Promise<void>
  /**
   * Reopens the backend's finished scan after the webview reloaded (a dev
   * reload, or the page being rebuilt), instead of scanning again. Does
   * nothing if the user has already moved on or no scan finished.
   */
  reattach: () => Promise<void>
  cancel: () => Promise<void>
  showVolumes: () => void
  openLast: () => void
  /** Opens the tree table at `node`, from the döküm or the table itself. */
  browse: (node: NodeEntry) => void
  /** Folder history of the tree table; a new scan clears it. */
  history: History | null
  /**
   * Moves back (`-1`) or forward (`1`) in the folder history. A folder no
   * longer in the tree is dropped and the next one is tried. Resolves `true`
   * once a folder is on show.
   */
  go: (step: -1 | 1) => Promise<boolean>
  /** Leaves the tree table for its döküm. */
  showSummary: () => void
  /** Opens or closes a döküm folder in place. */
  toggleOpened: (id: number) => void
  /**
   * Takes in the scan root as it is after a move to the trash, and lists the
   * döküm again; the table stays where it is.
   */
  trashed: (scan: ScanId, root: NodeEntry) => void
  basket: Basket | null
  /**
   * Puts an item in the basket. An item inside a basket folder is not added;
   * a folder takes the place of the basket items inside it.
   */
  addToBasket: (scan: ScanId, item: BasketItem) => BasketAdd
  removeFromBasket: (id: number) => void
  clearBasket: () => void
  /**
   * After a move to the trash: drops the items that are gone and takes in the
   * new sizes of the rest (a basket folder may have lost a file).
   */
  refreshBasket: (scan: ScanId) => Promise<void>
}

const LOCALE_KEY = 'cleany.locale'

function initialLocale(): Locale {
  try {
    const saved = localStorage.getItem(LOCALE_KEY)
    if (saved === 'tr' || saved === 'en') return saved
  } catch {
    // Storage can be unavailable; fall back to the system language.
  }
  return detectLocale()
}

const ACCESS_DISMISSED_KEY = 'cleany.accessDismissed'

function initialAccessDismissed(): boolean {
  try {
    return localStorage.getItem(ACCESS_DISMISSED_KEY) === '1'
  } catch {
    return false
  }
}

const PERMANENT_DELETE_KEY = 'cleany.permanentDelete'

function initialPermanentDelete(): boolean {
  try {
    return localStorage.getItem(PERMANENT_DELETE_KEY) === '1'
  } catch {
    return false
  }
}

const TARGET_KEY = 'cleany.target'

/** Remembers what a finished scan was of; the backend knows only its path. */
function saveTarget(scan: ScanId, target: Target) {
  try {
    sessionStorage.setItem(TARGET_KEY, JSON.stringify({ scan, target }))
  } catch {
    // The fallback in `savedTarget` names it from the path.
  }
}

/** The target saved for `scan`, or one named after the root's path. */
function savedTarget(summary: ScanSummary): Target {
  try {
    const raw = sessionStorage.getItem(TARGET_KEY)
    const saved = raw ? (JSON.parse(raw) as { scan?: unknown; target?: Target }) : null
    if (saved?.scan === summary.scan && typeof saved.target?.path === 'string') return saved.target
  } catch {
    // Unreadable storage or a malformed entry; fall back below.
  }
  // The root's name is the scanned path itself.
  const path = summary.root.name
  return { path, name: path.split('/').filter(Boolean).at(-1) ?? path }
}

const message = (error: unknown) =>
  typeof error === 'object' && error !== null && 'message' in error
    ? String((error as { message: unknown }).message)
    : String(error)

// Each scan bumps the generation; events and replies from an older one are
// dropped, so a late `done` can never overwrite a newer screen.
let generation = 0
// Cancel pressed before `startScan` returned the id; applied once it does.
let cancelPending = false

export const useStore = create<State>()((set, get) => ({
  locale: initialLocale(),
  access: null,
  accessDismissed: initialAccessDismissed(),
  permanentDelete: initialPermanentDelete(),
  volumes: { status: 'loading' },
  screen: { kind: 'volumes' },
  breakdowns: {},
  last: null,
  opened: [],
  basket: null,
  history: null,

  setLocale: (locale) => {
    try {
      localStorage.setItem(LOCALE_KEY, locale)
    } catch {
      // Not persisting the choice is acceptable.
    }
    set({ locale })
  },

  checkAccess: async () => {
    try {
      set({ access: await getDiskAccess() })
    } catch {
      // A failed check says nothing about access; show nothing.
      set({ access: 'unknown' })
    }
  },

  dismissAccess: () => {
    try {
      localStorage.setItem(ACCESS_DISMISSED_KEY, '1')
    } catch {
      // Not persisting the choice is acceptable.
    }
    set({ accessDismissed: true })
  },

  setPermanentDelete: (on) => {
    try {
      if (on) localStorage.setItem(PERMANENT_DELETE_KEY, '1')
      else localStorage.removeItem(PERMANENT_DELETE_KEY)
    } catch {
      // Not persisting the choice is acceptable; it holds for this session.
    }
    set({ permanentDelete: on })
  },

  loadVolumes: async () => {
    set({ volumes: { status: 'loading' } })
    try {
      set({ volumes: { status: 'ready', list: await listVolumes() } })
    } catch (error) {
      set({ volumes: { status: 'error', message: message(error) } })
    }
  },

  scan: async (target) => {
    const gen = ++generation
    const current = () => gen === generation
    cancelPending = false
    set({ history: null })
    set({ screen: { kind: 'scanning', target, scan: null, progress: null }, last: null, opened: [], basket: null })

    try {
      const scan = await startScan(target.path, false, (event) => {
        // Keyed by scan id: `last` keeps updating after going back to the volumes.
        if (live(event)) return
        if (!current()) return
        switch (event.event) {
          case 'progress': {
            const { screen } = get()
            if (screen.kind !== 'scanning') return
            const { dirs, files, allocated, elapsedMs } = event
            set({ screen: { ...screen, progress: { dirs, files, allocated, elapsedMs } } })
            return
          }
          case 'done': {
            const done = finished(target, event.summary)
            saveTarget(event.summary.scan, target)
            set({ screen: done, last: done })
            void loadTop(target, event.summary)
            return
          }
          case 'cancelled':
            set({ screen: { kind: 'cancelled', target } })
            return
          case 'failed':
            set({ screen: { kind: 'failed', target, message: event.message } })
            return
        }
      })
      if (!current()) {
        // Left the screen (or started another scan) before the id arrived.
        await cancelScan(scan).catch(() => undefined)
        return
      }
      const { screen } = get()
      if (screen.kind === 'scanning') {
        set({ screen: { ...screen, scan } })
        if (cancelPending) await get().cancel()
      }
    } catch (error) {
      if (current()) set({ screen: { kind: 'failed', target, message: message(error) } })
    }
  },

  reattach: async () => {
    const gen = generation
    let summary: ScanSummary | null
    try {
      summary = await attachScan(live)
    } catch {
      // Nothing to reopen; the volumes screen stays.
      return
    }
    // A scan started, or the user left the volumes, while the reply was on its way.
    if (!summary || gen !== generation || get().screen.kind !== 'volumes') return
    const target = savedTarget(summary)
    const done = finished(target, summary)
    set({ screen: done, last: done })
    void loadTop(target, summary)
  },

  cancel: async () => {
    const { screen } = get()
    if (screen.kind !== 'scanning') return
    if (screen.scan === null) {
      cancelPending = true
      return
    }
    // The `cancelled` event switches the screen; a stale id is harmless.
    await cancelScan(screen.scan).catch(() => undefined)
  },

  showVolumes: () => {
    const { screen } = get()
    if (screen.kind === 'scanning') void get().cancel()
    generation += 1
    set({ screen: { kind: 'volumes' } })
    void get().loadVolumes()
  },

  openLast: () => {
    const { last } = get()
    if (last) set({ screen: last })
  },

  browse: (node) => {
    const { screen, last } = get()
    // The döküm on show, or the one the table was opened from; `last` holds
    // its latest `top`, which may have loaded after the table opened.
    const done = screen.kind === 'done' ? screen : screen.kind === 'browse' ? screen.done : null
    if (!done) return
    const current = last?.summary.scan === done.summary.scan ? last : done
    set({ screen: { kind: 'browse', done: current, node }, history: record(get().history, done.summary.scan, node) })
  },

  go: async (step) => {
    for (;;) {
      const { screen, history } = get()
      if (screen.kind !== 'browse' || history?.scan !== screen.done.summary.scan) return false
      const at = history.at + step
      const entry = history.entries[at]
      if (!entry) return false
      let fresh: NodeEntry | undefined
      try {
        // Read again: its size may have changed, or it may be gone.
        fresh = (await getAncestors(history.scan, entry.id)).at(-1)
      } catch {
        fresh = undefined
      }
      const now = get()
      // Another move, a trip to the döküm or a new scan came first.
      if (now.history !== history || now.screen.kind !== 'browse') return false
      if (fresh) {
        const entries = history.entries.map((e, i) => (i === at ? fresh : e))
        set({ screen: { ...now.screen, node: fresh }, history: { ...history, entries, at } })
        return true
      }
      const entries = history.entries.filter((_, i) => i !== at)
      set({ history: { ...history, entries, at: at < history.at ? history.at - 1 : history.at } })
    }
  },

  showSummary: () => {
    const { screen, last } = get()
    if (screen.kind !== 'browse') return
    set({ screen: last?.summary.scan === screen.done.summary.scan ? last : screen.done })
  },

  toggleOpened: (id) =>
    set(({ opened }) => ({ opened: opened.includes(id) ? opened.filter((o) => o !== id) : [...opened, id] })),

  trashed: (scan, root) => {
    const { screen, last } = get()
    const done = screen.kind === 'done' ? screen : screen.kind === 'browse' ? screen.done : last
    if (done?.summary.scan !== scan) return
    const summary = { ...done.summary, root }
    const fresh: DoneScreen = { ...done, summary, top: { status: 'loading' } }
    set({
      screen: screen.kind === 'browse' && screen.done.summary.scan === scan ? { ...screen, done: fresh } : screen,
      last: last?.summary.scan === scan ? fresh : last,
    })
    void loadTop(done.target, summary)
  },

  addToBasket: (scan, item) => {
    const { basket } = get()
    const items = basket?.scan === scan ? basket.items : []
    const holder = items.find((b) => item.chain.includes(b.id))
    if (holder) return { kind: 'covered', by: holder.name }
    const inside = items.filter((b) => b.chain.includes(item.id)).length
    const kept = items.filter((b) => b.id !== item.id && !b.chain.includes(item.id))
    set({ basket: { scan, items: [...kept, item] } })
    return { kind: 'added', replaced: inside }
  },

  removeFromBasket: (id) => {
    const { basket } = get()
    if (!basket) return
    const items = basket.items.filter((b) => b.id !== id)
    set({ basket: items.length > 0 ? { ...basket, items } : null })
  },

  clearBasket: () => set({ basket: null }),

  refreshBasket: async (scan) => {
    const { basket } = get()
    if (basket?.scan !== scan) return
    const fresh = await Promise.all(
      basket.items.map(async (item): Promise<BasketItem | null> => {
        try {
          const chain = await getAncestors(scan, item.id)
          const node = chain.at(-1)
          return node ? { ...item, allocated: node.allocated, files: node.files } : null
        } catch {
          // Gone from the tree: moved to the trash, or under something that was.
          return null
        }
      }),
    )
    const now = get().basket
    // The basket may have changed while the sizes were loading; keep its items.
    if (now?.scan !== scan) return
    const byId = new Map(fresh.filter((f) => f !== null).map((f) => [f.id, f]))
    const items = now.items.flatMap((item) => {
      if (!basket.items.some((b) => b.id === item.id)) return [item]
      const f = byId.get(item.id)
      return f ? [f] : []
    })
    set({ basket: items.length > 0 ? { ...now, items } : null })
  },
}))

/**
 * `history` with `node` on show: the same folder is taken in place (a new
 * size), another one drops the forward entries and is added, as in a browser.
 */
function record(history: History | null, scan: ScanId, node: NodeEntry): History {
  const h = history?.scan === scan ? history : { scan, entries: [], at: -1 }
  if (h.entries[h.at]?.id === node.id) {
    return { ...h, entries: h.entries.map((e, i) => (i === h.at ? node : e)) }
  }
  return { scan, entries: [...h.entries.slice(0, h.at + 1), node], at: h.at + 1 }
}

/**
 * Applies a live update (`changed`, `outdated`) to the finished scan it
 * belongs to, on show or kept in `last`. Returns `false` for other events.
 */
function live(event: ScanEvent): boolean {
  if (event.event !== 'changed' && event.event !== 'outdated') return false
  const { screen, last } = useStore.getState()
  const shown = screen.kind === 'done' ? screen : screen.kind === 'browse' ? screen.done : null
  // `last` holds the latest `top`; the table's copy may be older.
  const done = last?.summary.scan === event.scan ? last : shown
  if (done?.summary.scan !== event.scan) return true
  const scan = event.scan
  const fresh: DoneScreen =
    event.event === 'changed'
      ? // `top` keeps showing until `loadTop` replaces it.
        { ...done, summary: { ...done.summary, root: event.root }, revision: event.revision }
      : { ...done, outdated: true }
  useStore.setState({
    screen:
      screen.kind === 'done' && screen.summary.scan === scan
        ? fresh
        : screen.kind === 'browse' && screen.done.summary.scan === scan
          ? { ...screen, done: fresh }
          : screen,
    last: last?.summary.scan === scan ? fresh : last,
  })
  if (event.event === 'changed') {
    void loadTop(done.target, fresh.summary)
    void useStore.getState().refreshBasket(scan)
  }
  return true
}

async function loadTop(target: Target, summary: ScanSummary) {
  // Keyed by the summary, not generation: the result also lands in `last` when
  // the user has already gone back to the volumes, and a reply for a tree that
  // has since changed again is dropped for the newer one.
  const set = (top: TopItems) =>
    useStore.setState(({ screen, last }) => ({
      screen: screen.kind === 'done' && screen.summary === summary ? { ...screen, top } : screen,
      last: last?.summary === summary ? { ...last, top } : last,
    }))
  try {
    const page = await getChildren(summary.scan, summary.root.id, 'allocated', 'desc', 0, TOP_ITEMS)
    set({ status: 'ready', items: page.items, total: page.total })
    const breakdown: VolumeBreakdown = {
      items: page.items.map(({ name, allocated }) => ({ name, allocated })),
      allocated: summary.root.allocated,
      at: Date.now(),
    }
    useStore.setState((s) => ({ breakdowns: { ...s.breakdowns, [target.path]: breakdown } }))
  } catch {
    set({ status: 'error' })
  }
}
