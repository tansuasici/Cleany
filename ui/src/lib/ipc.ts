import { Channel, invoke } from '@tauri-apps/api/core'

// Hand-written mirror of the serialized types in src-tauri/src/lib.rs,
// src-tauri/src/session.rs, src-tauri/src/volumes.rs, src-tauri/src/access.rs
// and src-tauri/src/actions.rs. The `ipc_ts_mentions_every_serialized_key`,
// `ipc_ts_mentions_every_volume_key`, `ipc_ts_mentions_every_access_tag` and
// `ipc_ts_mentions_every_action_tag` Rust tests fail when a key or tag here
// drifts from the backend.

export interface AppInfo {
  name: string
  version: string
  os: string
  arch: string
}

export const getAppInfo = () => invoke<AppInfo>('app_info')

/** A mounted volume; pass `mount` to `startScan` to scan it. */
export interface VolumeEntry {
  mount: string
  name: string
  /** Capacity in bytes. */
  total: number
  /** Bytes available to the current user. */
  available: number
  removable: boolean
}

/** Mounted volumes, sorted by mount point. */
export const listVolumes = () => invoke<VolumeEntry[]>('list_volumes')

/** Identifies one scan; a newer scan makes older ids stale. */
export type ScanId = number

export type NodeKind = 'file' | 'dir' | 'symlink' | 'other'

/** One node of a finished tree. `id` is passed back to address it. */
export interface NodeEntry {
  id: number
  name: string
  kind: NodeKind
  logical: number
  allocated: number
  files: number
  /** Seconds since the Unix epoch. */
  modified: number
  hasChildren: boolean
  hardlink: boolean
  placeholder: boolean
}

export interface ChildrenPage {
  total: number
  offset: number
  items: NodeEntry[]
}

/** One treemap box, in CSS pixels from the treemap's top left. */
export interface TreemapCell {
  /** Node index, or `null` for merged small children ("Diğer N öğe"). */
  id: number | null
  /** The folder this cell lies in. */
  parent: number
  /** 1 for the node's children, 2 for theirs, and so on. */
  depth: number
  x: number
  y: number
  w: number
  h: number
  allocated: number
  /** 0 for merged children. */
  files: number
  /** Empty for merged children. */
  name: string
  /** `null` for merged children. */
  kind: NodeKind | null
  hasChildren: boolean
  /** How many children were merged into this cell; 0 for a node. */
  others: number
}

/** A node's treemap; parents come before their children. */
export interface TreemapSlice {
  node: NodeEntry
  cells: TreemapCell[]
}

/** One slice of a sunburst ring; angles in turns (0–1), clockwise from 12 o'clock. */
export interface RingArc {
  /** Node index, or `null` for merged small children. */
  id: number | null
  /** The folder this slice lies in. */
  parent: number
  /** Ring: 1 for the node's children, 2 for theirs, and so on. */
  depth: number
  start: number
  end: number
  allocated: number
  /** 0 for merged children. */
  files: number
  /** Empty for merged children. */
  name: string
  /** `null` for merged children. */
  kind: NodeKind | null
  hasChildren: boolean
  /** How many children were merged into this slice; 0 for a node. */
  others: number
}

/** A node's sunburst; parents come before their children. */
export interface SunburstSlice {
  node: NodeEntry
  arcs: RingArc[]
}

export interface ScanStats {
  dirs: number
  files: number
  symlinks: number
  others: number
  skippedMounts: number
  hardlinks: number
  placeholders: number
  elapsedMs: number
}

export interface ScanIssue {
  path: string
  error: string
}

export interface ScanSummary {
  scan: ScanId
  root: NodeEntry
  stats: ScanStats
  /** The first 100 unreadable entries; `issueCount` has the total. */
  issues: ScanIssue[]
  issueCount: number
}

/** Streamed while a scan runs; `done`, `cancelled` or `failed` ends it. */
export type ScanEvent =
  | { event: 'progress'; dirs: number; files: number; allocated: number; elapsedMs: number }
  | { event: 'done'; summary: ScanSummary }
  | { event: 'cancelled' }
  | { event: 'failed'; message: string }

export type SortKey = 'allocated' | 'logical' | 'files' | 'name' | 'modified'
export type SortOrder = 'asc' | 'desc'

/** Rejection value of the scan commands. */
export type SessionError =
  | { kind: 'staleScan' }
  | { kind: 'notReady' }
  | { kind: 'unknownNode' }
  | { kind: 'invalidPath' }
  | { kind: 'spawn'; message: string }

/** Largest page `getChildren` returns. */
export const MAX_PAGE = 1000

/** Starts scanning `path` (absolute), cancelling any running scan. */
export function startScan(
  path: string,
  crossMounts: boolean,
  onEvent: (event: ScanEvent) => void,
): Promise<ScanId> {
  const channel = new Channel<ScanEvent>()
  channel.onmessage = onEvent
  return invoke<ScanId>('start_scan', { path, crossMounts, onEvent: channel })
}

export const cancelScan = (scan: ScanId) => invoke<void>('cancel_scan', { scan })

export const getChildren = (
  scan: ScanId,
  node: number,
  sort: SortKey,
  order: SortOrder,
  offset: number,
  limit: number,
) => invoke<ChildrenPage>('get_children', { scan, node, sort, order, offset, limit })

/** Most cells `getTreemap` returns. */
export const MAX_CELLS = 5000
/** Deepest treemap the backend lays out. */
export const MAX_TREEMAP_DEPTH = 3

/**
 * `node`'s treemap in a `width` × `height` CSS-pixel box, `depth` levels
 * deep (1–3). Children smaller than `minArea` px² merge into one cell.
 */
export const getTreemap = (scan: ScanId, node: number, width: number, height: number, depth = 2, minArea = 24) =>
  invoke<TreemapSlice>('get_treemap', { scan, node, width, height, depth, minArea })

/** Most rings the backend lays out. */
export const MAX_SUNBURST_DEPTH = 4

/**
 * `node`'s sunburst, `depth` rings deep (1–4). Children narrower than
 * `minAngle` turns merge into one slice per folder.
 */
export const getSunburst = (scan: ScanId, node: number, depth = 3, minAngle = 0.004) =>
  invoke<SunburstSlice>('get_sunburst', { scan, node, depth, minAngle })

export const getAncestors = (scan: ScanId, node: number) =>
  invoke<NodeEntry[]>('get_ancestors', { scan, node })

/** Rejection value of `revealNode`, `openNode` and `getPath`. */
export type ActionError =
  | { kind: 'staleScan' }
  | { kind: 'notReady' }
  | { kind: 'unknownNode' }
  | { kind: 'gone' }
  | { kind: 'launch'; message: string }

/** Shows the node selected in Finder / Explorer / the file manager. */
export const revealNode = (scan: ScanId, node: number) => invoke<void>('reveal_node', { scan, node })

/** Opens the node with its default app, as a double-click would. */
export const openNode = (scan: ScanId, node: number) => invoke<void>('open_node', { scan, node })

/** The node's full path, for the clipboard. */
export const getPath = (scan: ScanId, node: number) => invoke<string>('get_path', { scan, node })

/**
 * Whether macOS lets the app read protected folders (Mail, Safari, other
 * apps' data). A best-effort check: `unknown` when it cannot tell.
 */
export type DiskAccess = 'granted' | 'denied' | 'unknown' | 'notApplicable'

/** Rejection value of `openDiskAccessSettings`. */
export type AccessError = { kind: 'unsupported' } | { kind: 'launch'; message: string }

export const getDiskAccess = () => invoke<DiskAccess>('disk_access')

/** Opens System Settings at Privacy & Security → Full Disk Access. */
export const openDiskAccessSettings = () => invoke<void>('open_disk_access_settings')
