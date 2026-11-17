//! The largest files and folders anywhere below a node (see docs/PLAN.md §M5).
//!
//! One depth-first walk keeps the best `limit` entries in a bounded min-heap,
//! so the cost is `O(n log limit)` over the subtree and nothing is cached.
//!
//! Folders are ranked by the bytes of their *direct* non-folder children, not
//! by their subtree total: an ancestor always outweighs its descendants, so
//! ranking by subtree would fill the list with one chain of nested folders.

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use crate::Size;
use crate::tree::{NodeFlags, NodeId, NodeKind, Tree};

/// Ranking key: larger allocated size first, then lower arena index, so
/// equal sizes come out in a stable order.
type Key = (u64, Reverse<u32>);

/// A ranked item, ordered by its key alone (keys are unique per node).
struct Ranked<T> {
    key: Key,
    item: T,
}

impl<T> PartialEq for Ranked<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for Ranked<T> {}

impl<T> PartialOrd for Ranked<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Ranked<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

/// Keeps the `limit` largest keys seen so far.
struct Best<T> {
    limit: usize,
    heap: BinaryHeap<Reverse<Ranked<T>>>,
}

impl<T> Best<T> {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            heap: BinaryHeap::with_capacity(limit.min(4096).saturating_add(1)),
        }
    }

    fn offer(&mut self, key: Key, item: T) {
        if self.limit == 0 {
            return;
        }
        if self.heap.len() < self.limit {
            self.heap.push(Reverse(Ranked { key, item }));
        } else if self
            .heap
            .peek()
            .is_some_and(|Reverse(worst)| key > worst.key)
        {
            self.heap.pop();
            self.heap.push(Reverse(Ranked { key, item }));
        }
    }

    /// Best first.
    fn into_sorted(self) -> Vec<T> {
        let mut all: Vec<Ranked<T>> = self.heap.into_iter().map(|Reverse(r)| r).collect();
        all.sort_by(|a, b| b.cmp(a));
        all.into_iter().map(|r| r.item).collect()
    }
}

fn key(id: NodeId, allocated: u64) -> Key {
    (
        allocated,
        Reverse(u32::try_from(id.index()).unwrap_or(u32::MAX)),
    )
}

/// Calls `visit` for `scope` and every node below it.
fn walk(tree: &Tree, scope: NodeId, mut visit: impl FnMut(NodeId)) {
    let mut stack = vec![scope];
    while let Some(id) = stack.pop() {
        visit(id);
        stack.extend(tree.children(id));
    }
}

/// The `limit` largest regular files in `scope`'s subtree (or `scope` itself
/// when it is a file), by allocated size, largest first. Hard links whose
/// bytes are counted on another node, symlinks and empty files are left out.
#[must_use]
pub fn largest_files(tree: &Tree, scope: NodeId, limit: usize) -> Vec<NodeId> {
    let mut best = Best::new(limit);
    walk(tree, scope, |id| {
        let Some(n) = tree.node(id) else { return };
        let allocated = n.size().allocated;
        if n.kind() == NodeKind::File && !n.flags().contains(NodeFlags::HARDLINK) && allocated > 0 {
            best.offer(key(id, allocated), id);
        }
    });
    best.into_sorted()
}

/// The `limit` folders in `scope`'s subtree (`scope` included) whose direct
/// non-folder children take the most allocated bytes, largest first, each
/// with that direct-children total. Folders holding no such bytes are left out.
#[must_use]
pub fn largest_dirs(tree: &Tree, scope: NodeId, limit: usize) -> Vec<(NodeId, Size)> {
    let mut best = Best::new(limit);
    walk(tree, scope, |id| {
        if tree.node(id).is_none_or(|n| n.kind() != NodeKind::Dir) {
            return;
        }
        let own = tree
            .children(id)
            .filter_map(|c| tree.node(c))
            .filter(|c| c.kind() != NodeKind::Dir)
            .fold(Size::default(), |sum, c| sum.saturating_add(c.size()));
        if own.allocated > 0 {
            best.offer(key(id, own.allocated), (id, own));
        }
    });
    best.into_sorted()
}

#[cfg(test)]
mod tests {
    use super::{largest_dirs, largest_files};
    use crate::Size;
    use crate::tree::{EntryInfo, NodeFlags, NodeId, NodeKind, Tree, TreeBuilder};
    use std::ffi::OsStr;

    fn info(kind: NodeKind, allocated: u64) -> EntryInfo {
        EntryInfo {
            kind,
            size: Size {
                logical: allocated,
                allocated,
            },
            mtime: 0,
            flags: NodeFlags::NONE,
        }
    }

    fn name(tree: &Tree, id: NodeId) -> String {
        tree.name(id).unwrap().to_string_lossy().into_owned()
    }

    fn names(tree: &Tree, ids: &[NodeId]) -> Vec<String> {
        ids.iter().map(|&id| name(tree, id)).collect()
    }

    /// ```text
    /// /r
    ///   small (5)
    ///   a/
    ///     a1 (10)
    ///     deep/
    ///       big (100)
    ///       d1 (1)
    ///   b/
    ///     b1 (50)
    ///     link -> symlink (70)
    ///     hard (0, hard link)
    ///     empty (0)
    /// ```
    fn sample() -> (Tree, NodeId, NodeId) {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0)).unwrap();
        let root = b.root();
        b.add_child(root, OsStr::new("small"), info(NodeKind::File, 5))
            .unwrap();
        let a = b
            .add_child(root, OsStr::new("a"), info(NodeKind::Dir, 0))
            .unwrap();
        b.add_child(a, OsStr::new("a1"), info(NodeKind::File, 10))
            .unwrap();
        let deep = b
            .add_child(a, OsStr::new("deep"), info(NodeKind::Dir, 0))
            .unwrap();
        b.add_child(deep, OsStr::new("big"), info(NodeKind::File, 100))
            .unwrap();
        b.add_child(deep, OsStr::new("d1"), info(NodeKind::File, 1))
            .unwrap();
        let bdir = b
            .add_child(root, OsStr::new("b"), info(NodeKind::Dir, 0))
            .unwrap();
        b.add_child(bdir, OsStr::new("b1"), info(NodeKind::File, 50))
            .unwrap();
        b.add_child(bdir, OsStr::new("link"), info(NodeKind::Symlink, 70))
            .unwrap();
        let hard = b
            .add_child(bdir, OsStr::new("hard"), info(NodeKind::File, 0))
            .unwrap();
        b.mark_hardlink(hard).unwrap();
        b.add_child(bdir, OsStr::new("empty"), info(NodeKind::File, 0))
            .unwrap();
        let tree = b.finish();
        let a = tree.children(tree.root()).find(|&c| name(&tree, c) == "a");
        let bdir = tree.children(tree.root()).find(|&c| name(&tree, c) == "b");
        (tree, a.unwrap(), bdir.unwrap())
    }

    #[test]
    fn files_largest_first_across_depths() {
        let (tree, _, _) = sample();
        let top = largest_files(&tree, tree.root(), 3);
        assert_eq!(names(&tree, &top), ["big", "b1", "a1"]);
    }

    #[test]
    fn files_limit_caps_results() {
        let (tree, _, _) = sample();
        assert_eq!(largest_files(&tree, tree.root(), 2).len(), 2);
        assert!(largest_files(&tree, tree.root(), 0).is_empty());
        // Asking for more than exists gives every candidate once.
        assert_eq!(largest_files(&tree, tree.root(), 100).len(), 5);
    }

    #[test]
    fn files_scoped_to_subtree() {
        let (tree, a, _) = sample();
        let top = largest_files(&tree, a, 10);
        assert_eq!(names(&tree, &top), ["big", "a1", "d1"]);
    }

    #[test]
    fn files_skip_hardlinks_symlinks_empty_and_dirs() {
        let (tree, _, b) = sample();
        let top = largest_files(&tree, b, 10);
        assert_eq!(names(&tree, &top), ["b1"]);
    }

    #[test]
    fn dirs_rank_by_direct_files_not_subtree() {
        let (tree, _, _) = sample();
        let top = largest_dirs(&tree, tree.root(), 10);
        let ranked: Vec<(String, u64)> = top
            .iter()
            .map(|&(id, own)| (name(&tree, id), own.allocated))
            .collect();
        // `a` holds 111 bytes in its subtree but only 10 directly; `b` counts
        // its symlink's own bytes, as the listing does.
        assert_eq!(
            ranked,
            [
                ("b".to_owned(), 120),
                ("deep".to_owned(), 101),
                ("a".to_owned(), 10),
                ("/r".to_owned(), 5),
            ]
        );
    }

    #[test]
    fn dirs_include_scope_root_itself() {
        let (tree, a, _) = sample();
        let top = largest_dirs(&tree, a, 10);
        assert!(top.iter().any(|&(id, _)| id == a));
    }

    #[test]
    fn ties_break_by_id() {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0)).unwrap();
        let root = b.root();
        for n in ["x", "y", "z"] {
            b.add_child(root, OsStr::new(n), info(NodeKind::File, 7))
                .unwrap();
        }
        let tree = b.finish();
        let top = largest_files(&tree, tree.root(), 2);
        let mut expected: Vec<NodeId> = tree.children(tree.root()).collect();
        expected.sort();
        expected.truncate(2);
        assert_eq!(top, expected);
        assert_eq!(top, largest_files(&tree, tree.root(), 2));
    }

    #[test]
    fn detached_nodes_are_gone() {
        let (mut tree, a, _) = sample();
        let deep = tree.children(a).find(|&c| name(&tree, c) == "deep");
        tree.detach(deep.unwrap()).unwrap();
        let files = largest_files(&tree, tree.root(), 10);
        assert!(!names(&tree, &files).contains(&"big".to_owned()));
        let dirs = largest_dirs(&tree, tree.root(), 10);
        assert!(dirs.iter().all(|&(id, _)| name(&tree, id) != "deep"));
    }
}
