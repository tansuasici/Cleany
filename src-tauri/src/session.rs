//! Scan sessions: one scan at a time, its finished tree, and paged access to it.
//!
//! These are plain methods on [`Sessions`]. The `#[tauri::command]` wrappers
//! in `lib.rs` only forward to them, so everything here is testable without a
//! Tauri runtime. The UI never receives the whole tree (docs/PLAN.md §3.4):
//! it asks for one page of one directory's children at a time, addressing
//! nodes as `(ScanId, node index)` and never by path.
//!
//! Every serialized type here is mirrored by hand in `ui/src/lib/ipc.ts`;
//! the `ipc_ts_mentions_every_serialized_key` test catches drift.

use std::fmt;
use std::mem;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use cleany_core::{
    CancelToken, CellTarget, Node, NodeFlags, NodeId, NodeKind, Progress, ScanError, ScanOptions,
    ScanResult, SunburstOptions, Tree, TreemapOptions, scan, sunburst, treemap,
};
use serde::{Deserialize, Serialize};

/// Largest page [`Sessions::children`] returns, whatever `limit` asks for.
pub const MAX_PAGE: u32 = 1_000;

/// Most cells [`Sessions::treemap`] returns.
pub const MAX_CELLS: usize = 5_000;

/// Deepest treemap [`Sessions::treemap`] lays out, in levels below the node.
pub const MAX_TREEMAP_DEPTH: u8 = 3;

/// Largest treemap side, in CSS pixels; bigger requests are cut to this.
pub const MAX_TREEMAP_SIDE: f32 = 16_384.0;

/// Most rings [`Sessions::sunburst`] lays out, in levels below the node.
pub const MAX_SUNBURST_DEPTH: u8 = 4;

/// Largest `min_angle` [`Sessions::sunburst`] accepts, in turns.
pub const MAX_SUNBURST_MIN_ANGLE: f32 = 0.5;

/// How many unreadable entries a [`ScanSummary`] lists; the rest are counted.
pub const MAX_ISSUES: usize = 100;

/// Identifies one scan. A new scan gets a new id; ids are never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScanId(u64);

/// Column to sort children by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    /// Bytes on disk.
    Allocated,
    /// Logical length.
    Logical,
    /// Files below the node.
    Files,
    /// Name, case-insensitive.
    Name,
    /// Modification time.
    Modified,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortOrder {
    /// Smallest first.
    Asc,
    /// Largest first.
    Desc,
}

/// Entry kind as the UI sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    /// Regular file.
    File,
    /// Directory.
    Dir,
    /// Symlink, junction or reparse point (never followed).
    Symlink,
    /// Socket, FIFO, device.
    Other,
}

impl From<NodeKind> for EntryKind {
    fn from(kind: NodeKind) -> Self {
        match kind {
            NodeKind::File => Self::File,
            NodeKind::Dir => Self::Dir,
            NodeKind::Symlink => Self::Symlink,
            NodeKind::Other => Self::Other,
        }
    }
}

/// One node of a finished tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEntry {
    /// Node index; pass it back to address this node.
    pub id: u32,
    /// Display name (lossy for non-UTF-8 names). The root's is its full path.
    pub name: String,
    /// Entry kind.
    pub kind: EntryKind,
    /// Logical bytes, aggregated for directories.
    pub logical: u64,
    /// Allocated bytes, aggregated for directories.
    pub allocated: u64,
    /// Non-directory entries below (1 for a file).
    pub files: u32,
    /// Modification time, seconds since the Unix epoch.
    pub modified: i64,
    /// Whether the node has children to drill into.
    pub has_children: bool,
    /// A hard link whose bytes are counted on another node.
    pub hardlink: bool,
    /// A cloud placeholder whose data is not stored locally.
    pub placeholder: bool,
}

/// One page of a node's sorted children.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildrenPage {
    /// Number of children in total.
    pub total: u32,
    /// Position of the first item (clamped to `total`).
    pub offset: u32,
    /// At most [`MAX_PAGE`] children.
    pub items: Vec<NodeEntry>,
}

/// One box of a treemap, in CSS pixels from the treemap's top left.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapCell {
    /// Node index, or `None` for merged small children.
    pub id: Option<u32>,
    /// The folder this cell lies in.
    pub parent: u32,
    /// 1 for the node's children, 2 for theirs, and so on.
    pub depth: u8,
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
    /// Allocated bytes.
    pub allocated: u64,
    /// Non-directory entries below (1 for a file, 0 for merged children).
    pub files: u32,
    /// Display name; empty for merged children.
    pub name: String,
    /// Entry kind; `None` for merged children.
    pub kind: Option<EntryKind>,
    /// Whether the node has children to drill into.
    pub has_children: bool,
    /// How many children were merged into this cell (0 for a node).
    pub others: u32,
}

/// A node's treemap: the node itself and its cells, parents before children.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapSlice {
    /// The node laid out.
    pub node: NodeEntry,
    /// At most [`MAX_CELLS`] cells.
    pub cells: Vec<TreemapCell>,
}

/// One slice of a sunburst ring. Angles are in turns (0 to 1), clockwise
/// from twelve o'clock.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RingArc {
    /// Node index, or `None` for merged small children.
    pub id: Option<u32>,
    /// The folder this slice lies in.
    pub parent: u32,
    /// Ring: 1 for the node's children, 2 for theirs, and so on.
    pub depth: u8,
    /// Start angle.
    pub start: f32,
    /// End angle.
    pub end: f32,
    /// Allocated bytes.
    pub allocated: u64,
    /// Non-directory entries below (1 for a file, 0 for merged children).
    pub files: u32,
    /// Display name; empty for merged children.
    pub name: String,
    /// Entry kind; `None` for merged children.
    pub kind: Option<EntryKind>,
    /// Whether the node has children to drill into.
    pub has_children: bool,
    /// How many children were merged into this slice (0 for a node).
    pub others: u32,
}

/// A node's sunburst: the node itself and its slices, parents before children.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SunburstSlice {
    /// The node at the centre.
    pub node: NodeEntry,
    /// At most [`MAX_CELLS`] slices.
    pub arcs: Vec<RingArc>,
}

/// Counters of a finished scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    /// Directories, including the root.
    pub dirs: u64,
    /// Regular files.
    pub files: u64,
    /// Symlinks and reparse points (not followed).
    pub symlinks: u64,
    /// Sockets, FIFOs, devices.
    pub others: u64,
    /// Directories on other volumes that were not entered.
    pub skipped_mounts: u64,
    /// Extra hard links counted once.
    pub hardlinks: u64,
    /// Cloud placeholders not downloaded.
    pub placeholders: u64,
    /// Wall-clock duration.
    pub elapsed_ms: u64,
}

/// An entry that could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanIssue {
    /// Path that failed (lossy).
    pub path: String,
    /// The error message.
    pub error: String,
}

/// Result of a finished scan, sent once with [`ScanEvent::Done`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    /// The scan this describes.
    pub scan: ScanId,
    /// The root node.
    pub root: NodeEntry,
    /// Counters.
    pub stats: ScanStats,
    /// The first [`MAX_ISSUES`] unreadable entries.
    pub issues: Vec<ScanIssue>,
    /// Number of unreadable entries in total.
    pub issue_count: usize,
}

/// Events streamed to the UI while a scan runs. Exactly one of `Done`,
/// `Cancelled` or `Failed` ends the stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "event",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ScanEvent {
    /// Running totals, at most every `PROGRESS_INTERVAL` (10 Hz).
    Progress {
        /// Directories so far.
        dirs: u64,
        /// Non-directory entries so far.
        files: u64,
        /// Allocated bytes so far.
        allocated: u64,
        /// Time since the scan started.
        elapsed_ms: u64,
    },
    /// The scan finished; its tree can now be queried.
    Done {
        /// What was found.
        summary: ScanSummary,
    },
    /// The scan was cancelled or replaced by a newer one; it has no tree.
    Cancelled,
    /// The scan could not run (e.g. the root is unreadable).
    Failed {
        /// Why.
        message: String,
    },
}

impl ScanEvent {
    fn progress(p: &Progress) -> Self {
        Self::Progress {
            dirs: p.dirs,
            files: p.files,
            allocated: p.allocated,
            elapsed_ms: millis(p.elapsed),
        }
    }
}

/// Why a session request was refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionError {
    /// The id is not the current scan (a newer one replaced it).
    StaleScan,
    /// The current scan has no tree: it is still running, was cancelled or failed.
    NotReady,
    /// The node index is not in the tree.
    UnknownNode,
    /// The scan root must be an absolute path.
    InvalidPath,
    /// The scan thread could not be started.
    Spawn {
        /// The OS error.
        message: String,
    },
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleScan => f.write_str("scan was replaced by a newer one"),
            Self::NotReady => f.write_str("scan has no results"),
            Self::UnknownNode => f.write_str("unknown node"),
            Self::InvalidPath => f.write_str("scan path must be absolute"),
            Self::Spawn { message } => write!(f, "cannot start scan thread: {message}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// A finished tree plus the children order last asked for, so that paging
/// through one big directory sorts it once.
struct Finished {
    tree: Tree,
    sorted: Option<SortedChildren>,
}

struct SortedChildren {
    parent: NodeId,
    key: SortKey,
    order: SortOrder,
    ids: Vec<NodeId>,
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Scanning {
        id: ScanId,
        cancel: CancelToken,
    },
    Done {
        id: ScanId,
        scan: Box<Finished>,
    },
    Cancelled {
        id: ScanId,
    },
    Failed {
        id: ScanId,
    },
}

impl State {
    const fn id(&self) -> Option<ScanId> {
        match self {
            Self::Idle => None,
            Self::Scanning { id, .. }
            | Self::Done { id, .. }
            | Self::Cancelled { id }
            | Self::Failed { id } => Some(*id),
        }
    }

    fn finished_mut(&mut self, want: ScanId) -> Result<&mut Finished, SessionError> {
        if self.id() != Some(want) {
            return Err(SessionError::StaleScan);
        }
        match self {
            Self::Done { scan, .. } => Ok(scan),
            _ => Err(SessionError::NotReady),
        }
    }
}

/// The app's scan session: at most one scan runs, and only the latest
/// scan's tree is kept. Starting a new scan cancels the running one and
/// drops the previous tree.
#[derive(Default)]
pub struct Sessions {
    state: Arc<Mutex<State>>,
    next_id: AtomicU64,
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    // A panic while holding the lock leaves `State` consistent: every write
    // is a single assignment.
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Sessions {
    /// Starts scanning `root` on a background thread and returns its id.
    /// `emit` receives [`ScanEvent`]s on that thread; the last one is
    /// `Done`, `Cancelled` or `Failed`, sent after the session state is
    /// updated (so the tree is queryable once `Done` arrives).
    ///
    /// # Errors
    /// [`SessionError::InvalidPath`] for a relative path,
    /// [`SessionError::Spawn`] if the thread cannot start.
    pub fn start<F>(
        &self,
        root: PathBuf,
        options: ScanOptions,
        emit: F,
    ) -> Result<ScanId, SessionError>
    where
        F: FnMut(ScanEvent) + Send + 'static,
    {
        self.start_with(root, options, CancelToken::new(), emit)
    }

    fn start_with<F>(
        &self,
        root: PathBuf,
        options: ScanOptions,
        cancel: CancelToken,
        mut emit: F,
    ) -> Result<ScanId, SessionError>
    where
        F: FnMut(ScanEvent) + Send + 'static,
    {
        if !root.is_absolute() {
            return Err(SessionError::InvalidPath);
        }
        let id = ScanId(self.next_id.fetch_add(1, Ordering::Relaxed) + 1);
        let previous = {
            let mut state = lock(&self.state);
            mem::replace(
                &mut *state,
                State::Scanning {
                    id,
                    cancel: cancel.clone(),
                },
            )
        };
        if let State::Scanning { cancel: old, .. } = &previous {
            old.cancel();
        }
        // A previous tree can hold millions of nodes; free it outside the lock.
        drop(previous);

        let shared = Arc::clone(&self.state);
        let spawned = thread::Builder::new()
            .name(format!("cleany-scan-{}", id.0))
            .spawn(move || {
                let result = scan(&root, &options, &cancel, |p| {
                    emit(ScanEvent::progress(p));
                });
                let last = finish(&shared, id, result);
                emit(last);
            });
        if let Err(e) = spawned {
            let mut state = lock(&self.state);
            if state.id() == Some(id) {
                *state = State::Failed { id };
            }
            return Err(SessionError::Spawn {
                message: e.to_string(),
            });
        }
        Ok(id)
    }

    /// Asks scan `id` to stop. Idempotent: cancelling a scan that already
    /// ended is a no-op.
    ///
    /// # Errors
    /// [`SessionError::StaleScan`] if `id` is not the current scan.
    pub fn cancel(&self, id: ScanId) -> Result<(), SessionError> {
        let state = lock(&self.state);
        match &*state {
            State::Scanning { id: cur, cancel } if *cur == id => {
                cancel.cancel();
                Ok(())
            }
            s if s.id() == Some(id) => Ok(()),
            _ => Err(SessionError::StaleScan),
        }
    }

    /// One page of `node`'s children, sorted by `key` in `order`. At most
    /// [`MAX_PAGE`] items are returned; an `offset` past the end yields an
    /// empty page.
    ///
    /// # Errors
    /// [`SessionError::StaleScan`], [`SessionError::NotReady`] or
    /// [`SessionError::UnknownNode`].
    pub fn children(
        &self,
        id: ScanId,
        node: u32,
        key: SortKey,
        order: SortOrder,
        offset: u32,
        limit: u32,
    ) -> Result<ChildrenPage, SessionError> {
        let mut state = lock(&self.state);
        let Finished { tree, sorted } = state.finished_mut(id)?;
        let parent = tree.node_id(node).ok_or(SessionError::UnknownNode)?;
        let ids = sorted_children(tree, sorted, parent, key, order);

        let total = ids.len();
        let start = (offset as usize).min(total);
        let end = start
            .saturating_add(limit.min(MAX_PAGE) as usize)
            .min(total);
        let items = ids[start..end]
            .iter()
            .filter_map(|&c| entry(tree, c))
            .collect();
        Ok(ChildrenPage {
            total: u32::try_from(total).unwrap_or(u32::MAX),
            offset: u32::try_from(start).unwrap_or(u32::MAX),
            items,
        })
    }

    /// `node`'s treemap in a `width` × `height` box (CSS pixels), `depth`
    /// levels deep (1 to [`MAX_TREEMAP_DEPTH`]). Children smaller than
    /// `min_area` px² (at least 1) are merged into one cell per folder. A box
    /// with no area gives no cells; sides past [`MAX_TREEMAP_SIDE`] are cut.
    ///
    /// # Errors
    /// [`SessionError::StaleScan`], [`SessionError::NotReady`] or
    /// [`SessionError::UnknownNode`].
    pub fn treemap(
        &self,
        id: ScanId,
        node: u32,
        width: f32,
        height: f32,
        depth: u8,
        min_area: f32,
    ) -> Result<TreemapSlice, SessionError> {
        let mut state = lock(&self.state);
        let tree = &state.finished_mut(id)?.tree;
        let root = tree.node_id(node).ok_or(SessionError::UnknownNode)?;
        let node_entry = entry(tree, root).ok_or(SessionError::UnknownNode)?;
        let side = |v: f32| {
            if v.is_finite() && v > 0.0 {
                v.min(MAX_TREEMAP_SIDE)
            } else {
                0.0
            }
        };
        let opts = TreemapOptions {
            width: side(width),
            height: side(height),
            depth: depth.clamp(1, MAX_TREEMAP_DEPTH),
            min_area: min_area.max(1.0),
            max_cells: MAX_CELLS,
            padding: 1.0,
        };
        let cells = treemap(tree, root, &opts)
            .into_iter()
            .filter_map(|c| {
                let parent = u32::try_from(c.parent.index()).ok()?;
                let base = TreemapCell {
                    id: None,
                    parent,
                    depth: c.depth,
                    x: c.rect.x,
                    y: c.rect.y,
                    w: c.rect.w,
                    h: c.rect.h,
                    allocated: c.allocated,
                    files: 0,
                    name: String::new(),
                    kind: None,
                    has_children: false,
                    others: 0,
                };
                Some(match c.target {
                    CellTarget::Node(n) => {
                        let e = entry(tree, n)?;
                        TreemapCell {
                            id: Some(e.id),
                            files: e.files,
                            name: e.name,
                            kind: Some(e.kind),
                            has_children: e.has_children,
                            ..base
                        }
                    }
                    CellTarget::Others { count, .. } => TreemapCell {
                        others: count,
                        ..base
                    },
                })
            })
            .collect();
        Ok(TreemapSlice {
            node: node_entry,
            cells,
        })
    }

    /// `node`'s sunburst, `depth` rings deep (1 to [`MAX_SUNBURST_DEPTH`]).
    /// Children narrower than `min_angle` turns (0 to
    /// [`MAX_SUNBURST_MIN_ANGLE`]; anything else counts as 0) are merged into
    /// one slice per folder.
    ///
    /// # Errors
    /// [`SessionError::StaleScan`], [`SessionError::NotReady`] or
    /// [`SessionError::UnknownNode`].
    pub fn sunburst(
        &self,
        id: ScanId,
        node: u32,
        depth: u8,
        min_angle: f32,
    ) -> Result<SunburstSlice, SessionError> {
        let mut state = lock(&self.state);
        let tree = &state.finished_mut(id)?.tree;
        let root = tree.node_id(node).ok_or(SessionError::UnknownNode)?;
        let node_entry = entry(tree, root).ok_or(SessionError::UnknownNode)?;
        let min_angle = if min_angle.is_finite() {
            min_angle.clamp(0.0, MAX_SUNBURST_MIN_ANGLE)
        } else {
            0.0
        };
        let opts = SunburstOptions {
            depth: depth.clamp(1, MAX_SUNBURST_DEPTH),
            min_angle,
            max_arcs: MAX_CELLS,
        };
        let arcs = sunburst(tree, root, &opts)
            .into_iter()
            .filter_map(|a| {
                let parent = u32::try_from(a.parent.index()).ok()?;
                let base = RingArc {
                    id: None,
                    parent,
                    depth: a.depth,
                    start: a.start,
                    end: a.end,
                    allocated: a.allocated,
                    files: 0,
                    name: String::new(),
                    kind: None,
                    has_children: false,
                    others: 0,
                };
                Some(match a.target {
                    CellTarget::Node(n) => {
                        let e = entry(tree, n)?;
                        RingArc {
                            id: Some(e.id),
                            files: e.files,
                            name: e.name,
                            kind: Some(e.kind),
                            has_children: e.has_children,
                            ..base
                        }
                    }
                    CellTarget::Others { count, .. } => RingArc {
                        others: count,
                        ..base
                    },
                })
            })
            .collect();
        Ok(SunburstSlice {
            node: node_entry,
            arcs,
        })
    }

    /// `node` and its ancestors, root first, for the breadcrumb.
    ///
    /// # Errors
    /// [`SessionError::StaleScan`], [`SessionError::NotReady`] or
    /// [`SessionError::UnknownNode`].
    pub fn ancestors(&self, id: ScanId, node: u32) -> Result<Vec<NodeEntry>, SessionError> {
        let mut state = lock(&self.state);
        let tree = &state.finished_mut(id)?.tree;
        let mut cur = Some(tree.node_id(node).ok_or(SessionError::UnknownNode)?);
        let mut out = Vec::new();
        while let Some(c) = cur {
            out.extend(entry(tree, c));
            cur = tree.node(c).and_then(Node::parent);
        }
        out.reverse();
        Ok(out)
    }
}

/// Records the outcome of scan `id` and returns the event that ends its
/// stream. A scan that was replaced meanwhile leaves the state alone.
fn finish(state: &Mutex<State>, id: ScanId, result: Result<ScanResult, ScanError>) -> ScanEvent {
    let mut state = lock(state);
    let current = matches!(&*state, State::Scanning { id: cur, .. } if *cur == id);
    if !current {
        return ScanEvent::Cancelled;
    }
    match result {
        Ok(r) => match summary(id, &r) {
            Some(summary) => {
                *state = State::Done {
                    id,
                    scan: Box::new(Finished {
                        tree: r.tree,
                        sorted: None,
                    }),
                };
                ScanEvent::Done { summary }
            }
            None => {
                *state = State::Failed { id };
                ScanEvent::Failed {
                    message: "scan produced no root".to_owned(),
                }
            }
        },
        Err(ScanError::Cancelled) => {
            *state = State::Cancelled { id };
            ScanEvent::Cancelled
        }
        Err(e) => {
            *state = State::Failed { id };
            ScanEvent::Failed {
                message: e.to_string(),
            }
        }
    }
}

fn summary(id: ScanId, r: &ScanResult) -> Option<ScanSummary> {
    let s = &r.stats;
    Some(ScanSummary {
        scan: id,
        root: entry(&r.tree, r.tree.root())?,
        stats: ScanStats {
            dirs: s.dirs,
            files: s.files,
            symlinks: s.symlinks,
            others: s.others,
            skipped_mounts: s.skipped_mounts,
            hardlinks: s.hardlinks,
            placeholders: s.placeholders,
            elapsed_ms: millis(s.elapsed),
        },
        issues: r
            .issues
            .iter()
            .take(MAX_ISSUES)
            .map(|i| ScanIssue {
                path: i.path.to_string_lossy().into_owned(),
                error: i.error.to_string(),
            })
            .collect(),
        issue_count: r.issues.len(),
    })
}

fn entry(tree: &Tree, id: NodeId) -> Option<NodeEntry> {
    let n = tree.node(id)?;
    Some(NodeEntry {
        id: u32::try_from(id.index()).ok()?,
        name: tree.name(id)?.to_string_lossy().into_owned(),
        kind: n.kind().into(),
        logical: n.size().logical,
        allocated: n.size().allocated,
        files: n.file_count(),
        modified: n.mtime(),
        has_children: tree.children(id).next().is_some(),
        hardlink: n.flags().contains(NodeFlags::HARDLINK),
        placeholder: n.flags().contains(NodeFlags::PLACEHOLDER),
    })
}

/// `parent`'s children in the requested order, reusing the cached order when
/// the same directory is paged again.
fn sorted_children<'a>(
    tree: &Tree,
    cache: &'a mut Option<SortedChildren>,
    parent: NodeId,
    key: SortKey,
    order: SortOrder,
) -> &'a [NodeId] {
    let hit = cache
        .as_ref()
        .is_some_and(|c| c.parent == parent && c.key == key && c.order == order);
    if !hit {
        *cache = Some(SortedChildren {
            parent,
            key,
            order,
            ids: sort(tree, parent, key, order),
        });
    }
    cache.as_ref().map_or(&[], |c| c.ids.as_slice())
}

fn sort(tree: &Tree, parent: NodeId, key: SortKey, order: SortOrder) -> Vec<NodeId> {
    let mut ids: Vec<NodeId> = tree.children(parent).collect();
    let node = |id: NodeId| tree.node(id);
    // Ties fall back to the node id, so the order is deterministic.
    match key {
        SortKey::Allocated => {
            ids.sort_unstable_by_key(|&id| (node(id).map_or(0, |n| n.size().allocated), id));
        }
        SortKey::Logical => {
            ids.sort_unstable_by_key(|&id| (node(id).map_or(0, |n| n.size().logical), id));
        }
        SortKey::Files => {
            ids.sort_unstable_by_key(|&id| (node(id).map_or(0, Node::file_count), id))
        }
        SortKey::Modified => ids.sort_unstable_by_key(|&id| (node(id).map_or(0, Node::mtime), id)),
        SortKey::Name => ids.sort_by_cached_key(|&id| {
            let name = tree.name(id).unwrap_or_default();
            (name.to_string_lossy().to_lowercase(), id)
        }),
    }
    if order == SortOrder::Desc {
        ids.reverse();
    }
    ids
}

fn millis(d: std::time::Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        CancelToken, ChildrenPage, EntryKind, Finished, MAX_SUNBURST_DEPTH, MAX_TREEMAP_DEPTH,
        NodeEntry, RingArc, ScanEvent, ScanId, ScanIssue, ScanStats, ScanSummary, SessionError,
        Sessions, SortKey, SortOrder, State, SunburstSlice, TreemapCell, TreemapSlice, lock,
    };
    use cleany_core::{EntryInfo, NodeFlags, NodeKind, ScanOptions, Size, Tree, TreeBuilder};
    use std::ffi::OsStr;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::Duration;

    const WAIT: Duration = Duration::from_secs(30);

    fn info(kind: NodeKind, allocated: u64, mtime: i64) -> EntryInfo {
        EntryInfo {
            kind,
            size: Size {
                logical: allocated / 2,
                allocated,
            },
            mtime,
            flags: NodeFlags::NONE,
        }
    }

    /// `/r` holding `b` (300), `a` (100), `C/` (dir with `x` 200) — by name:
    /// a, b, C.
    fn small_tree() -> Tree {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 0)).unwrap();
        let root = b.root();
        b.add_child(root, OsStr::new("b"), info(NodeKind::File, 300, 3))
            .unwrap();
        b.add_child(root, OsStr::new("a"), info(NodeKind::File, 100, 1))
            .unwrap();
        let c = b
            .add_child(root, OsStr::new("C"), info(NodeKind::Dir, 0, 2))
            .unwrap();
        b.add_child(c, OsStr::new("x"), info(NodeKind::File, 200, 0))
            .unwrap();
        b.finish()
    }

    /// A session whose current scan is already finished with `tree`.
    fn done(tree: Tree) -> (Sessions, ScanId) {
        let s = Sessions::default();
        let id = ScanId(7);
        *lock(&s.state) = State::Done {
            id,
            scan: Box::new(Finished { tree, sorted: None }),
        };
        (s, id)
    }

    fn names(page: &ChildrenPage) -> Vec<&str> {
        page.items.iter().map(|e| e.name.as_str()).collect()
    }

    /// A real directory to scan: this crate's sources.
    fn src_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    /// Starts a scan and returns its id plus a receiver for its events.
    fn start(s: &Sessions, cancel: CancelToken) -> (ScanId, mpsc::Receiver<ScanEvent>) {
        let (tx, rx) = mpsc::channel();
        let id = s
            .start_with(src_dir(), ScanOptions::default(), cancel, move |e| {
                let _ = tx.send(e);
            })
            .unwrap();
        (id, rx)
    }

    /// Waits for the event that ends a scan's stream.
    fn last_event(rx: &mpsc::Receiver<ScanEvent>) -> ScanEvent {
        loop {
            match rx.recv_timeout(WAIT).unwrap() {
                ScanEvent::Progress { .. } => {}
                end => return end,
            }
        }
    }

    #[test]
    fn children_page_sorted_by_allocated_desc() {
        let (s, id) = done(small_tree());
        let page = s
            .children(id, 0, SortKey::Allocated, SortOrder::Desc, 0, 10)
            .unwrap();
        assert_eq!(page.total, 3);
        assert_eq!(names(&page), ["b", "C", "a"]);
        let c = &page.items[1];
        assert_eq!(c.kind, EntryKind::Dir);
        assert_eq!(c.allocated, 200, "directory size is aggregated");
        assert!(c.has_children);
        assert!(!page.items[0].has_children);

        let by_name = s
            .children(id, 0, SortKey::Name, SortOrder::Asc, 0, 10)
            .unwrap();
        assert_eq!(names(&by_name), ["a", "b", "C"], "case-insensitive");
        let by_time = s
            .children(id, 0, SortKey::Modified, SortOrder::Asc, 0, 10)
            .unwrap();
        assert_eq!(names(&by_time), ["a", "C", "b"]);
    }

    #[test]
    fn children_pagination_offset_limit() {
        let (s, id) = done(small_tree());
        let page = |offset, limit| {
            s.children(id, 0, SortKey::Name, SortOrder::Asc, offset, limit)
                .unwrap()
        };
        assert_eq!(names(&page(0, 2)), ["a", "b"]);
        let last = page(2, 2);
        assert_eq!(names(&last), ["C"]);
        assert_eq!((last.total, last.offset), (3, 2));
        let past = page(99, 2);
        assert!(past.items.is_empty());
        assert_eq!(past.offset, 3, "offset is clamped to total");
        assert!(page(0, 0).items.is_empty());
        // A file has no children.
        let a = page(0, 1).items[0].id;
        let none = s
            .children(id, a, SortKey::Name, SortOrder::Asc, 0, 10)
            .unwrap();
        assert_eq!(none.total, 0);
    }

    #[test]
    fn stale_scan_id_is_rejected() {
        let (s, id) = done(small_tree());
        let other = ScanId(id.0 + 1);
        let err = s.children(other, 0, SortKey::Name, SortOrder::Asc, 0, 10);
        assert_eq!(err, Err(SessionError::StaleScan));
        assert_eq!(s.ancestors(other, 0), Err(SessionError::StaleScan));
        assert_eq!(s.cancel(other), Err(SessionError::StaleScan));
        assert_eq!(
            s.cancel(id),
            Ok(()),
            "cancelling a finished scan is a no-op"
        );
    }

    #[test]
    fn unknown_node_is_rejected() {
        let (s, id) = done(small_tree());
        let err = s.children(id, 99, SortKey::Name, SortOrder::Asc, 0, 10);
        assert_eq!(err, Err(SessionError::UnknownNode));
        assert_eq!(s.ancestors(id, u32::MAX), Err(SessionError::UnknownNode));
    }

    #[test]
    fn ancestors_are_root_first() {
        let (s, id) = done(small_tree());
        let c = s
            .children(id, 0, SortKey::Name, SortOrder::Asc, 2, 1)
            .unwrap()
            .items[0]
            .id;
        let x = s
            .children(id, c, SortKey::Name, SortOrder::Asc, 0, 1)
            .unwrap()
            .items[0]
            .id;
        let chain = s.ancestors(id, x).unwrap();
        let chain: Vec<_> = chain.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(chain, ["/r", "C", "x"]);
        assert_eq!(s.ancestors(id, 0).unwrap().len(), 1);
    }

    #[test]
    fn sort_100k_children_is_cached_across_pages() {
        const N: u64 = 100_000;
        let mut b = TreeBuilder::new(OsStr::new("/big"), info(NodeKind::Dir, 0, 0)).unwrap();
        let root = b.root();
        for i in 0..N {
            let name = format!("f{i:06}");
            b.add_child(root, OsStr::new(&name), info(NodeKind::File, i * 7 % N, 0))
                .unwrap();
        }
        let (s, id) = done(b.finish());

        let first = s
            .children(id, 0, SortKey::Allocated, SortOrder::Desc, 0, 5_000)
            .unwrap();
        assert_eq!(first.total, 100_000);
        assert_eq!(first.items.len(), 1_000, "limit is capped at MAX_PAGE");
        let second = s
            .children(id, 0, SortKey::Allocated, SortOrder::Desc, 1_000, 1_000)
            .unwrap();
        let sizes: Vec<u64> = first
            .items
            .iter()
            .chain(&second.items)
            .map(|e| e.allocated)
            .collect();
        assert!(
            sizes.windows(2).all(|w| w[0] >= w[1]),
            "pages continue the order"
        );

        let state = lock(&s.state);
        let State::Done { scan, .. } = &*state else {
            panic!("expected a finished scan");
        };
        let cached = scan.sorted.as_ref().unwrap();
        assert_eq!(cached.ids.len(), 100_000);
        assert_eq!(
            (cached.key, cached.order),
            (SortKey::Allocated, SortOrder::Desc)
        );
    }

    #[test]
    fn scan_runs_and_tree_is_queryable_on_done() {
        let s = Sessions::default();
        let (id, rx) = start(&s, CancelToken::new());
        let ScanEvent::Done { summary } = last_event(&rx) else {
            panic!("expected Done");
        };
        assert_eq!(summary.scan, id);
        assert!(summary.root.has_children);
        assert!(summary.stats.files >= 2, "lib.rs and session.rs");
        let page = s
            .children(id, summary.root.id, SortKey::Name, SortOrder::Asc, 0, 100)
            .unwrap();
        assert!(names(&page).contains(&"session.rs"));
    }

    #[test]
    fn cancel_ends_scan_without_a_tree() {
        let s = Sessions::default();
        let cancel = CancelToken::new();
        cancel.cancel();
        let (id, rx) = start(&s, cancel);
        assert_eq!(last_event(&rx), ScanEvent::Cancelled);
        let err = s.children(id, 0, SortKey::Name, SortOrder::Asc, 0, 10);
        assert_eq!(err, Err(SessionError::NotReady));
        assert_eq!(s.cancel(id), Ok(()));
    }

    #[test]
    fn cancel_signals_the_running_scan() {
        let s = Sessions::default();
        let id = ScanId(3);
        let token = CancelToken::new();
        *lock(&s.state) = State::Scanning {
            id,
            cancel: token.clone(),
        };
        let err = s.children(id, 0, SortKey::Name, SortOrder::Asc, 0, 10);
        assert_eq!(err, Err(SessionError::NotReady));
        assert_eq!(s.cancel(id), Ok(()));
        assert!(token.is_cancelled());
        assert_eq!(s.cancel(ScanId(4)), Err(SessionError::StaleScan));
    }

    #[test]
    fn new_start_cancels_running_scan() {
        let s = Sessions::default();
        let first_token = CancelToken::new();
        let (first, first_rx) = start(&s, first_token.clone());
        let (second, second_rx) = start(&s, CancelToken::new());
        assert!(
            first_token.is_cancelled(),
            "the running scan is told to stop"
        );
        assert_ne!(first, second);

        // A tiny tree may finish before the second start (`Done`); otherwise
        // it ends `Cancelled`. Either way it never takes over the session.
        assert!(matches!(
            last_event(&first_rx),
            ScanEvent::Cancelled | ScanEvent::Done { .. }
        ));
        assert!(matches!(last_event(&second_rx), ScanEvent::Done { .. }));
        let err = s.children(first, 0, SortKey::Name, SortOrder::Asc, 0, 10);
        assert_eq!(err, Err(SessionError::StaleScan));
        assert!(
            s.children(second, 0, SortKey::Name, SortOrder::Asc, 0, 10)
                .is_ok()
        );
    }

    #[test]
    fn missing_root_fails_and_relative_path_is_rejected() {
        let s = Sessions::default();
        let (tx, rx) = mpsc::channel();
        let missing = src_dir().join("does-not-exist");
        let id = s
            .start(missing, ScanOptions::default(), move |e| {
                let _ = tx.send(e);
            })
            .unwrap();
        assert!(matches!(last_event(&rx), ScanEvent::Failed { .. }));
        let err = s.children(id, 0, SortKey::Name, SortOrder::Asc, 0, 10);
        assert_eq!(err, Err(SessionError::NotReady));

        let rel = s.start("relative".into(), ScanOptions::default(), |_| {});
        assert_eq!(rel, Err(SessionError::InvalidPath));
    }

    #[test]
    fn get_treemap_rejects_stale_scan_and_unknown_node() {
        let (s, id) = done(small_tree());
        assert_eq!(
            s.treemap(ScanId(1), 0, 100.0, 100.0, 2, 24.0),
            Err(SessionError::StaleScan)
        );
        assert_eq!(
            s.treemap(id, 99, 100.0, 100.0, 2, 24.0),
            Err(SessionError::UnknownNode)
        );

        let slice = s.treemap(id, 0, 100.0, 100.0, 9, 0.0).unwrap();
        assert_eq!(slice.node.id, 0);
        // b, C, a at level 1 and C's child x at level 2.
        assert_eq!(slice.cells.len(), 4);
        assert!(slice.cells.iter().all(|c| c.depth <= MAX_TREEMAP_DEPTH));
        let x = slice.cells.iter().find(|c| c.name == "x").unwrap();
        assert_eq!((x.depth, x.files, x.kind), (2, 1, Some(EntryKind::File)));
        let c = slice.cells.iter().find(|c| c.name == "C").unwrap();
        assert_eq!(x.parent, c.id.unwrap());
        assert!(c.has_children);

        // No area: no cells, but the node is still described.
        for (w, h) in [(0.0, 100.0), (f32::NAN, 100.0), (100.0, -1.0)] {
            assert!(s.treemap(id, 0, w, h, 2, 24.0).unwrap().cells.is_empty());
        }
        // Huge boxes are cut to the side cap.
        let big = s.treemap(id, 0, 1e9, 1e9, 1, 24.0).unwrap();
        let right = big.cells.iter().map(|c| c.x + c.w).fold(0.0, f32::max);
        assert!(right <= super::MAX_TREEMAP_SIDE + 1.0);
    }

    #[test]
    fn get_sunburst_rejects_stale_scan_and_unknown_node() {
        let (s, id) = done(small_tree());
        assert_eq!(
            s.sunburst(ScanId(1), 0, 2, 0.004),
            Err(SessionError::StaleScan)
        );
        assert_eq!(s.sunburst(id, 99, 2, 0.004), Err(SessionError::UnknownNode));

        let slice = s.sunburst(id, 0, 9, 0.0).unwrap();
        assert_eq!(slice.node.id, 0);
        // b, C, a in ring 1 and C's child x in ring 2.
        assert_eq!(slice.arcs.len(), 4);
        assert!(slice.arcs.iter().all(|a| a.depth <= MAX_SUNBURST_DEPTH));
        let x = slice.arcs.iter().find(|a| a.name == "x").unwrap();
        assert_eq!((x.depth, x.files, x.kind), (2, 1, Some(EntryKind::File)));
        let c = slice.arcs.iter().find(|a| a.name == "C").unwrap();
        assert_eq!(x.parent, c.id.unwrap());
        assert!(c.has_children);
        // x fills all of C's span.
        assert_eq!((x.start, x.end), (c.start, c.end));

        // Depth 0 still lays out one ring.
        let one = s.sunburst(id, 0, 0, 0.0).unwrap();
        assert!(one.arcs.iter().all(|a| a.depth == 1));
        assert_eq!(one.arcs.len(), 3);
        // A bad or huge angle is clamped, never panics; the first ring always
        // covers the full turn.
        for m in [f32::NAN, f32::INFINITY, -1.0, 9.0] {
            let arcs = s.sunburst(id, 0, 1, m).unwrap().arcs;
            assert!(!arcs.is_empty());
            assert_eq!(arcs.last().unwrap().end, 1.0);
        }
    }

    fn sample_entry() -> NodeEntry {
        NodeEntry {
            id: 3,
            name: "C".to_owned(),
            kind: EntryKind::Dir,
            logical: 1,
            allocated: 2,
            files: 4,
            modified: 5,
            has_children: true,
            hardlink: false,
            placeholder: false,
        }
    }

    fn samples() -> Vec<serde_json::Value> {
        let summary = ScanSummary {
            scan: ScanId(1),
            root: sample_entry(),
            stats: ScanStats {
                dirs: 1,
                files: 2,
                symlinks: 3,
                others: 4,
                skipped_mounts: 5,
                hardlinks: 6,
                placeholders: 7,
                elapsed_ms: 8,
            },
            issues: vec![ScanIssue {
                path: "/p".to_owned(),
                error: "denied".to_owned(),
            }],
            issue_count: 1,
        };
        let page = ChildrenPage {
            total: 1,
            offset: 0,
            items: vec![sample_entry()],
        };
        let events = [
            ScanEvent::Progress {
                dirs: 1,
                files: 2,
                allocated: 3,
                elapsed_ms: 4,
            },
            ScanEvent::Done { summary },
            ScanEvent::Cancelled,
            ScanEvent::Failed {
                message: "m".to_owned(),
            },
        ];
        let errors = [
            SessionError::StaleScan,
            SessionError::NotReady,
            SessionError::UnknownNode,
            SessionError::InvalidPath,
            SessionError::Spawn {
                message: "m".to_owned(),
            },
        ];
        let mut out: Vec<_> = events
            .iter()
            .map(|e| serde_json::to_value(e).unwrap())
            .collect();
        out.extend(errors.iter().map(|e| serde_json::to_value(e).unwrap()));
        out.push(serde_json::to_value(page).unwrap());
        let cell = TreemapCell {
            id: Some(3),
            parent: 0,
            depth: 1,
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            allocated: 2,
            files: 4,
            name: "C".to_owned(),
            kind: Some(EntryKind::Dir),
            has_children: true,
            others: 0,
        };
        let others = TreemapCell {
            id: None,
            kind: None,
            others: 5,
            ..cell.clone()
        };
        out.push(
            serde_json::to_value(TreemapSlice {
                node: sample_entry(),
                cells: vec![cell, others],
            })
            .unwrap(),
        );
        let arc = RingArc {
            id: Some(3),
            parent: 0,
            depth: 1,
            start: 0.0,
            end: 0.5,
            allocated: 2,
            files: 4,
            name: "C".to_owned(),
            kind: Some(EntryKind::Dir),
            has_children: true,
            others: 0,
        };
        let rest = RingArc {
            id: None,
            kind: None,
            others: 5,
            ..arc.clone()
        };
        out.push(
            serde_json::to_value(SunburstSlice {
                node: sample_entry(),
                arcs: vec![arc, rest],
            })
            .unwrap(),
        );
        for k in [EntryKind::File, EntryKind::Symlink, EntryKind::Other] {
            out.push(serde_json::to_value(k).unwrap());
        }
        out
    }

    #[test]
    fn scan_event_json_shape() {
        let v = serde_json::to_value(ScanEvent::Progress {
            dirs: 1,
            files: 2,
            allocated: 3,
            elapsed_ms: 4,
        })
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({"event": "progress", "dirs": 1, "files": 2, "allocated": 3, "elapsedMs": 4})
        );
        let e = serde_json::to_value(sample_entry()).unwrap();
        assert_eq!(e["hasChildren"], true);
        assert_eq!(e["kind"], "dir");
        let err = serde_json::to_value(SessionError::StaleScan).unwrap();
        assert_eq!(err, serde_json::json!({"kind": "staleScan"}));
        let sort: SortKey = serde_json::from_str("\"modified\"").unwrap();
        assert_eq!(sort, SortKey::Modified);
        let order: SortOrder = serde_json::from_str("\"desc\"").unwrap();
        assert_eq!(order, SortOrder::Desc);
    }

    /// Every key and tag value the backend sends must appear in the
    /// hand-written TS types, so a rename on one side fails here.
    #[test]
    fn ipc_ts_mentions_every_serialized_key() {
        fn walk(v: &serde_json::Value, out: &mut Vec<String>) {
            match v {
                serde_json::Value::Object(map) => {
                    for (k, v) in map {
                        out.push(k.clone());
                        if (k == "event" || k == "kind")
                            && let Some(tag) = v.as_str()
                        {
                            out.push(format!("'{tag}'"));
                        }
                        walk(v, out);
                    }
                }
                serde_json::Value::Array(items) => items.iter().for_each(|i| walk(i, out)),
                _ => {}
            }
        }
        let ts = include_str!("../../ui/src/lib/ipc.ts");
        let mut words = Vec::new();
        for v in samples() {
            if let Some(tag) = v.as_str() {
                words.push(format!("'{tag}'"));
            }
            walk(&v, &mut words);
        }
        for tag in ["allocated", "logical", "files", "name", "modified"] {
            words.push(format!("'{tag}'"));
        }
        words.push("'asc'".to_owned());
        words.push("'desc'".to_owned());
        let missing: Vec<_> = words.iter().filter(|w| !ts.contains(w.as_str())).collect();
        assert!(missing.is_empty(), "ui/src/lib/ipc.ts lacks {missing:?}");
    }
}
