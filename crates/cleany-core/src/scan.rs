//! Parallel filesystem scanner (see docs/PLAN.md §3.2).
//!
//! Rayon workers list directories and send each listing over a channel to the
//! calling thread, which is the only writer of the [`TreeBuilder`]: it assigns
//! node ids and queues the subdirectories it finds. Links and junctions are
//! recorded but never followed; mount points are not crossed unless asked.
//! Hard-linked files are counted once (at their smallest path), and cloud
//! placeholder directories are recorded but never listed.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use cleany_platform::{EntryKind, EntryMeta, read_entry_meta, read_meta};
use thiserror::Error;

use crate::Size;
use crate::tree::{EntryInfo, NodeFlags, NodeId, NodeKind, Tree, TreeBuilder, TreeError};

/// Minimum time between two progress callbacks (at most 10 per second).
pub const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// How often the builder re-checks the cancel flag while waiting for workers.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Options for [`scan`].
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Descend into directories on other devices / volumes.
    pub cross_mounts: bool,
    /// Worker threads; `None` uses one per logical CPU.
    pub threads: Option<NonZeroUsize>,
}

/// Cooperative cancellation flag shared with a running [`scan`].
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token that is not cancelled yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks every scan holding this token to stop.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Whether [`CancelToken::cancel`] has been called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Snapshot passed to the progress callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Directories recorded so far, including the root.
    pub dirs: u64,
    /// Non-directory entries recorded so far.
    pub files: u64,
    /// Allocated bytes recorded so far (each hard-linked file once).
    pub allocated: u64,
    /// Time since the scan started.
    pub elapsed: Duration,
}

/// An entry that could not be read; the scan continues without it.
#[derive(Debug)]
pub struct ScanIssue {
    /// Path that failed.
    pub path: PathBuf,
    /// The I/O error.
    pub error: io::Error,
}

/// Counters for a finished scan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanStats {
    /// Directories, including the root.
    pub dirs: u64,
    /// Regular files.
    pub files: u64,
    /// Symlinks, junctions and other reparse points (not followed).
    pub symlinks: u64,
    /// Sockets, FIFOs, devices.
    pub others: u64,
    /// Directories on another device that were recorded but not entered.
    pub skipped_mounts: u64,
    /// Further hard links to a file already counted; they add no bytes.
    pub hardlinks: u64,
    /// Cloud placeholders: files whose data is not local, and directories
    /// that were recorded but not entered.
    pub placeholders: u64,
    /// Wall-clock duration of the scan.
    pub elapsed: Duration,
}

/// Output of [`scan`].
#[derive(Debug)]
pub struct ScanResult {
    /// Aggregated tree; the root is named after the path passed to [`scan`].
    pub tree: Tree,
    /// Entries that could not be read.
    pub issues: Vec<ScanIssue>,
    /// Counters.
    pub stats: ScanStats,
}

/// Errors that abort a [`scan`].
#[derive(Debug, Error)]
pub enum ScanError {
    /// The root itself could not be read.
    #[error("cannot read scan root {}: {source}", path.display())]
    Root {
        /// The root path.
        path: PathBuf,
        /// The I/O error.
        source: io::Error,
    },
    /// The scan was cancelled through its [`CancelToken`].
    #[error("scan cancelled")]
    Cancelled,
    /// The tree ran out of capacity.
    #[error(transparent)]
    Tree(#[from] TreeError),
    /// The worker pool could not be started.
    #[error("cannot start scan threads: {0}")]
    ThreadPool(String),
}

impl From<EntryKind> for NodeKind {
    fn from(kind: EntryKind) -> Self {
        match kind {
            EntryKind::File => Self::File,
            EntryKind::Dir => Self::Dir,
            EntryKind::Symlink => Self::Symlink,
            EntryKind::Other => Self::Other,
        }
    }
}

/// Scans `root` and returns its aggregated tree.
///
/// A symlink root is resolved once; nothing below it is followed. Unreadable
/// entries are collected in [`ScanResult::issues`]. `on_progress` runs on the
/// calling thread at most every [`PROGRESS_INTERVAL`].
///
/// # Errors
/// [`ScanError::Root`] if `root` cannot be read, [`ScanError::Cancelled`] if
/// `cancel` fires, or a capacity / thread-pool error.
pub fn scan<F>(
    root: &Path,
    opts: &ScanOptions,
    cancel: &CancelToken,
    mut on_progress: F,
) -> Result<ScanResult, ScanError>
where
    F: FnMut(&Progress),
{
    let start = Instant::now();
    if cancel.is_cancelled() {
        return Err(ScanError::Cancelled);
    }
    let root_err = |source| ScanError::Root {
        path: root.to_path_buf(),
        source,
    };
    let mut meta = read_meta(root).map_err(root_err)?;
    let mut walk_root = root.to_path_buf();
    if meta.kind == EntryKind::Symlink {
        walk_root = fs::canonicalize(root).map_err(root_err)?;
        meta = read_meta(&walk_root).map_err(root_err)?;
    }

    let mut state = Builder {
        tree: TreeBuilder::new(root.as_os_str(), entry_info(&meta))?,
        issues: Vec::new(),
        stats: ScanStats::default(),
        allocated: 0,
        links: HashMap::new(),
    };
    let root_id = state.tree.root();
    state.record(root_id, &meta);
    // An explicitly chosen root is listed even if it is a placeholder.
    if meta.kind != EntryKind::Dir {
        return state.finish(start);
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(opts.threads.map_or(0, NonZeroUsize::get))
        .thread_name(|i| format!("cleany-scan-{i}"))
        .build()
        .map_err(|e| ScanError::ThreadPool(e.to_string()))?;
    let (tx, rx) = mpsc::channel::<Listing>();
    let spawn = |dir: NodeId, path: PathBuf| spawn_listing(&pool, &tx, cancel, dir, path);

    spawn(root_id, walk_root);
    let mut pending: usize = 1;
    let mut throttle = Throttle::new(start, PROGRESS_INTERVAL);
    let mut cancelled = false;

    while pending > 0 {
        let listing = match rx.recv_timeout(POLL_INTERVAL) {
            Ok(listing) => listing,
            Err(RecvTimeoutError::Timeout) => {
                cancelled |= cancel.is_cancelled();
                continue;
            }
            // Unreachable while `tx` is alive; stop rather than spin.
            Err(RecvTimeoutError::Disconnected) => break,
        };
        pending -= 1;
        cancelled |= cancel.is_cancelled();
        if cancelled {
            // Drain in-flight workers (they skip work once cancelled).
            continue;
        }
        for (id, path) in state.apply(listing, meta.dev, opts.cross_mounts)? {
            spawn(id, path);
            pending += 1;
        }
        let now = Instant::now();
        if throttle.ready(now) {
            on_progress(&state.progress(now.duration_since(start)));
        }
    }

    if cancelled {
        return Err(ScanError::Cancelled);
    }
    state.finish(start)
}

/// Builder-thread state: the only writer of the tree.
struct Builder {
    tree: TreeBuilder,
    issues: Vec<ScanIssue>,
    stats: ScanStats,
    allocated: u64,
    /// Files with more than one link, by `(dev, file_id)`.
    links: HashMap<(u64, u64), Vec<NodeId>>,
}

impl Builder {
    /// Updates counters and hard-link groups for a node just added.
    fn record(&mut self, id: NodeId, meta: &EntryMeta) {
        let slot = match meta.kind {
            EntryKind::Dir => &mut self.stats.dirs,
            EntryKind::File => &mut self.stats.files,
            EntryKind::Symlink => &mut self.stats.symlinks,
            EntryKind::Other => &mut self.stats.others,
        };
        *slot += 1;
        if meta.placeholder {
            self.stats.placeholders += 1;
        }
        // Directories' link counts mean subdirectories on Unix, so only
        // files are grouped; an unknown id (0) is never grouped.
        let mut first = true;
        if meta.kind == EntryKind::File && meta.nlink > 1 && meta.file_id != 0 {
            let group = self.links.entry((meta.dev, meta.file_id)).or_default();
            first = group.is_empty();
            group.push(id);
        }
        if first {
            self.allocated = self.allocated.saturating_add(meta.allocated);
        }
    }

    /// Records one listing and returns the subdirectories to scan next.
    fn apply(
        &mut self,
        listing: Listing,
        root_dev: u64,
        cross_mounts: bool,
    ) -> Result<Vec<(NodeId, PathBuf)>, TreeError> {
        let Listing { dir, path, read } = listing;
        let read = match read {
            Ok(read) => read,
            Err(error) => {
                self.issues.push(ScanIssue { path, error });
                return Ok(Vec::new());
            }
        };
        self.issues
            .extend(read.errors.into_iter().map(|error| ScanIssue {
                path: path.clone(),
                error,
            }));
        let mut subdirs = Vec::new();
        for Entry { name, meta } in read.entries {
            let meta = match meta {
                Ok(meta) => meta,
                Err(error) => {
                    self.issues.push(ScanIssue {
                        path: path.join(&name),
                        error,
                    });
                    continue;
                }
            };
            let id = self.tree.add_child(dir, &name, entry_info(&meta))?;
            self.record(id, &meta);
            if meta.kind == EntryKind::Dir {
                if meta.placeholder {
                    // Listing a dataless directory can start a download.
                } else if cross_mounts || meta.dev == root_dev {
                    subdirs.push((id, path.join(&name)));
                } else {
                    self.stats.skipped_mounts += 1;
                }
            }
        }
        Ok(subdirs)
    }

    fn progress(&self, elapsed: Duration) -> Progress {
        Progress {
            dirs: self.stats.dirs,
            files: self.stats.files + self.stats.symlinks + self.stats.others,
            allocated: self.allocated,
            elapsed,
        }
    }

    /// Counts each hard-linked file once, at the link with the smallest
    /// path, so the result does not depend on listing order.
    fn dedup_hardlinks(&mut self) -> Result<(), TreeError> {
        for group in self.links.values().filter(|g| g.len() > 1) {
            let owner = group
                .iter()
                .copied()
                .min_by_key(|&id| self.tree.path(id))
                .ok_or(TreeError::NotAFile)?;
            for &id in group.iter().filter(|&&id| id != owner) {
                self.tree.mark_hardlink(id)?;
                self.stats.hardlinks += 1;
            }
        }
        Ok(())
    }

    fn finish(mut self, start: Instant) -> Result<ScanResult, ScanError> {
        self.dedup_hardlinks()?;
        self.stats.elapsed = start.elapsed();
        Ok(ScanResult {
            tree: self.tree.finish(),
            issues: self.issues,
            stats: self.stats,
        })
    }
}

/// One entry read by a worker.
struct Entry {
    name: OsString,
    meta: io::Result<EntryMeta>,
}

/// Result of listing one directory.
struct DirRead {
    entries: Vec<Entry>,
    /// Errors while iterating, where the entry name is unknown.
    errors: Vec<io::Error>,
}

/// Message from a worker to the builder.
struct Listing {
    dir: NodeId,
    path: PathBuf,
    read: io::Result<DirRead>,
}

fn spawn_listing(
    pool: &rayon::ThreadPool,
    tx: &Sender<Listing>,
    cancel: &CancelToken,
    dir: NodeId,
    path: PathBuf,
) {
    let tx = tx.clone();
    let cancel = cancel.clone();
    pool.spawn(move || {
        let read = if cancel.is_cancelled() {
            Ok(DirRead {
                entries: Vec::new(),
                errors: Vec::new(),
            })
        } else {
            list_dir(&path)
        };
        // A send error only means the scan already returned; nothing to do.
        let _ = tx.send(Listing { dir, path, read });
    });
}

fn list_dir(path: &Path) -> io::Result<DirRead> {
    let mut read = DirRead {
        entries: Vec::new(),
        errors: Vec::new(),
    };
    for entry in fs::read_dir(path)? {
        match entry {
            Ok(e) => read.entries.push(Entry {
                name: e.file_name(),
                meta: read_entry_meta(&e),
            }),
            Err(error) => read.errors.push(error),
        }
    }
    Ok(read)
}

fn entry_info(meta: &EntryMeta) -> EntryInfo {
    EntryInfo {
        kind: meta.kind.into(),
        size: Size {
            logical: meta.logical,
            allocated: meta.allocated,
        },
        mtime: meta.mtime,
        flags: if meta.placeholder {
            NodeFlags::PLACEHOLDER
        } else {
            NodeFlags::NONE
        },
    }
}

/// Lets an event through at most once per `interval`, starting one interval
/// after `start`.
struct Throttle {
    interval: Duration,
    last: Instant,
}

impl Throttle {
    const fn new(start: Instant, interval: Duration) -> Self {
        Self {
            interval,
            last: start,
        }
    }

    fn ready(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.last) >= self.interval {
            self.last = now;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PROGRESS_INTERVAL, Throttle};
    use std::time::{Duration, Instant};

    #[test]
    fn progress_is_throttled() {
        let t0 = Instant::now();
        let ms = Duration::from_millis;
        let mut th = Throttle::new(t0, PROGRESS_INTERVAL);
        assert!(!th.ready(t0 + ms(50)));
        assert!(th.ready(t0 + ms(100)));
        assert!(!th.ready(t0 + ms(150)));
        assert!(!th.ready(t0 + ms(199)));
        assert!(th.ready(t0 + ms(230)));
        assert!(!th.ready(t0 + ms(300)));
        assert!(th.ready(t0 + ms(330)));
    }
}
