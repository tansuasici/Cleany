//! Arena tree of scanned entries (see docs/PLAN.md §3.1).
//!
//! Nodes live in one `Vec` and link to each other by `u32` index, names live
//! in a packed pool. Build with [`TreeBuilder`], which only records each
//! entry's *own* size; [`TreeBuilder::finish`] rolls sizes up bottom-up once
//! and yields an immutable [`Tree`]. Children are always appended after their
//! parent, so a single reverse pass over the arena aggregates the whole tree.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};

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
    /// Removed from the tree by [`Tree::detach`] (e.g. moved to the trash).
    /// The node keeps its slot so ids stay stable, but it is no longer
    /// linked to its parent and [`Tree::node_id`] refuses it.
    pub const DETACHED: Self = Self(1 << 2);

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

/// What [`Tree::detach`] took off its ancestors' totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Removed {
    /// Aggregated size of the detached subtree.
    pub size: Size,
    /// Non-directory entries in the detached subtree.
    pub files: u32,
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

    /// The id of the node at arena position `index`, or `None` if out of
    /// range. Inverse of [`NodeId::index`], so ids can cross a process
    /// boundary (e.g. IPC) as plain integers.
    /// Nodes removed by [`Tree::detach`], and nodes below them, are refused.
    #[must_use]
    pub fn node_id(&self, index: u32) -> Option<NodeId> {
        let id = NodeId(index);
        self.is_live(id).then_some(id)
    }

    /// Whether `id` is in this tree and neither it nor an ancestor was detached.
    fn is_live(&self, id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(c) = cur {
            match self.node(c) {
                Some(n) if !n.flags.contains(NodeFlags::DETACHED) => cur = n.parent(),
                _ => return false,
            }
        }
        true
    }

    /// Unlinks `id` from its parent and takes its size and file count off
    /// every ancestor, as if the entry had never been scanned. The slot
    /// stays in the arena (marked [`NodeFlags::DETACHED`]) so other ids keep
    /// their meaning. Sizes of hard links shared with the rest of the tree
    /// are taken as scanned, so the new totals are approximate for them.
    ///
    /// Returns `None` for the root, an id not in this tree, or a node that
    /// is already detached (itself or through an ancestor).
    pub fn detach(&mut self, id: NodeId) -> Option<Removed> {
        if id == self.root() || !self.is_live(id) {
            return None;
        }
        let nodes = &mut self.arena.nodes;
        let (parent, next, removed) = {
            let n = nodes.get(id.index())?;
            let removed = Removed {
                size: n.size,
                files: n.file_count,
            };
            (n.parent, n.next_sibling, removed)
        };
        // Unlink from the parent's sibling chain.
        let first = nodes.get(parent as usize)?.first_child;
        if first == id.0 {
            nodes.get_mut(parent as usize)?.first_child = next;
        } else {
            let mut prev = first;
            while prev != NONE {
                let p = nodes.get_mut(prev as usize)?;
                if p.next_sibling == id.0 {
                    p.next_sibling = next;
                    break;
                }
                prev = p.next_sibling;
            }
        }
        let n = nodes.get_mut(id.index())?;
        n.next_sibling = NONE;
        n.flags = n.flags.union(NodeFlags::DETACHED);
        // Take the subtree's totals off every ancestor.
        let mut cur = parent;
        while let Some(a) = nodes.get_mut(cur as usize) {
            a.size = a.size.saturating_sub(removed.size);
            a.file_count = a.file_count.saturating_sub(removed.files);
            cur = a.parent;
        }
        Some(removed)
    }

    /// The live node at `path`, walking from the root one component at a
    /// time and matching children by name. `path` must start with the root
    /// path exactly as scanned. Detached nodes are never found.
    ///
    /// Each step is linear in the number of siblings.
    #[must_use]
    pub fn lookup(&self, path: &Path) -> Option<NodeId> {
        let root = self.path(self.root())?;
        let rel = path.strip_prefix(&root).ok()?;
        let mut cur = self.root();
        for comp in rel.components() {
            match comp {
                Component::Normal(name) => {
                    cur = self.children(cur).find(|&c| {
                        self.node(c)
                            .is_some_and(|n| self.arena.names.matches(n.name_off, n.name_len, name))
                    })?;
                }
                Component::CurDir => {}
                _ => return None,
            }
        }
        Some(cur)
    }

    /// Sets a non-directory's own size and modification time in place and
    /// moves the size difference onto every ancestor. The id is unchanged.
    /// A [`NodeFlags::HARDLINK`] node keeps its zero size (its bytes are
    /// counted elsewhere); only its time changes.
    ///
    /// Returns whether anything changed.
    ///
    /// # Errors
    /// [`TreeError::NotAFile`] if `id` is a directory, detached or not in
    /// this tree.
    pub fn update_entry(&mut self, id: NodeId, size: Size, mtime: i64) -> Result<bool, TreeError> {
        if !self.is_live(id) {
            return Err(TreeError::NotAFile);
        }
        let n = match self.arena.nodes.get_mut(id.index()) {
            Some(n) if n.kind != NodeKind::Dir => n,
            _ => return Err(TreeError::NotAFile),
        };
        let new = if n.flags.contains(NodeFlags::HARDLINK) {
            n.size
        } else {
            size
        };
        let old = n.size;
        if old == new && n.mtime == mtime {
            return Ok(false);
        }
        n.size = new;
        n.mtime = mtime;
        let mut cur = n.parent;
        while let Some(a) = self.arena.nodes.get_mut(cur as usize) {
            a.size = a.size.saturating_sub(old).saturating_add(new);
            cur = a.parent;
        }
        Ok(true)
    }

    /// Adds a new entry under the live directory `parent` of a finished
    /// tree, at the end of the arena, and adds its size and file count to
    /// every ancestor. Existing ids keep their meaning.
    ///
    /// # Errors
    /// [`TreeError::InvalidParent`] if `parent` is not a live directory,
    /// otherwise a capacity error from the arena or name pool.
    pub fn add_child(
        &mut self,
        parent: NodeId,
        name: &OsStr,
        info: EntryInfo,
    ) -> Result<NodeId, TreeError> {
        self.live_dir(parent)?;
        let raw = self.next_raw(1)?;
        let (name_off, name_len) = self.arena.names.push(name)?;
        let node = new_node(parent.0, name_off, name_len, info);
        let (size, files) = (node.size, node.file_count);
        self.arena.nodes.push(node);
        self.link(parent, raw, raw)?;
        self.add_totals(parent, size, files);
        Ok(NodeId(raw))
    }

    /// Appends the whole of `sub` (e.g. a fresh [`crate::scan`] of a new
    /// directory) as a new child `name` of the live directory `parent`. The
    /// grafted root keeps `sub`'s aggregated totals, which are added to every
    /// ancestor. Existing ids keep their meaning; `sub`'s ids do not carry
    /// over. Hard links are deduplicated only within `sub`.
    ///
    /// # Errors
    /// [`TreeError::InvalidParent`] if `parent` is not a live directory,
    /// otherwise a capacity error.
    pub fn graft(&mut self, parent: NodeId, name: &OsStr, sub: &Tree) -> Result<NodeId, TreeError> {
        let base = self.splice(parent, sub, Some(name))?;
        Ok(NodeId(base))
    }

    /// Like [`Tree::graft`], but `sub`'s root is dropped and its children
    /// become children of `parent` (used to rescan a directory in place, so
    /// its own id survives). Totals of the added children go to `parent` and
    /// every ancestor.
    ///
    /// # Errors
    /// See [`Tree::graft`].
    pub fn graft_children(&mut self, parent: NodeId, sub: &Tree) -> Result<(), TreeError> {
        self.splice(parent, sub, None).map(|_| ())
    }

    /// Appends `sub`'s nodes after the arena, rebasing their links. With
    /// `name`, `sub`'s root becomes one new child of `parent`; without, the
    /// root is skipped and its children are linked under `parent`. Returns
    /// the arena index of the first appended node.
    fn splice(
        &mut self,
        parent: NodeId,
        sub: &Tree,
        name: Option<&OsStr>,
    ) -> Result<u32, TreeError> {
        self.live_dir(parent)?;
        let skip = u32::from(name.is_none());
        let sub_len = u32::try_from(sub.len()).map_err(|_| TreeError::TooManyNodes)?;
        let base = self.next_raw(sub_len.saturating_sub(skip))?;
        // Old index `i` (>= skip) moves to `base + i - skip`; the skipped root
        // maps to `parent`.
        let map = |raw: u32| match raw {
            NONE => NONE,
            r if r < skip => parent.0,
            r => base + (r - skip),
        };
        // Build every node (and its name) before touching the arena, so a
        // capacity error leaves the tree unchanged.
        let mut fresh = Vec::with_capacity(sub.len());
        for (i, n) in sub.arena.nodes.iter().enumerate().skip(skip as usize) {
            let own_name = match (i, name) {
                (0, Some(name)) => name.to_os_string(),
                _ => sub.arena.names.get(n.name_off, n.name_len),
            };
            let (name_off, name_len) = self.arena.names.push(&own_name)?;
            let mut node = n.clone();
            node.parent = if i == 0 { parent.0 } else { map(n.parent) };
            node.first_child = map(n.first_child);
            node.next_sibling = if i == 0 { NONE } else { map(n.next_sibling) };
            node.name_off = name_off;
            node.name_len = name_len;
            fresh.push(node);
        }
        let sub_root = sub.node(sub.root()).ok_or(TreeError::InvalidParent)?;
        let (head, size, files) = if name.is_some() {
            (base, sub_root.size, sub_root.file_count)
        } else {
            let (size, files) = sub
                .children(sub.root())
                .filter_map(|c| sub.node(c))
                .fold((Size::default(), 0u32), |(s, f), n| {
                    (s.saturating_add(n.size), f.saturating_add(n.file_count))
                });
            (map(sub_root.first_child), size, files)
        };
        self.arena.nodes.extend(fresh);
        if head != NONE {
            // The last node of the new sibling chain links to the old head.
            let mut tail = head;
            while let Some(next) = self.node(NodeId(tail)).map(|n| n.next_sibling) {
                if next == NONE {
                    break;
                }
                tail = next;
            }
            self.link(parent, head, tail)?;
            self.add_totals(parent, size, files);
        }
        Ok(base)
    }

    /// `Ok` if `id` is a live directory.
    fn live_dir(&self, id: NodeId) -> Result<(), TreeError> {
        match self.node(id) {
            Some(n) if n.kind == NodeKind::Dir && self.is_live(id) => Ok(()),
            _ => Err(TreeError::InvalidParent),
        }
    }

    /// Arena index of the next node, if `count` more nodes still fit.
    fn next_raw(&self, count: u32) -> Result<u32, TreeError> {
        let raw = u32::try_from(self.arena.nodes.len()).map_err(|_| TreeError::TooManyNodes)?;
        match raw.checked_add(count) {
            Some(end) if end < NONE => Ok(raw),
            _ => Err(TreeError::TooManyNodes),
        }
    }

    /// Prepends the sibling chain `head..=tail` to `parent`'s children.
    fn link(&mut self, parent: NodeId, head: u32, tail: u32) -> Result<(), TreeError> {
        let nodes = &mut self.arena.nodes;
        let p = nodes
            .get_mut(parent.index())
            .ok_or(TreeError::InvalidParent)?;
        let old_first = p.first_child;
        p.first_child = head;
        let t = nodes
            .get_mut(tail as usize)
            .ok_or(TreeError::InvalidParent)?;
        t.next_sibling = old_first;
        Ok(())
    }

    /// Adds `size` and `files` to `from` and every ancestor.
    fn add_totals(&mut self, from: NodeId, size: Size, files: u32) {
        let mut cur = from.0;
        while let Some(a) = self.arena.nodes.get_mut(cur as usize) {
            a.size = a.size.saturating_add(size);
            a.file_count = a.file_count.saturating_add(files);
            cur = a.parent;
        }
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
    use super::{EntryInfo, Node, NodeFlags, NodeKind, Removed, Tree, TreeBuilder, TreeError};
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

    #[test]
    fn node_id_round_trips_index() {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 0)).unwrap();
        let a = b
            .add_child(b.root(), OsStr::new("a"), info(NodeKind::File, 1, 1))
            .unwrap();
        let t = b.finish();
        for id in [t.root(), a] {
            let index = u32::try_from(id.index()).unwrap();
            assert_eq!(t.node_id(index), Some(id));
        }
    }

    #[test]
    fn node_id_rejects_out_of_range() {
        let b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 0)).unwrap();
        let t = b.finish();
        assert_eq!(t.node_id(1), None);
        assert_eq!(t.node_id(u32::MAX), None);
    }

    /// `/r` with `sub/{a 10, b 20}`, `c 5` (logical bytes; 4096 allocated each).
    fn detach_tree() -> (Tree, [super::NodeId; 4]) {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 4096)).unwrap();
        let root = b.root();
        let sub = b
            .add_child(root, OsStr::new("sub"), info(NodeKind::Dir, 0, 4096))
            .unwrap();
        let a = b
            .add_child(sub, OsStr::new("a"), info(NodeKind::File, 10, 4096))
            .unwrap();
        let bb = b
            .add_child(sub, OsStr::new("b"), info(NodeKind::File, 20, 4096))
            .unwrap();
        let c = b
            .add_child(root, OsStr::new("c"), info(NodeKind::File, 5, 4096))
            .unwrap();
        (b.finish(), [sub, a, bb, c])
    }

    #[test]
    fn detach_takes_totals_off_ancestors() {
        let (mut t, [sub, a, _, c]) = detach_tree();
        assert_eq!(
            t.detach(a),
            Some(Removed {
                size: Size {
                    logical: 10,
                    allocated: 4096
                },
                files: 1
            })
        );
        let s = t.node(sub).unwrap();
        assert_eq!(s.size().logical, 20);
        assert_eq!(s.file_count(), 1);
        let r = t.node(t.root()).unwrap();
        assert_eq!(
            r.size(),
            Size {
                logical: 25,
                allocated: 4 * 4096
            }
        );
        assert_eq!(r.file_count(), 2);
        assert!(t.node_id(u32::try_from(c.index()).unwrap()).is_some());
    }

    #[test]
    fn detach_unlinks_from_children() {
        let (mut t, [sub, a, bb, c]) = detach_tree();
        // `bb` is the head of `sub`'s chain (children are prepended), `a` the tail.
        t.detach(bb).unwrap();
        assert_eq!(t.children(sub).collect::<Vec<_>>(), [a]);
        t.detach(a).unwrap();
        assert_eq!(t.children(sub).count(), 0);
        t.detach(sub).unwrap();
        assert_eq!(t.children(t.root()).collect::<Vec<_>>(), [c]);
        let r = t.node(t.root()).unwrap();
        assert_eq!(r.size().logical, 5);
        assert_eq!(r.file_count(), 1);
    }

    #[test]
    fn detach_middle_sibling_keeps_the_rest() {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 0)).unwrap();
        let root = b.root();
        let ids: Vec<_> = ["x", "y", "z"]
            .iter()
            .map(|n| {
                b.add_child(root, OsStr::new(n), info(NodeKind::File, 1, 1))
                    .unwrap()
            })
            .collect();
        let mut t = b.finish();
        t.detach(ids[1]).unwrap();
        let mut left: Vec<_> = t.children(root).collect();
        left.sort();
        assert_eq!(left, [ids[0], ids[2]]);
    }

    #[test]
    fn detach_twice_root_and_descendants_are_refused() {
        let (mut t, [sub, a, _, _]) = detach_tree();
        assert_eq!(t.detach(t.root()), None);
        t.detach(sub).unwrap();
        assert_eq!(t.detach(sub), None);
        assert_eq!(t.detach(a), None, "a node under a detached one");
        for id in [sub, a] {
            assert_eq!(t.node_id(u32::try_from(id.index()).unwrap()), None);
        }
        assert!(t.node(sub).unwrap().flags().contains(NodeFlags::DETACHED));
        assert_eq!(t.node(t.root()).unwrap().file_count(), 1);
    }

    /// `/r` with `sub/{a 10, b 20}` and `c 5`; every entry allocates 4096.
    fn patch_fixture() -> (Tree, super::NodeId, super::NodeId) {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0, 4096)).unwrap();
        let root = b.root();
        let sub = b
            .add_child(root, OsStr::new("sub"), info(NodeKind::Dir, 0, 4096))
            .unwrap();
        let a = b
            .add_child(sub, OsStr::new("a"), info(NodeKind::File, 10, 4096))
            .unwrap();
        b.add_child(sub, OsStr::new("b"), info(NodeKind::File, 20, 4096))
            .unwrap();
        b.add_child(root, OsStr::new("c"), info(NodeKind::File, 5, 4096))
            .unwrap();
        (b.finish(), sub, a)
    }

    #[test]
    fn update_entry_moves_size_delta_to_every_ancestor() {
        let (mut t, sub, a) = patch_fixture();
        let new = Size {
            logical: 110,
            allocated: 8192,
        };
        assert_eq!(t.update_entry(a, new, 7), Ok(true));
        assert_eq!(t.update_entry(a, new, 7), Ok(false));
        assert_eq!(t.node(a).unwrap().size(), new);
        assert_eq!(t.node(a).unwrap().mtime(), 7);
        assert_eq!(t.node(sub).unwrap().size().logical, 130);
        assert_eq!(t.node(sub).unwrap().size().allocated, 4 * 4096);
        let r = t.node(t.root()).unwrap();
        assert_eq!(r.size().logical, 135);
        assert_eq!(r.size().allocated, 6 * 4096);
        assert_eq!(r.file_count(), 3);
        assert_eq!(t.update_entry(sub, new, 0), Err(TreeError::NotAFile));
        t.detach(a).unwrap();
        assert_eq!(t.update_entry(a, new, 0), Err(TreeError::NotAFile));
    }

    #[test]
    fn graft_appends_subtree_and_keeps_existing_ids() {
        let (mut t, sub, a) = patch_fixture();
        let before = t.len();
        // A scanned subtree: `/elsewhere/new` with `x 3` and `deep/y 4`.
        let mut b =
            TreeBuilder::new(OsStr::new("/elsewhere/new"), info(NodeKind::Dir, 0, 4096)).unwrap();
        let sr = b.root();
        b.add_child(sr, OsStr::new("x"), info(NodeKind::File, 3, 4096))
            .unwrap();
        let deep = b
            .add_child(sr, OsStr::new("deep"), info(NodeKind::Dir, 0, 4096))
            .unwrap();
        b.add_child(deep, OsStr::new("y"), info(NodeKind::File, 4, 4096))
            .unwrap();
        let sub_tree = b.finish();

        let new = t.graft(sub, OsStr::new("new"), &sub_tree).unwrap();
        assert_eq!(t.len(), before + 4);
        assert_eq!(t.name(a).unwrap(), "a");
        assert_eq!(t.node(new).unwrap().parent(), Some(sub));
        assert_eq!(
            t.lookup(&PathBuf::from("/r/sub/new/deep/y")).map(|id| t
                .node(id)
                .unwrap()
                .size()
                .logical),
            Some(4)
        );
        assert_eq!(t.node(sub).unwrap().size().logical, 37);
        assert_eq!(t.node(sub).unwrap().file_count(), 4);
        assert_eq!(t.node(t.root()).unwrap().size().logical, 42);

        // Grafting only the children merges them into an existing dir.
        let c = t.lookup(&PathBuf::from("/r/sub/new")).unwrap();
        for child in t.children(c).collect::<Vec<_>>() {
            t.detach(child).unwrap();
        }
        assert_eq!(t.node(c).unwrap().size().logical, 0);
        t.graft_children(c, &sub_tree).unwrap();
        let mut names: Vec<_> = t.children(c).map(|id| t.name(id).unwrap()).collect();
        names.sort();
        assert_eq!(names, ["deep", "x"]);
        assert_eq!(t.node(c).unwrap().size().logical, 7);
        assert_eq!(t.node(c).unwrap().size().allocated, 4 * 4096);
        assert_eq!(t.node(t.root()).unwrap().size().logical, 42);
        assert_eq!(t.node(t.root()).unwrap().file_count(), 5);

        let leaf = t
            .add_child(c, OsStr::new("z"), info(NodeKind::File, 1, 4096))
            .unwrap();
        assert_eq!(t.node(leaf).unwrap().parent(), Some(c));
        assert_eq!(t.node(t.root()).unwrap().size().logical, 43);
        assert_eq!(
            t.add_child(leaf, OsStr::new("q"), info(NodeKind::File, 1, 1)),
            Err(TreeError::InvalidParent)
        );
    }

    #[test]
    fn lookup_finds_node_by_path_and_refuses_detached() {
        let (mut t, sub, a) = patch_fixture();
        assert_eq!(t.lookup(&PathBuf::from("/r")), Some(t.root()));
        assert_eq!(t.lookup(&PathBuf::from("/r/sub")), Some(sub));
        assert_eq!(t.lookup(&PathBuf::from("/r/sub/a")), Some(a));
        assert_eq!(t.lookup(&PathBuf::from("/r/./sub/a")), Some(a));
        assert_eq!(t.lookup(&PathBuf::from("/r/sub/zzz")), None);
        assert_eq!(t.lookup(&PathBuf::from("/r/sub/../c")), None);
        assert_eq!(t.lookup(&PathBuf::from("/other/sub")), None);
        t.detach(sub).unwrap();
        assert_eq!(t.lookup(&PathBuf::from("/r/sub")), None);
        assert_eq!(t.lookup(&PathBuf::from("/r/sub/a")), None);
    }
}
