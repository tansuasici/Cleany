//! Arena tree of scanned entries (see docs/PLAN.md §3.1).
//!
//! Nodes live in one `Vec` and link to each other by `u32` index, names live
//! in a packed pool. Build with [`TreeBuilder`], which only records each
//! entry's *own* size; [`TreeBuilder::finish`] rolls sizes up bottom-up once
//! and yields an immutable [`Tree`]. Children are always appended after their
//! parent, so a single reverse pass over the arena aggregates the whole tree.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use thiserror::Error;

use crate::Size;
use crate::names::NamePool;

const NONE: u32 = u32::MAX;

/// Errors raised while building a [`Tree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TreeError {
    /// The tree reached `u32::MAX - 1` nodes.
    #[error("too many entries for one scan tree")]
    TooManyNodes,
    /// The name pool exceeded `u32::MAX` units.
    #[error("name storage for the scan tree is full")]
    NamePoolFull,
    /// A single name is longer than `u16::MAX` units.
    #[error("entry name is too long")]
    NameTooLong,
    /// The parent id does not refer to a directory in this tree.
    #[error("parent is not a directory in this tree")]
    InvalidParent,
    /// The id does not refer to a non-directory in this tree.
    #[error("node is not a file in this tree")]
    NotAFile,
}

/// Handle to a node in a [`Tree`]. Only valid for the tree that issued it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

impl NodeId {
    /// Position of the node in the arena (stable for the tree's lifetime).
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// What kind of filesystem entry a node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NodeKind {
    /// Regular file.
    File,
    /// Directory.
    Dir,
    /// Symbolic link, junction or other reparse point (never followed).
    Symlink,
    /// Anything else: sockets, FIFOs, devices.
    Other,
}

/// Extra facts about a node, as a small bit set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NodeFlags(u8);

impl NodeFlags {
    /// No flags set.
    pub const NONE: Self = Self(0);
    /// Another hard link to the same file in this tree carries its bytes;
    /// this node's own size is zero so the data is counted once.
    pub const HARDLINK: Self = Self(1);
    /// Cloud placeholder whose data is not stored locally. A placeholder
    /// directory is recorded but not descended into.
    pub const PLACEHOLDER: Self = Self(1 << 1);

    /// Whether every flag in `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Both sets of flags.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// One entry of the tree. Sizes are aggregated for directories once the tree
/// is finished.
#[derive(Debug, Clone)]
pub struct Node {
    parent: u32,
    first_child: u32,
    next_sibling: u32,
    name_off: u32,
    name_len: u16,
    kind: NodeKind,
    flags: NodeFlags,
    file_count: u32,
    size: Size,
    mtime: i64,
}

impl Node {
    /// Entry kind.
    #[must_use]
    pub const fn kind(&self) -> NodeKind {
        self.kind
    }

    /// Flags such as [`NodeFlags::HARDLINK`].
    #[must_use]
    pub const fn flags(&self) -> NodeFlags {
        self.flags
    }

    /// Size of the entry; for directories, the entry itself plus everything below it.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Number of non-directory entries in this subtree (1 for a file).
    #[must_use]
    pub const fn file_count(&self) -> u32 {
        self.file_count
    }

    /// Modification time in seconds since the Unix epoch.
    #[must_use]
    pub const fn mtime(&self) -> i64 {
        self.mtime
    }

    /// Parent node, `None` for the root.
    #[must_use]
    pub const fn parent(&self) -> Option<NodeId> {
        opt(self.parent)
    }
}

const fn opt(raw: u32) -> Option<NodeId> {
    if raw == NONE { None } else { Some(NodeId(raw)) }
}

/// Data recorded for a new entry: its own size, not including children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryInfo {
    /// Entry kind.
    pub kind: NodeKind,
    /// Own size of the entry (for a directory, just its own blocks).
    pub size: Size,
    /// Modification time in seconds since the Unix epoch.
    pub mtime: i64,
    /// Initial flags (the scanner sets [`NodeFlags::PLACEHOLDER`]).
    pub flags: NodeFlags,
}

/// Arena storage shared by [`TreeBuilder`] and [`Tree`].
#[derive(Debug)]
struct Arena {
    nodes: Vec<Node>,
    names: NamePool,
}

impl Arena {
    fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.index())
    }

    fn path(&self, id: NodeId) -> Option<PathBuf> {
        let mut parts = Vec::new();
        let mut cur = Some(id);
        while let Some(c) = cur {
            let n = self.node(c)?;
            parts.push(self.names.get(n.name_off, n.name_len));
            cur = n.parent();
        }
        Some(parts.iter().rev().collect())
    }
}

/// Incrementally builds a [`Tree`]; sizes are rolled up by [`TreeBuilder::finish`].
#[derive(Debug)]
pub struct TreeBuilder {
    arena: Arena,
}

impl TreeBuilder {
    /// Starts a tree whose root is named `root_name` (usually the scanned path).
    ///
    /// # Errors
    /// Returns [`TreeError::NameTooLong`] if `root_name` does not fit the pool.
    pub fn new(root_name: &OsStr, root: EntryInfo) -> Result<Self, TreeError> {
        let mut arena = Arena {
            nodes: Vec::new(),
            names: NamePool::default(),
        };
        let (name_off, name_len) = arena.names.push(root_name)?;
        arena.nodes.push(new_node(NONE, name_off, name_len, root));
        Ok(Self { arena })
    }

    /// The root node.
    #[must_use]
    pub const fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Number of nodes added so far, including the root.
    #[must_use]
    pub fn len(&self) -> usize {
        self.arena.nodes.len()
    }

    /// Always `false`: a builder holds at least the root.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.arena.nodes.is_empty()
    }

    /// Adds `name` under directory `parent` and returns the new node.
    ///
    /// # Errors
    /// [`TreeError::InvalidParent`] if `parent` is not a directory of this
    /// tree, otherwise a capacity error from the arena or name pool.
    pub fn add_child(
        &mut self,
        parent: NodeId,
        name: &OsStr,
        info: EntryInfo,
    ) -> Result<NodeId, TreeError> {
        let parent_first = match self.arena.node(parent) {
            Some(p) if p.kind == NodeKind::Dir => p.first_child,
            _ => return Err(TreeError::InvalidParent),
        };
        let raw = u32::try_from(self.arena.nodes.len()).map_err(|_| TreeError::TooManyNodes)?;
        if raw == NONE {
            return Err(TreeError::TooManyNodes);
        }
        let (name_off, name_len) = self.arena.names.push(name)?;
        let mut node = new_node(parent.0, name_off, name_len, info);
        node.next_sibling = parent_first;
        self.arena.nodes.push(node);
        if let Some(p) = self.arena.nodes.get_mut(parent.index()) {
            p.first_child = raw;
        }
        Ok(NodeId(raw))
    }

    /// Full path of a node added so far; see [`Tree::path`].
    #[must_use]
    pub fn path(&self, id: NodeId) -> Option<PathBuf> {
        self.arena.path(id)
    }

    /// Marks a non-directory as a further hard link to data counted at
    /// another node: its own size becomes zero and it gets
    /// [`NodeFlags::HARDLINK`]. It still counts as one file.
    ///
    /// # Errors
    /// [`TreeError::NotAFile`] if `id` is a directory or not in this tree.
    pub fn mark_hardlink(&mut self, id: NodeId) -> Result<(), TreeError> {
        match self.arena.nodes.get_mut(id.index()) {
            Some(n) if n.kind != NodeKind::Dir => {
                n.size = Size::default();
                n.flags = n.flags.union(NodeFlags::HARDLINK);
                Ok(())
            }
            _ => Err(TreeError::NotAFile),
        }
    }

    /// Rolls sizes and file counts up to every ancestor and freezes the tree.
    #[must_use]
    pub fn finish(mut self) -> Tree {
        let nodes = &mut self.arena.nodes;
        // Children always have a higher index than their parent, so walking
        // the arena backwards visits every child before its parent.
        for i in (1..nodes.len()).rev() {
            let (size, files, parent) = {
                let n = &nodes[i];
                (n.size, n.file_count, n.parent as usize)
            };
            if let Some(p) = nodes.get_mut(parent) {
                p.size = p.size.saturating_add(size);
                p.file_count = p.file_count.saturating_add(files);
            }
        }
        Tree { arena: self.arena }
    }
}

const fn new_node(parent: u32, name_off: u32, name_len: u16, info: EntryInfo) -> Node {
    Node {
        parent,
        first_child: NONE,
        next_sibling: NONE,
        name_off,
        name_len,
        kind: info.kind,
        flags: info.flags,
        file_count: if matches!(info.kind, NodeKind::Dir) {
            0
        } else {
            1
        },
        size: info.size,
        mtime: info.mtime,
    }
}

/// A finished, aggregated scan tree.
#[derive(Debug)]
pub struct Tree {
    arena: Arena,
}

impl Tree {
    /// The root node.
    #[must_use]
    pub const fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Number of nodes, including the root.
    #[must_use]
    pub fn len(&self) -> usize {
        self.arena.nodes.len()
    }

    /// Always `false`: a tree holds at least the root.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.arena.nodes.is_empty()
    }

    /// The node for `id`, or `None` if `id` is not from this tree.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.arena.node(id)
    }

    /// Name of the entry (the full scanned path for the root).
    #[must_use]
    pub fn name(&self, id: NodeId) -> Option<OsString> {
        self.node(id)
            .map(|n| self.arena.names.get(n.name_off, n.name_len))
    }

    /// Full path of the entry: the root path joined with every name below it.
    #[must_use]
    pub fn path(&self, id: NodeId) -> Option<PathBuf> {
        self.arena.path(id)
    }

    /// Direct children of `id` (order is unspecified; callers sort).
    #[must_use]
    pub fn children(&self, id: NodeId) -> Children<'_> {
        Children {
            tree: self,
            next: self.node(id).map_or(NONE, |n| n.first_child),
        }
    }
}

/// Iterator over the direct children of a node; see [`Tree::children`].
#[derive(Debug, Clone)]
pub struct Children<'a> {
    tree: &'a Tree,
    next: u32,
}

impl Iterator for Children<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let id = opt(self.next)?;
        self.next = self.tree.node(id).map_or(NONE, |n| n.next_sibling);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::{EntryInfo, Node, NodeFlags, NodeKind, TreeBuilder, TreeError};
    use crate::Size;
    use std::ffi::OsStr;
    use std::path::PathBuf;

    fn info(kind: NodeKind, logical: u64, allocated: u64) -> EntryInfo {
        EntryInfo {
            kind,
            size: Size { logical, allocated },
            mtime: 0,
            flags: NodeFlags::NONE,
        }
    }

    #[test]
    fn node_is_at_most_48_bytes() {
        assert!(
            size_of::<Node>() <= 48,
            "Node grew to {} bytes",
            size_of::<Node>()
        );
    }

    #[test]
    fn aggregate_sums_children_bottom_up() {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 4096)).unwrap();
        let root = b.root();
        let sub = b
            .add_child(root, OsStr::new("sub"), info(NodeKind::Dir, 0, 4096))
            .unwrap();
        b.add_child(sub, OsStr::new("a"), info(NodeKind::File, 10, 4096))
            .unwrap();
        b.add_child(sub, OsStr::new("b"), info(NodeKind::File, 20, 4096))
            .unwrap();
        b.add_child(root, OsStr::new("c"), info(NodeKind::File, 5, 4096))
            .unwrap();
        let t = b.finish();

        let s = t.node(sub).unwrap();
        assert_eq!(
            s.size(),
            Size {
                logical: 30,
                allocated: 3 * 4096
            }
        );
        assert_eq!(s.file_count(), 2);
        let r = t.node(t.root()).unwrap();
        assert_eq!(
            r.size(),
            Size {
                logical: 35,
                allocated: 5 * 4096
            }
        );
        assert_eq!(r.file_count(), 3);
    }

    #[test]
    fn children_and_path_walk_the_tree() {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 0)).unwrap();
        let root = b.root();
        let sub = b
            .add_child(root, OsStr::new("sub"), info(NodeKind::Dir, 0, 0))
            .unwrap();
        let f = b
            .add_child(sub, OsStr::new("f.txt"), info(NodeKind::File, 1, 1))
            .unwrap();
        b.add_child(root, OsStr::new("g"), info(NodeKind::File, 1, 1))
            .unwrap();
        let t = b.finish();

        let mut names: Vec<_> = t.children(t.root()).map(|c| t.name(c).unwrap()).collect();
        names.sort();
        assert_eq!(names, ["g", "sub"]);
        assert_eq!(t.children(f).count(), 0);
        assert_eq!(t.node(f).unwrap().parent(), Some(sub));
        assert_eq!(
            t.path(f).unwrap(),
            ["/r", "sub", "f.txt"].iter().collect::<PathBuf>()
        );
        assert_eq!(t.len(), 4);
    }

    #[test]
    fn add_child_rejects_non_directory_parent() {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 0)).unwrap();
        let root = b.root();
        let f = b
            .add_child(root, OsStr::new("f"), info(NodeKind::File, 1, 1))
            .unwrap();
        assert_eq!(
            b.add_child(f, OsStr::new("x"), info(NodeKind::File, 1, 1)),
            Err(TreeError::InvalidParent)
        );
    }

    #[test]
    fn hardlink_mark_zeroes_own_size_only() {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 0)).unwrap();
        let root = b.root();
        let sub = b
            .add_child(root, OsStr::new("sub"), info(NodeKind::Dir, 0, 0))
            .unwrap();
        let a = b
            .add_child(root, OsStr::new("a"), info(NodeKind::File, 10, 4096))
            .unwrap();
        let dup = b
            .add_child(sub, OsStr::new("a2"), info(NodeKind::File, 10, 4096))
            .unwrap();
        assert_eq!(b.mark_hardlink(sub), Err(TreeError::NotAFile));
        b.mark_hardlink(dup).unwrap();
        let t = b.finish();

        let d = t.node(dup).unwrap();
        assert!(d.flags().contains(NodeFlags::HARDLINK));
        assert_eq!(d.size(), Size::default());
        assert_eq!(d.file_count(), 1);
        assert!(!t.node(a).unwrap().flags().contains(NodeFlags::HARDLINK));
        let r = t.node(t.root()).unwrap();
        assert_eq!(r.size().allocated, 4096);
        assert_eq!(r.file_count(), 2);
    }
}
