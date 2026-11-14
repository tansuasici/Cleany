import { create } from 'zustand'
import {
  type DiskAccess,
  type NodeEntry,
  type ScanId,
  type ScanSummary,
  type VolumeEntry,
  cancelScan,
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
export type DoneScreen = { kind: 'done'; target: Target; summary: ScanSummary; top: TopItems }

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

/** How many items the döküm lists. */
export const TOP_ITEMS = 12

interface State {
  locale: Locale
  /** macOS Full Disk Access; `null` until the first check answers. */
  access: DiskAccess | null
  /** The user put the Full Disk Access panel aside ("Şimdilik geç"). */
  accessDismissed: boolean
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
  loadVolumes: () => Promise<void>
  scan: (target: Target) => Promise<void>
  cancel: () => Promise<void>
  showVolumes: () => void
  openLast: () => void
  /** Opens the tree table at `node`, from the döküm or the table itself. */
  browse: (node: NodeEntry) => void
  /** Leaves the tree table for its döküm. */
  showSummary: () => void
  /** Opens or closes a döküm folder in place. */
  toggleOpened: (id: number) => void
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
  volumes: { status: 'loading' },
  screen: { kind: 'volumes' },
  breakdowns: {},
  last: null,
  opened: [],

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
    set({ screen: { kind: 'scanning', target, scan: null, progress: null }, last: null, opened: [] })

    try {
      const scan = await startScan(target.path, false, (event) => {
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
            const done: DoneScreen = { kind: 'done', target, summary: event.summary, top: { status: 'loading' } }
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
    set({ screen: { kind: 'browse', done: current, node } })
  },

  showSummary: () => {
    const { screen, last } = get()
    if (screen.kind !== 'browse') return
    set({ screen: last?.summary.scan === screen.done.summary.scan ? last : screen.done })
  },

  toggleOpened: (id) =>
    set(({ opened }) => ({ opened: opened.includes(id) ? opened.filter((o) => o !== id) : [...opened, id] })),
}))

async function loadTop(target: Target, summary: ScanSummary) {
  // Keyed by scan id, not generation: the result also lands in `last` when the
  // user has already gone back to the volumes.
  const set = (top: TopItems) =>
    useStore.setState(({ screen, last }) => ({
      screen: screen.kind === 'done' && screen.summary.scan === summary.scan ? { ...screen, top } : screen,
      last: last?.summary.scan === summary.scan ? { ...last, top } : last,
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
