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
  /** After `done`: disk changes were patched into the tree. Untouched node ids stay valid. */
  | { event: 'changed'; scan: ScanId; revision: number; root: NodeEntry }
  /** After `done`: too much changed to patch; live updates of this scan stopped. */
  | { event: 'outdated'; scan: ScanId }

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

/**
 * The current scan's summary if it finished (e.g. after a webview reload), else `null`.
 * Live updates of that scan (`changed`, `outdated`) go to `onEvent` from then on.
 */
export function attachScan(onEvent: (event: ScanEvent) => void): Promise<ScanSummary | null> {
  const channel = new Channel<ScanEvent>()
  channel.onmessage = onEvent
  return invoke<ScanSummary | null>('attach_scan', { onEvent: channel })
}

export const getChildren = (
  scan: ScanId,
  node: number,
  sort: SortKey,
  order: SortOrder,
  offset: number,
  limit: number,
) => invoke<ChildrenPage>('get_children', { scan, node, sort, order, offset, limit })

/** What `getLargest` ranks. */
export type LargestKind = 'files' | 'dirs'

/** One row of `getLargest`. */
export interface LargestEntry {
  /** The node; for a folder, `allocated` is its whole subtree. */
  entry: NodeEntry
  /** Bytes it was ranked by: a file's own, a folder's direct files'. */
  own: number
  /** The folder holding it, relative to the scope and `/`-joined; empty for the scope and its direct children. */
  dir: string
}

/** The `limit` (at most `MAX_PAGE`) largest files or folders anywhere below `node`, largest first. */
export const getLargest = (scan: ScanId, node: number, kind: LargestKind, limit: number) =>
  invoke<LargestEntry[]>('get_largest', { scan, node, kind, limit })

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

/** Rejection value of `revealNode`, `openNode`, `openTerminal`, `getPath`, `trashNodes` and `deleteNodes`. */
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

/** Opens a terminal in the node if it is a folder, else in the folder holding it. */
export const openTerminal = (scan: ScanId, node: number) => invoke<void>('open_terminal', { scan, node })

/** The node's full path, for the clipboard. */
export const getPath = (scan: ScanId, node: number) => invoke<string>('get_path', { scan, node })

/** What happened to one item sent to the trash or deleted for good. */
export type TrashOutcome =
  | { kind: 'trashed' }
  /** Deleted from disk for good, without the trash. */
  | { kind: 'deleted' }
  | { kind: 'gone' }
  /** The scan root, a disk or home folder (or above it), a mount point, or a cloud placeholder. */
  | { kind: 'protected' }
  | { kind: 'failed'; message: string }

export interface TrashItem {
  node: number
  name: string
  outcome: TrashOutcome
}

export interface TrashReport {
  /** In request order; nodes under another requested node are left out. */
  items: TrashItem[]
  /** Bytes freed by the items this request removed from disk. */
  freed: { logical: number; allocated: number }
  /** Items this request removed from disk, to the trash or for good. */
  trashed: number
  /** The root with its new totals; `null` when a newer scan replaced this one. */
  root: NodeEntry | null
}

/**
 * Moves the nodes to the OS trash and takes them out of the scan tree.
 * Never asks: the UI confirms first. Per-item results are in the report.
 */
export const trashNodes = (scan: ScanId, nodes: number[]) => invoke<TrashReport>('trash_nodes', { scan, nodes })

/**
 * Deletes the nodes from disk for good, skipping the trash, and takes them out
 * of the scan tree. Never asks: the UI offers it only when Settings allow it
 * and asks for a typed confirmation. Same report as `trashNodes`.
 */
export const deleteNodes = (scan: ScanId, nodes: number[]) => invoke<TrashReport>('delete_nodes', { scan, nodes })

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

/** What an export writes: one item and everything below it, a folder's rows in a sort, or a largest list. */
export type ExportScope =
  | { kind: 'subtree'; node: number }
  | { kind: 'children'; node: number; sort: SortKey; order: SortOrder }
  | { kind: 'largest'; node: number; largest: LargestKind; limit: number }

export type ExportFormat = 'csv' | 'json'

/** A finished export: rows written, file size, and the name chosen in the save dialog. */
export interface ExportReport {
  rows: number
  bytes: number
  fileName: string
}

export type ExportError =
  | { kind: 'staleScan' }
  | { kind: 'notReady' }
  | { kind: 'unknownNode' }
  | { kind: 'io'; message: string }
  | { kind: 'dialog'; message: string }

/**
 * Asks where to save, then writes `scope` there as `format`. `null` when the
 * save dialog is closed without a choice.
 */
export const exportScan = (scan: ScanId, scope: ExportScope, format: ExportFormat) =>
  invoke<ExportReport | null>('export_scan', { scan, scope, format })
