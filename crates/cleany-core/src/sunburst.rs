//! Sunburst layout of one folder, a few rings deep.
//!
//! Like [`mod@crate::treemap`], the UI gets only what it draws: the arcs of
//! one folder's subtree, a few levels deep. Each folder's children split the
//! folder's own span of the circle by size, largest first, clockwise. Angles
//! are in turns (0 to 1), so the layout does not depend on the canvas size.
//! Children too thin to see are merged into one "others" arc, so every ring
//! still adds up to its parent.

use crate::tree::{NodeFlags, NodeId, NodeKind, Tree};
use crate::treemap::CellTarget;

/// Inputs of [`sunburst`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunburstOptions {
    /// Rings to lay out around the root; 0 lays out nothing.
    pub depth: u8,
    /// Children whose span would be narrower than this (in turns) are merged
    /// into one [`CellTarget::Others`] arc.
    pub min_angle: f32,
    /// Most arcs returned. The first ring is cut to fit; a deeper ring that
    /// would not fit is left out whole.
    pub max_arcs: usize,
}

/// One laid-out arc of a ring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunburstArc {
    /// The node, or the merged small children.
    pub target: CellTarget,
    /// The folder this arc lies in.
    pub parent: NodeId,
    /// 1 for the root's children (the inner ring), 2 for theirs, and so on.
    pub depth: u8,
    /// Where the arc starts, in turns clockwise from 12 o'clock.
    pub start: f32,
    /// Where the arc ends, in turns; never less than `start`.
    pub end: f32,
    /// Allocated bytes the arc's span stands for.
    pub allocated: u64,
}

/// Lays out `root`'s subtree as sunburst rings.
///
/// Arcs come ring by ring, so every parent precedes its children; the root
/// itself is not an arc (it is the whole circle). Only allocated bytes count:
/// empty entries and extra hard links (whose bytes are on another node) get
/// no arc.
#[must_use]
pub fn sunburst(tree: &Tree, root: NodeId, opts: &SunburstOptions) -> Vec<SunburstArc> {
    let mut out = Vec::new();
    if opts.max_arcs == 0 || tree.node(root).is_none() {
        return out;
    }
    let mut frontier = vec![(root, 0.0_f64, 1.0_f64)];
    for depth in 1..=opts.depth {
        let mut ring = Vec::new();
        for &(parent, start, end) in &frontier {
            let limit = if depth == 1 {
                opts.max_arcs
            } else {
                usize::MAX
            };
            split(
                tree,
                parent,
                depth,
                start,
                end,
                opts.min_angle,
                limit,
                &mut ring,
            );
        }
        if ring.is_empty() || out.len().saturating_add(ring.len()) > opts.max_arcs {
            break;
        }
        frontier = ring
            .iter()
            .filter_map(|a| match a.target {
                CellTarget::Node(id) if is_open_dir(tree, id) => {
                    Some((id, f64::from(a.start), f64::from(a.end)))
                }
                _ => None,
            })
            .collect();
        out.append(&mut ring);
        if frontier.is_empty() {
            break;
        }
    }
    out
}

fn is_open_dir(tree: &Tree, id: NodeId) -> bool {
    tree.node(id).is_some_and(|n| n.kind() == NodeKind::Dir) && tree.children(id).next().is_some()
}

/// Splits `start..end` among `parent`'s visible children, at most `limit` arcs.
#[allow(clippy::too_many_arguments)]
fn split(
    tree: &Tree,
    parent: NodeId,
    depth: u8,
    start: f64,
    end: f64,
    min_angle: f32,
    limit: usize,
    out: &mut Vec<SunburstArc>,
) {
    let mut kids: Vec<(NodeId, u64)> = tree
        .children(parent)
        .filter_map(|id| {
            let n = tree.node(id)?;
            let allocated = n.size().allocated;
            (allocated > 0 && !n.flags().contains(NodeFlags::HARDLINK)).then_some((id, allocated))
        })
        .collect();
    if kids.is_empty() || limit == 0 || end <= start {
        return;
    }
    kids.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let total: u128 = kids.iter().map(|&(_, a)| u128::from(a)).sum();
    let span = end - start;
    let share = |a: u64| to_f64(a) / to_f64_u128(total) * span;

    // Sorted largest first, so the children wide enough to see are a prefix.
    let mut keep = kids
        .iter()
        .take_while(|&&(_, a)| share(a) >= f64::from(min_angle))
        .count();
    if kids.len() - keep == 1 {
        keep += 1;
    }
    if keep + usize::from(keep < kids.len()) > limit {
        keep = limit - 1;
    }

    let mut targets: Vec<(CellTarget, u64)> = kids[..keep]
        .iter()
        .map(|&(id, a)| (CellTarget::Node(id), a))
        .collect();
    let tail = &kids[keep..];
    if !tail.is_empty() {
        let allocated = tail.iter().fold(0u64, |s, &(_, a)| s.saturating_add(a));
        let count = u32::try_from(tail.len()).unwrap_or(u32::MAX);
        targets.push((CellTarget::Others { count, allocated }, allocated));
    }

    // Running sums, so rounding never opens a gap or pushes past `end`.
    let last = targets.len() - 1;
    let mut at = start;
    let mut sum: u128 = 0;
    for (i, (target, allocated)) in targets.into_iter().enumerate() {
        sum += u128::from(allocated);
        let next = if i == last {
            end
        } else {
            start + to_f64_u128(sum) / to_f64_u128(total) * span
        };
        out.push(SunburstArc {
            target,
            parent,
            depth,
            start: to_f32(at),
            end: to_f32(next),
            allocated,
        });
        at = next;
    }
}

// Byte counts only set proportions, so losing low bits is harmless.
const fn to_f64(v: u64) -> f64 {
    v as f64
}

const fn to_f64_u128(v: u128) -> f64 {
    v as f64
}

// Turns fit f32 with room to spare.
const fn to_f32(v: f64) -> f32 {
    v as f32
}

#[cfg(test)]
mod tests {
    use super::{SunburstArc, SunburstOptions, sunburst};
    use crate::Size;
    use crate::tree::{EntryInfo, NodeFlags, NodeId, NodeKind, Tree, TreeBuilder};
    use crate::treemap::CellTarget;
    use std::ffi::OsStr;

    const EPS: f32 = 1e-5;

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

    fn opts(depth: u8) -> SunburstOptions {
        SunburstOptions {
            depth,
            min_angle: 0.0,
            max_arcs: 5_000,
        }
    }

    /// A root holding one file per size.
    fn flat(sizes: &[u64]) -> Tree {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0)).unwrap();
        let root = b.root();
        for (i, &s) in sizes.iter().enumerate() {
            b.add_child(root, OsStr::new(&format!("f{i}")), info(NodeKind::File, s))
                .unwrap();
        }
        b.finish()
    }

    fn span(a: &SunburstArc) -> f32 {
        a.end - a.start
    }

    /// `/r/{x,y}/{0..n}`: two folders of equal files, plus a loose file.
    fn two_dirs(n: usize) -> (Tree, NodeId) {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0)).unwrap();
        let root = b.root();
        for (d, size) in [("x", 30), ("y", 10)] {
            let dir = b
                .add_child(root, OsStr::new(d), info(NodeKind::Dir, 0))
                .unwrap();
            for i in 0..n {
                b.add_child(dir, OsStr::new(&format!("{i}")), info(NodeKind::File, size))
                    .unwrap();
            }
        }
        b.add_child(root, OsStr::new("loose"), info(NodeKind::File, 40))
            .unwrap();
        (b.finish(), root)
    }

    #[test]
    fn children_split_parent_span_by_size() {
        let tree = flat(&[600, 300, 100]);
        let arcs = sunburst(&tree, tree.root(), &opts(1));
        assert_eq!(arcs.len(), 3);
        assert!((arcs[0].start).abs() < EPS);
        for (a, want) in arcs.iter().zip([0.6, 0.3, 0.1]) {
            assert!((span(a) - want).abs() < EPS, "{a:?}");
        }
        // Consecutive, no gaps, ending at a full turn.
        for w in arcs.windows(2) {
            assert!((w[0].end - w[1].start).abs() < EPS);
        }
        assert!((arcs[2].end - 1.0).abs() < EPS);
        // Largest first.
        assert_eq!(
            arcs.iter().map(|a| a.allocated).collect::<Vec<_>>(),
            [600, 300, 100]
        );
    }

    #[test]
    fn level_spans_add_up_to_parent() {
        let (tree, root) = two_dirs(4);
        let arcs = sunburst(&tree, root, &opts(2));
        for p in arcs.iter().filter(|a| a.depth == 1) {
            let CellTarget::Node(id) = p.target else {
                continue;
            };
            let kids: Vec<&SunburstArc> = arcs.iter().filter(|a| a.parent == id).collect();
            if kids.is_empty() {
                continue;
            }
            let total: f32 = kids.iter().map(|a| span(a)).sum();
            assert!((total - span(p)).abs() < EPS, "{p:?}");
            assert!((kids[0].start - p.start).abs() < EPS);
            assert!((kids[kids.len() - 1].end - p.end).abs() < EPS);
            assert!(
                kids.iter()
                    .all(|k| k.start >= p.start - EPS && k.end <= p.end + EPS)
            );
        }
    }

    #[test]
    fn parents_precede_children() {
        let (tree, root) = two_dirs(3);
        let arcs = sunburst(&tree, root, &opts(3));
        for (i, a) in arcs.iter().enumerate() {
            if a.parent == root {
                assert_eq!(a.depth, 1);
                continue;
            }
            let p = arcs[..i]
                .iter()
                .find(|p| p.target == CellTarget::Node(a.parent))
                .unwrap();
            assert_eq!(p.depth + 1, a.depth);
        }
    }

    #[test]
    fn small_children_merge_into_others() {
        let tree = flat(&[1_000_000, 500_000, 250_000, 1, 1, 1, 1, 1]);
        let o = SunburstOptions {
            min_angle: 0.01,
            ..opts(1)
        };
        let arcs = sunburst(&tree, tree.root(), &o);
        assert_eq!(arcs.len(), 4);
        assert_eq!(
            arcs[3].target,
            CellTarget::Others {
                count: 5,
                allocated: 5
            }
        );
        assert!((arcs[3].end - 1.0).abs() < EPS);
        let sum: f32 = arcs.iter().map(span).sum();
        assert!((sum - 1.0).abs() < EPS);

        // All specks: one arc fills the circle.
        let tree = flat(&[1; 200]);
        let arcs = sunburst(&tree, tree.root(), &o);
        assert_eq!(arcs.len(), 1);
        assert_eq!(
            arcs[0].target,
            CellTarget::Others {
                count: 200,
                allocated: 200
            }
        );
        assert!((span(&arcs[0]) - 1.0).abs() < EPS);
    }

    #[test]
    fn single_small_child_is_not_merged() {
        let tree = flat(&[1_000_000, 1]);
        let o = SunburstOptions {
            min_angle: 0.01,
            ..opts(1)
        };
        let arcs = sunburst(&tree, tree.root(), &o);
        assert_eq!(arcs.len(), 2);
        assert!(arcs.iter().all(|a| matches!(a.target, CellTarget::Node(_))));
    }

    #[test]
    fn depth_limit_is_respected() {
        // `/r/a/b/c/leaf` plus a sibling file at each level.
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0)).unwrap();
        let mut dir = b.root();
        for name in ["a", "b", "c"] {
            b.add_child(
                dir,
                OsStr::new(&format!("{name}.txt")),
                info(NodeKind::File, 100),
            )
            .unwrap();
            dir = b
                .add_child(dir, OsStr::new(name), info(NodeKind::Dir, 0))
                .unwrap();
        }
        b.add_child(dir, OsStr::new("leaf"), info(NodeKind::File, 400))
            .unwrap();
        let tree = b.finish();
        for depth in 0..=6u8 {
            let arcs = sunburst(&tree, tree.root(), &opts(depth));
            let max = arcs.iter().map(|a| a.depth).max().unwrap_or(0);
            assert_eq!(max, depth.min(4), "depth {depth}");
        }
    }

    #[test]
    fn max_arcs_is_respected() {
        // The first ring is cut, with the rest in one arc.
        let sizes: Vec<u64> = (1..=100).map(|i| i * 10).collect();
        let tree = flat(&sizes);
        let o = SunburstOptions {
            max_arcs: 10,
            ..opts(1)
        };
        let arcs = sunburst(&tree, tree.root(), &o);
        assert_eq!(arcs.len(), 10);
        assert!(matches!(
            arcs[9].target,
            CellTarget::Others { count: 91, .. }
        ));
        assert!((arcs[9].end - 1.0).abs() < EPS);

        // A deeper ring that does not fit is left out whole.
        let (tree, root) = two_dirs(10);
        let mut o = opts(2);
        o.max_arcs = 15;
        assert_eq!(sunburst(&tree, root, &o).len(), 3);
        o.max_arcs = 23;
        assert_eq!(sunburst(&tree, root, &o).len(), 23);
    }

    #[test]
    fn zero_and_hardlink_nodes_are_skipped() {
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0)).unwrap();
        let root = b.root();
        let big = b
            .add_child(root, OsStr::new("big"), info(NodeKind::File, 100))
            .unwrap();
        b.add_child(root, OsStr::new("empty"), info(NodeKind::File, 0))
            .unwrap();
        b.add_child(root, OsStr::new("emptydir"), info(NodeKind::Dir, 0))
            .unwrap();
        let link = b
            .add_child(root, OsStr::new("link"), info(NodeKind::File, 100))
            .unwrap();
        b.mark_hardlink(link).unwrap();
        let tree = b.finish();
        let arcs = sunburst(&tree, root, &opts(2));
        assert_eq!(arcs.len(), 1);
        assert_eq!(arcs[0].target, CellTarget::Node(big));
        assert!((span(&arcs[0]) - 1.0).abs() < EPS);
    }

    #[test]
    fn empty_folder_has_no_arcs() {
        let tree = flat(&[]);
        assert!(sunburst(&tree, tree.root(), &opts(3)).is_empty());
        let tree = flat(&[5]);
        assert!(sunburst(&tree, tree.root(), &opts(0)).is_empty());
    }
}
