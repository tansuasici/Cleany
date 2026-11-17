//! Keeping a finished scan tree current without rescanning it (ADR-0006).
//!
//! Change notifications are folded by a [`Coalescer`] into a short list of
//! directories to refresh. Each directory is then refreshed in three steps so
//! that a caller holding the tree behind a lock does disk I/O without it:
//!
//! 1. [`request`] (tree, read-only): what is known about the directory.
//! 2. [`read`] (disk, no tree): list it, and scan new subdirectories.
//! 3. [`apply`] (tree, mutable): patch the tree in place.
//!
//! A shallow refresh matches the listing with the existing children by name
//! and kind: files are updated in place, vanished entries are detached, new
//! files are appended and new subdirectories are scanned and grafted.
//! Existing subdirectories are left alone; their own changes arrive as their
//! own notifications. Ids of untouched nodes never change. Patches are
//! idempotent, so a change the app made itself (e.g. a move to the trash it
//! already applied with [`Tree::detach`]) refreshes to a no-op.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cleany_platform::{EntryKind, EntryMeta, read_entry_meta, read_meta};

use crate::firmlinks;
use crate::scan::{self, CancelToken, ScanError, ScanIssue, ScanOptions, entry_info};
use crate::tree::{NodeId, NodeKind, Tree, TreeError};

/// Changes are applied once no new one arrived for this long.
pub const QUIET_WINDOW: Duration = Duration::from_secs(1);

/// Changes are applied at least this often while they keep arriving.
pub const MAX_DELAY: Duration = Duration::from_secs(5);

/// More changed subdirectories than this under one parent are replaced by
/// one rescan of the parent.
pub const SIBLING_LIMIT: usize = 256;

/// More pending directories than this give up and rescan the whole root.
pub const BUFFER_LIMIT: usize = 10_000;

/// What a change notification says about a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The entry itself was created, removed or modified; its parent
    /// directory needs a fresh listing.
    Entry,
    /// Anything below the path may have changed (events were dropped or
    /// merged); the whole subtree needs a rescan.
    Subtree,
}

/// How deep one directory is refreshed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Depth {
    /// List the directory one level deep and patch its direct children.
    Shallow,
    /// Rescan everything below the directory.
    Subtree,
}

/// What a [`Coalescer`] wants refreshed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshPlan {
    /// Too much changed (or the root itself): scan the root again.
    Full,
    /// Refresh these directories, parents before children. May be empty.
    Dirs(Vec<(PathBuf, Depth)>),
}

/// Folds change notifications into a [`RefreshPlan`]; pure logic, no I/O.
///
/// Each changed entry marks its parent directory; paths outside the root are
/// dropped; directories below a [`Depth::Subtree`] mark are dropped; too
/// many changed siblings escalate to their parent.
#[derive(Debug)]
pub struct Coalescer {
    root: PathBuf,
    dirs: BTreeMap<PathBuf, Depth>,
    overflow: bool,
    first: Option<Instant>,
    last: Option<Instant>,
}

impl Coalescer {
    /// A coalescer for changes below `root` (the scanned path).
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            dirs: BTreeMap::new(),
            overflow: false,
            first: None,
            last: None,
        }
    }

    /// Records one change seen at `now`. Paths outside the root are ignored.
    pub fn push(&mut self, path: &Path, change: Change, now: Instant) {
        let Ok(rel) = path.strip_prefix(&self.root) else {
            return;
        };
        let (dir, depth) = match change {
            Change::Subtree => (path, Depth::Subtree),
            Change::Entry if rel.as_os_str().is_empty() => (path, Depth::Shallow),
            Change::Entry => (path.parent().unwrap_or(path), Depth::Shallow),
        };
        self.first.get_or_insert(now);
        self.last = Some(now);
        if self.overflow {
            return;
        }
        let slot = self.dirs.entry(dir.to_path_buf()).or_insert(depth);
        *slot = (*slot).max(depth);
        if self.dirs.len() > BUFFER_LIMIT {
            self.overflow = true;
            self.dirs.clear();
        }
    }

    /// Whether nothing is pending.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.first.is_none()
    }

    /// Whether pending changes should be applied at `now`: nothing arrived
    /// for [`QUIET_WINDOW`], or the oldest waited [`MAX_DELAY`].
    #[must_use]
    pub fn ready(&self, now: Instant) -> bool {
        match (self.first, self.last) {
            (Some(first), Some(last)) => {
                now.saturating_duration_since(last) >= QUIET_WINDOW
                    || now.saturating_duration_since(first) >= MAX_DELAY
            }
            _ => false,
        }
    }

    /// Takes everything pending as a plan and starts over.
    pub fn take(&mut self) -> RefreshPlan {
        self.first = None;
        self.last = None;
        let dirs = std::mem::take(&mut self.dirs);
        if std::mem::take(&mut self.overflow) {
            return RefreshPlan::Full;
        }
        let mut dirs = drop_covered(dirs);
        // Escalate parents with too many changed subdirectories.
        let mut per_parent: HashMap<PathBuf, usize> = HashMap::new();
        for dir in dirs.keys() {
            if let Some(parent) = dir.parent() {
                *per_parent.entry(parent.to_path_buf()).or_default() += 1;
            }
        }
        let crowded: Vec<PathBuf> = per_parent
            .into_iter()
            .filter(|(parent, n)| *n > SIBLING_LIMIT && parent.starts_with(&self.root))
            .map(|(parent, _)| parent)
            .collect();
        if !crowded.is_empty() {
            for parent in crowded {
                dirs.insert(parent, Depth::Subtree);
            }
            dirs = drop_covered(dirs);
        }
        if dirs.get(&self.root) == Some(&Depth::Subtree) {
            return RefreshPlan::Full;
        }
        RefreshPlan::Dirs(dirs.into_iter().collect())
    }
}

/// Drops every directory below a [`Depth::Subtree`] mark. Paths sort by
/// component, so a directory's descendants follow it directly.
fn drop_covered(dirs: BTreeMap<PathBuf, Depth>) -> BTreeMap<PathBuf, Depth> {
    let mut out = BTreeMap::new();
    let mut cover: Option<PathBuf> = None;
    for (dir, depth) in dirs {
        if cover.as_ref().is_some_and(|c| dir.starts_with(c)) {
            continue;
        }
        if depth == Depth::Subtree {
            cover = Some(dir.clone());
        }
        out.insert(dir, depth);
    }
    out
}

/// Step 1 of a refresh: what the tree knows about one directory.
#[derive(Debug, Clone)]
pub struct DirRequest {
    path: PathBuf,
    depth: Depth,
    /// Whether the directory is the scan root, which is always read.
    is_root: bool,
    /// Names of the directory's subdirectories already in the tree.
    known_dirs: HashSet<OsString>,
    /// The tree's root path, which decides the firmlink aliases to skip.
    origin: PathBuf,
}

/// Prepares a refresh of the directory at `path`. `None` if `path` is not a
/// live directory of `tree` (e.g. an ancestor was removed meanwhile).
#[must_use]
pub fn request(tree: &Tree, path: &Path, depth: Depth) -> Option<DirRequest> {
    let id = tree.lookup(path)?;
    if tree.node(id)?.kind() != NodeKind::Dir {
        return None;
    }
    let known_dirs = tree
        .children(id)
        .filter(|&c| tree.node(c).is_some_and(|n| n.kind() == NodeKind::Dir))
        .filter_map(|c| tree.name(c))
        .collect();
    Some(DirRequest {
        path: path.to_path_buf(),
        depth,
        is_root: id == tree.root(),
        known_dirs,
        origin: tree.path(tree.root())?,
    })
}

/// Step 2 of a refresh: the directory as read from disk.
#[derive(Debug)]
pub struct DirUpdate {
    path: PathBuf,
    read: Result<Read, io::Error>,
    issues: Vec<ScanIssue>,
}

#[derive(Debug)]
enum Read {
    Shallow {
        entries: Vec<(OsString, EntryMeta)>,
        /// Entries whose metadata failed: kept as they are, not detached.
        unreadable: HashSet<OsString>,
        /// Fresh scans of subdirectories the tree does not know yet.
        subtrees: HashMap<OsString, Tree>,
    },
    Subtree(Tree),
    /// The directory is one the scanner does not enter (a cloud
    /// placeholder or another volume), or no longer a directory.
    Outside,
}

/// Reads the directory of `req` from disk. A directory below the root that
/// the scanner would not enter (cloud placeholder, other volume) is not
/// read and patches to nothing. New subdirectories (and, for
/// [`Depth::Subtree`], the directory itself) are scanned with `opts`, whose
/// [`ScanOptions::root_dev`] should be the original scan's device so mount
/// points are treated the same as in the first scan. Takes no tree, so the
/// caller can run it without holding a lock.
#[must_use]
pub fn read(req: DirRequest, opts: &ScanOptions, cancel: &CancelToken) -> DirUpdate {
    let DirRequest {
        path,
        depth,
        is_root,
        known_dirs,
        origin,
    } = req;
    let mut issues = Vec::new();
    // Below the root, apply the scanner's rules to the directory itself:
    // listing a placeholder can start a download, and another volume's
    // files were never counted.
    if !is_root {
        match read_meta(&path) {
            Ok(meta) => {
                let other_volume =
                    !opts.cross_mounts && opts.root_dev.is_some_and(|dev| dev != meta.dev);
                if meta.kind != EntryKind::Dir
                    || meta.placeholder
                    || other_volume
                    || firmlinks::is_alias(&origin, &path)
                {
                    return DirUpdate {
                        path,
                        read: Ok(Read::Outside),
                        issues,
                    };
                }
            }
            Err(e) => {
                return DirUpdate {
                    path,
                    read: Err(e),
                    issues,
                };
            }
        }
    }
    let read = match depth {
        Depth::Subtree => match scan::scan_from(&path, &origin, opts, cancel, |_| {}) {
            Ok(res) => {
                issues = res.issues;
                Ok(Read::Subtree(res.tree))
            }
            Err(e) => Err(scan_error(e)),
        },
        Depth::Shallow => read_shallow(&path, &origin, &known_dirs, opts, cancel, &mut issues),
    };
    DirUpdate { path, read, issues }
}

fn read_shallow(
    path: &Path,
    origin: &Path,
    known_dirs: &HashSet<OsString>,
    opts: &ScanOptions,
    cancel: &CancelToken,
    issues: &mut Vec<ScanIssue>,
) -> Result<Read, io::Error> {
    let root_dev = match opts.root_dev {
        Some(dev) => dev,
        None => read_meta(path)?.dev,
    };
    let mut entries = Vec::new();
    let mut unreadable = HashSet::new();
    let mut subtrees = HashMap::new();
    let aliases = firmlinks::aliases_below(origin, path);
    for entry in fs::read_dir(path)? {
        let entry = match entry {
            Ok(e) => e,
            Err(error) => {
                issues.push(ScanIssue {
                    path: path.to_path_buf(),
                    error,
                });
                continue;
            }
        };
        let name = entry.file_name();
        let meta = match read_entry_meta(&entry) {
            Ok(meta) => meta,
            Err(error) => {
                issues.push(ScanIssue {
                    path: path.join(&name),
                    error,
                });
                unreadable.insert(name);
                continue;
            }
        };
        // Same rules as the scanner: never list placeholders, other volumes
        // or firmlink aliases.
        let descend = meta.kind == EntryKind::Dir
            && !meta.placeholder
            && (opts.cross_mounts || meta.dev == root_dev)
            && !known_dirs.contains(&name)
            && !aliases.contains(&path.join(&name));
        if descend {
            let sub_opts = ScanOptions {
                root_dev: Some(root_dev),
                ..opts.clone()
            };
            match scan::scan_from(&path.join(&name), origin, &sub_opts, cancel, |_| {}) {
                Ok(res) => {
                    issues.extend(res.issues);
                    subtrees.insert(name.clone(), res.tree);
                }
                Err(ScanError::Cancelled) => {
                    return Err(io::Error::from(io::ErrorKind::Interrupted));
                }
                // The directory is still recorded, with its own size only.
                Err(e) => issues.push(ScanIssue {
                    path: path.join(&name),
                    error: scan_error(e),
                }),
            }
        }
        entries.push((name, meta));
    }
    Ok(Read::Shallow {
        entries,
        unreadable,
        subtrees,
    })
}

fn scan_error(e: ScanError) -> io::Error {
    match e {
        ScanError::Root { source, .. } => source,
        ScanError::Cancelled => io::Error::from(io::ErrorKind::Interrupted),
        other => io::Error::other(other.to_string()),
    }
}

/// What [`apply`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The tree already matched the disk.
    Unchanged,
    /// The children of this directory changed.
    Changed(NodeId),
    /// The directory is no longer in the tree, or could not be read; nothing
    /// was done.
    Skipped,
    /// The scan root itself is gone; only a new scan can tell what is left.
    RootGone,
}

/// Result of [`apply`].
#[derive(Debug)]
pub struct Applied {
    /// What changed.
    pub outcome: Outcome,
    /// Entries that could not be read while refreshing.
    pub issues: Vec<ScanIssue>,
}

/// Step 3 of a refresh: patches `tree` with `update`. The directory is
/// looked up again by path, so a tree changed since [`request`] is fine.
///
/// # Errors
/// A capacity error from the tree; the tree is still consistent.
pub fn apply(tree: &mut Tree, update: DirUpdate) -> Result<Applied, TreeError> {
    let DirUpdate {
        path,
        read,
        mut issues,
    } = update;
    let outcome = patch(tree, &path, read, &mut issues)?;
    Ok(Applied { outcome, issues })
}

fn patch(
    tree: &mut Tree,
    path: &Path,
    read: Result<Read, io::Error>,
    issues: &mut Vec<ScanIssue>,
) -> Result<Outcome, TreeError> {
    let Some(dir) = tree.lookup(path) else {
        return Ok(Outcome::Skipped);
    };
    if tree.node(dir).map(|n| n.kind()) != Some(NodeKind::Dir) {
        return Ok(Outcome::Skipped);
    }
    let read = match read {
        Ok(read) => read,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if dir == tree.root() {
                return Ok(Outcome::RootGone);
            }
            let parent = tree.node(dir).and_then(|n| n.parent());
            tree.detach(dir);
            return Ok(parent.map_or(Outcome::Unchanged, Outcome::Changed));
        }
        Err(e) if e.kind() == io::ErrorKind::Interrupted => return Ok(Outcome::Skipped),
        Err(error) => {
            issues.push(ScanIssue {
                path: path.to_path_buf(),
                error,
            });
            return Ok(Outcome::Skipped);
        }
    };
    let changed = match read {
        Read::Outside => false,
        Read::Subtree(sub) => {
            let old: Vec<NodeId> = tree.children(dir).collect();
            for c in old {
                tree.detach(c);
            }
            tree.graft_children(dir, &sub)?;
            true
        }
        Read::Shallow {
            entries,
            unreadable,
            mut subtrees,
        } => {
            let mut existing: HashMap<OsString, NodeId> = tree
                .children(dir)
                .filter_map(|c| tree.name(c).map(|n| (n, c)))
                .collect();
            let mut changed = false;
            for (name, meta) in entries {
                let info = entry_info(&meta);
                if let Some(id) = existing.remove(&name) {
                    if tree.node(id).map(|n| n.kind()) == Some(info.kind) {
                        if info.kind != NodeKind::Dir {
                            changed |= tree.update_entry(id, info.size, info.mtime)?;
                        }
                        continue;
                    }
                    // Same name, different kind: replace the entry.
                    tree.detach(id);
                }
                match subtrees.remove(&name) {
                    Some(sub) if info.kind == NodeKind::Dir => {
                        tree.graft(dir, &name, &sub)?;
                    }
                    _ => {
                        tree.add_child(dir, &name, info)?;
                    }
                }
                changed = true;
            }
            for (name, id) in existing {
                if !unreadable.contains(&name) {
                    tree.detach(id);
                    changed = true;
                }
            }
            changed
        }
    };
    Ok(if changed {
        Outcome::Changed(dir)
    } else {
        Outcome::Unchanged
    })
}

/// [`request`], [`read`] and [`apply`] in one call, for callers that own
/// the tree outright.
///
/// # Errors
/// See [`apply`].
pub fn refresh_dir(
    tree: &mut Tree,
    path: &Path,
    depth: Depth,
    opts: &ScanOptions,
    cancel: &CancelToken,
) -> Result<Applied, TreeError> {
    let Some(req) = request(tree, path, depth) else {
        return Ok(Applied {
            outcome: Outcome::Skipped,
            issues: Vec::new(),
        });
    };
    apply(tree, read(req, opts, cancel))
}

#[cfg(test)]
mod tests {
    use super::{
        BUFFER_LIMIT, Change, Coalescer, Depth, MAX_DELAY, QUIET_WINDOW, RefreshPlan, SIBLING_LIMIT,
    };
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    /// Refreshes below a scan that skipped a firmlink alias keep skipping
    /// it: a subtree rescan of the alias's parent and a refresh of the alias
    /// itself must not bring its files back a second time.
    #[test]
    fn refresh_keeps_firmlink_aliases_unentered() {
        use crate::firmlinks::{Firmlink, override_table};
        use crate::scan::{CancelToken, ScanOptions, scan};
        use std::fs;

        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        for dir in ["Users", "data/Users", "data/extra"] {
            fs::create_dir_all(root.join(dir)).unwrap();
            fs::write(root.join(dir).join("f.bin"), vec![1u8; 10_000]).unwrap();
        }
        let alias = root.join("data/Users");
        let _table = override_table(vec![Firmlink {
            source: root.join("Users"),
            alias: alias.clone(),
        }]);
        let opts = ScanOptions::default();
        let cancel = CancelToken::new();
        let mut tree = scan(&root, &opts, &cancel, |_| {}).unwrap().tree;
        let files = |tree: &crate::Tree| tree.node(tree.root()).unwrap().file_count();
        assert_eq!(files(&tree), 2);

        for (dir, depth) in [
            (root.join("data"), Depth::Subtree),
            (root.join("data"), Depth::Shallow),
            (alias.clone(), Depth::Shallow),
            (alias.clone(), Depth::Subtree),
        ] {
            super::refresh_dir(&mut tree, &dir, depth, &opts, &cancel).unwrap();
            assert_eq!(files(&tree), 2, "{} {depth:?}", dir.display());
            let id = tree.lookup(&alias).unwrap();
            assert_eq!(tree.children(id).count(), 0);
        }
    }

    fn dirs(plan: RefreshPlan) -> Vec<(PathBuf, Depth)> {
        match plan {
            RefreshPlan::Dirs(d) => d,
            RefreshPlan::Full => panic!("expected directories, got a full rescan"),
        }
    }

    #[test]
    fn coalesce_marks_parent_dirs_and_keeps_parents_first() {
        let now = Instant::now();
        let mut c = Coalescer::new(Path::new("/r"));
        c.push(Path::new("/r/a/b/f.txt"), Change::Entry, now);
        c.push(Path::new("/r/a/g.txt"), Change::Entry, now);
        c.push(Path::new("/r/a/h.txt"), Change::Entry, now);
        c.push(Path::new("/r"), Change::Entry, now);
        assert_eq!(
            dirs(c.take()),
            [
                (PathBuf::from("/r"), Depth::Shallow),
                (PathBuf::from("/r/a"), Depth::Shallow),
                (PathBuf::from("/r/a/b"), Depth::Shallow),
            ]
        );
        assert!(c.is_empty());
        assert_eq!(c.take(), RefreshPlan::Dirs(Vec::new()));
    }

    #[test]
    fn coalesce_drops_paths_under_a_subtree_rescan() {
        let now = Instant::now();
        let mut c = Coalescer::new(Path::new("/r"));
        c.push(Path::new("/r/a/b/f"), Change::Entry, now);
        c.push(Path::new("/r/a/b/c/g"), Change::Entry, now);
        c.push(Path::new("/r/a"), Change::Subtree, now);
        c.push(Path::new("/r/ab/x"), Change::Entry, now);
        c.push(Path::new("/r/a/z"), Change::Entry, now);
        assert_eq!(
            dirs(c.take()),
            [
                (PathBuf::from("/r/a"), Depth::Subtree),
                (PathBuf::from("/r/ab"), Depth::Shallow),
            ]
        );
    }

    #[test]
    fn coalesce_escalates_many_siblings_to_common_ancestor() {
        let now = Instant::now();
        let mut c = Coalescer::new(Path::new("/r"));
        for i in 0..=SIBLING_LIMIT {
            c.push(&PathBuf::from(format!("/r/p/d{i}/f")), Change::Entry, now);
        }
        c.push(Path::new("/r/q/f"), Change::Entry, now);
        assert_eq!(
            dirs(c.take()),
            [
                (PathBuf::from("/r/p"), Depth::Subtree),
                (PathBuf::from("/r/q"), Depth::Shallow),
            ]
        );

        // Crowding right under the root means everything: a full rescan.
        for i in 0..=SIBLING_LIMIT {
            c.push(&PathBuf::from(format!("/r/d{i}/f")), Change::Entry, now);
        }
        assert_eq!(c.take(), RefreshPlan::Full);
    }

    #[test]
    fn coalesce_ignores_paths_outside_root() {
        let now = Instant::now();
        let mut c = Coalescer::new(Path::new("/r"));
        c.push(Path::new("/other/f"), Change::Entry, now);
        c.push(Path::new("/rx/f"), Change::Subtree, now);
        c.push(Path::new("/"), Change::Subtree, now);
        assert!(c.is_empty());
        assert!(!c.ready(now + MAX_DELAY));
        c.push(Path::new("/r"), Change::Subtree, now);
        assert_eq!(c.take(), RefreshPlan::Full);
    }

    #[test]
    fn coalesce_overflows_to_full_rescan() {
        let now = Instant::now();
        let mut c = Coalescer::new(Path::new("/r"));
        // Distinct parents so no sibling group gets crowded first.
        for i in 0..=BUFFER_LIMIT {
            c.push(
                &PathBuf::from(format!("/r/d{}/e{i}/f", i % 200)),
                Change::Entry,
                now,
            );
        }
        assert_eq!(c.take(), RefreshPlan::Full);
        assert!(c.is_empty());
    }

    #[test]
    fn coalesce_waits_for_quiet_or_max_delay() {
        let t0 = Instant::now();
        let mut c = Coalescer::new(Path::new("/r"));
        assert!(!c.ready(t0));
        c.push(Path::new("/r/f"), Change::Entry, t0);
        assert!(!c.ready(t0 + Duration::from_millis(500)));
        assert!(c.ready(t0 + QUIET_WINDOW));
        // A steady stream never goes quiet, but MAX_DELAY still fires.
        let mut t = t0;
        while t < t0 + MAX_DELAY {
            t += Duration::from_millis(500);
            c.push(Path::new("/r/f"), Change::Entry, t);
            if t < t0 + MAX_DELAY {
                assert!(!c.ready(t));
            }
        }
        assert!(c.ready(t));
    }
}
