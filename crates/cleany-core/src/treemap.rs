//! Squarified treemap layout of one folder, a few levels deep.
//!
//! The UI never gets the whole tree (docs/PLAN.md §3.4); it asks for the
//! cells of one folder within a pixel box. [`treemap`] lays out that
//! folder's children with the squarified algorithm (Bruls, Huizing, van
//! Wijk), then each folder cell's children inside it, level by level.
//! Children too small to see are merged into one "others" cell, so every
//! level still adds up to its parent.

use crate::tree::{NodeFlags, NodeId, NodeKind, Tree};

/// An axis-aligned box in the caller's units (CSS pixels for the UI).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

/// Inputs of [`treemap`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreemapOptions {
    /// Width of the box to fill.
    pub width: f32,
    /// Height of the box to fill.
    pub height: f32,
    /// Levels to lay out below the root; 0 lays out nothing.
    pub depth: u8,
    /// Children whose area would be smaller than this (units²) are merged
    /// into one [`CellTarget::Others`] cell.
    pub min_area: f32,
    /// Most cells returned. The first level is cut to fit; a deeper level
    /// that would not fit is left out whole.
    pub max_cells: usize,
    /// Inset of a folder's children inside the folder's own cell (levels 2+).
    pub padding: f32,
}

/// What a [`Cell`] stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellTarget {
    /// One node of the tree.
    Node(NodeId),
    /// Several small children of the same parent, merged.
    Others {
        /// How many children were merged.
        count: u32,
        /// Their allocated bytes together.
        allocated: u64,
    },
}

/// One laid-out box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// The node, or the merged small children.
    pub target: CellTarget,
    /// The folder this cell lies in.
    pub parent: NodeId,
    /// 1 for the root's children, 2 for theirs, and so on.
    pub depth: u8,
    /// Where the cell is, in the same box as [`TreemapOptions`].
    pub rect: Rect,
    /// Allocated bytes the cell's area stands for.
    pub allocated: u64,
}

/// Lays out `root`'s subtree as a squarified treemap.
///
/// Cells come level by level, so every parent precedes its children; the
/// root itself is not a cell. Only allocated bytes count: empty entries and
/// extra hard links (whose bytes are on another node) get no cell.
#[must_use]
pub fn treemap(tree: &Tree, root: NodeId, opts: &TreemapOptions) -> Vec<Cell> {
    let mut out = Vec::new();
    if !(opts.width > 0.0 && opts.height > 0.0) || opts.max_cells == 0 || tree.node(root).is_none()
    {
        return out;
    }
    let full = Rect {
        x: 0.0,
        y: 0.0,
        w: opts.width,
        h: opts.height,
    };
    let mut frontier = vec![(root, full)];
    for depth in 1..=opts.depth {
        let mut level = Vec::new();
        for &(parent, rect) in &frontier {
            let inner = if depth == 1 {
                rect
            } else {
                inset(rect, opts.padding)
            };
            if !(inner.w > 0.0 && inner.h > 0.0) {
                continue;
            }
            let limit = if depth == 1 {
                opts.max_cells
            } else {
                usize::MAX
            };
            lay_out_children(tree, parent, depth, inner, opts.min_area, limit, &mut level);
        }
        if level.is_empty() || out.len().saturating_add(level.len()) > opts.max_cells {
            break;
        }
        frontier = level
            .iter()
            .filter_map(|c| match c.target {
                CellTarget::Node(id) if is_open_dir(tree, id) => Some((id, c.rect)),
                _ => None,
            })
            .collect();
        out.append(&mut level);
        if frontier.is_empty() {
            break;
        }
    }
    out
}

fn is_open_dir(tree: &Tree, id: NodeId) -> bool {
    tree.node(id).is_some_and(|n| n.kind() == NodeKind::Dir) && tree.children(id).next().is_some()
}

fn inset(r: Rect, by: f32) -> Rect {
    let by = by.max(0.0);
    Rect {
        x: r.x + by,
        y: r.y + by,
        w: (r.w - 2.0 * by).max(0.0),
        h: (r.h - 2.0 * by).max(0.0),
    }
}

/// Lays out `parent`'s visible children inside `inner`, at most `limit` cells.
fn lay_out_children(
    tree: &Tree,
    parent: NodeId,
    depth: u8,
    inner: Rect,
    min_area: f32,
    limit: usize,
    out: &mut Vec<Cell>,
) {
    let mut kids: Vec<(NodeId, u64)> = tree
        .children(parent)
        .filter_map(|id| {
            let n = tree.node(id)?;
            let allocated = n.size().allocated;
            (allocated > 0 && !n.flags().contains(NodeFlags::HARDLINK)).then_some((id, allocated))
        })
        .collect();
    if kids.is_empty() || limit == 0 {
        return;
    }
    kids.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let total: u128 = kids.iter().map(|&(_, a)| u128::from(a)).sum();
    let area = f64::from(inner.w) * f64::from(inner.h);
    let share = |a: u64| to_f64(a) / to_f64_u128(total) * area;

    // Sorted largest first, so the children big enough to see are a prefix.
    let mut keep = kids
        .iter()
        .take_while(|&&(_, a)| share(a) >= f64::from(min_area))
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

    let areas: Vec<f64> = targets.iter().map(|&(_, a)| share(a)).collect();
    let rects = squarify(&areas, inner);
    out.extend(
        targets
            .into_iter()
            .zip(rects)
            .map(|((target, allocated), rect)| Cell {
                target,
                parent,
                depth,
                rect,
                allocated,
            }),
    );
}

// Byte counts only set proportions, so losing low bits is harmless.
const fn to_f64(v: u64) -> f64 {
    v as f64
}

const fn to_f64_u128(v: u128) -> f64 {
    v as f64
}

// Pixel coordinates fit f32.
const fn to_f32(v: f64) -> f32 {
    v as f32
}

/// Worst aspect ratio of a row of areas summing to `sum` laid along `side`.
fn worst(sum: f64, max: f64, min: f64, side: f64) -> f64 {
    if sum <= 0.0 || min <= 0.0 || side <= 0.0 {
        return f64::INFINITY;
    }
    let side2 = side * side;
    let sum2 = sum * sum;
    (side2 * max / sum2).max(sum2 / (side2 * min))
}

/// Splits `rect` into one box per area, in order. The areas should add up
/// to the rect's area; they are laid out in rows along the shorter side.
fn squarify(areas: &[f64], rect: Rect) -> Vec<Rect> {
    let (mut x, mut y) = (f64::from(rect.x), f64::from(rect.y));
    let (mut w, mut h) = (f64::from(rect.w), f64::from(rect.h));
    let mut out = Vec::with_capacity(areas.len());
    let mut i = 0;
    while i < areas.len() {
        let side = w.min(h);
        let first = areas[i].max(0.0);
        let (mut sum, mut max, mut min) = (first, first, first);
        let mut j = i + 1;
        while let Some(&next) = areas.get(j) {
            let next = next.max(0.0);
            let (s, hi, lo) = (sum + next, max.max(next), min.min(next));
            if worst(s, hi, lo, side) > worst(sum, max, min, side) {
                break;
            }
            (sum, max, min) = (s, hi, lo);
            j += 1;
        }
        let last = j == areas.len();
        if w >= h {
            // A column on the left, as wide as the row's area needs.
            let cw = if last || h <= 0.0 {
                w
            } else {
                (sum / h).min(w)
            };
            let mut cy = y;
            for (k, &a) in areas[i..j].iter().enumerate() {
                let ch = if i + k + 1 == j {
                    y + h - cy
                } else if cw > 0.0 {
                    a.max(0.0) / cw
                } else {
                    0.0
                };
                out.push(rect_of(x, cy, cw, ch));
                cy += ch;
            }
            x += cw;
            w = (w - cw).max(0.0);
        } else {
            // A row on top, as tall as the row's area needs.
            let rh = if last || w <= 0.0 {
                h
            } else {
                (sum / w).min(h)
            };
            let mut cx = x;
            for (k, &a) in areas[i..j].iter().enumerate() {
                let cw = if i + k + 1 == j {
                    x + w - cx
                } else if rh > 0.0 {
                    a.max(0.0) / rh
                } else {
                    0.0
                };
                out.push(rect_of(cx, y, cw, rh));
                cx += cw;
            }
            y += rh;
            h = (h - rh).max(0.0);
        }
        i = j;
    }
    out
}

fn rect_of(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect {
        x: to_f32(x),
        y: to_f32(y),
        w: to_f32(w.max(0.0)),
        h: to_f32(h.max(0.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::{Cell, CellTarget, Rect, TreemapOptions, squarify, treemap};
    use crate::Size;
    use crate::tree::{EntryInfo, NodeFlags, NodeId, NodeKind, Tree, TreeBuilder};
    use std::ffi::OsStr;

    const EPS: f32 = 1e-2;

    /// Every tree's root is its first node.
    fn root0() -> NodeId {
        flat(&[]).root()
    }

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

    fn opts(w: f32, h: f32, depth: u8) -> TreemapOptions {
        TreemapOptions {
            width: w,
            height: h,
            depth,
            min_area: 1.0,
            max_cells: 5_000,
            padding: 1.0,
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

    fn area(r: Rect) -> f32 {
        r.w * r.h
    }

    fn level(cells: &[Cell], depth: u8) -> Vec<Cell> {
        cells.iter().copied().filter(|c| c.depth == depth).collect()
    }

    fn assert_inside_disjoint(rects: &[Rect], bound: Rect) {
        for (i, a) in rects.iter().enumerate() {
            assert!(
                a.x >= bound.x - EPS && a.y >= bound.y - EPS,
                "{a:?} outside {bound:?}"
            );
            assert!(
                a.x + a.w <= bound.x + bound.w + EPS,
                "{a:?} outside {bound:?}"
            );
            assert!(
                a.y + a.h <= bound.y + bound.h + EPS,
                "{a:?} outside {bound:?}"
            );
            for b in &rects[i + 1..] {
                let ox = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
                let oy = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
                assert!(ox <= EPS || oy <= EPS, "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn squarify_preserves_total_area() {
        let sizes = [500, 300, 120, 80, 60, 30, 7, 3];
        let cells = treemap(&flat(&sizes), root0(), &opts(640.0, 400.0, 1));
        assert_eq!(cells.len(), sizes.len());
        let sum: f32 = cells.iter().map(|c| area(c.rect)).sum();
        assert!((sum - 640.0 * 400.0).abs() < 1.0, "{sum}");
        let total: u64 = sizes.iter().sum();
        for c in &cells {
            let want = c.allocated as f32 / total as f32 * 640.0 * 400.0;
            assert!((area(c.rect) - want).abs() < 1.0, "{c:?} want {want}");
        }

        let raw = squarify(
            &[6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0],
            Rect {
                x: 0.0,
                y: 0.0,
                w: 6.0,
                h: 4.0,
            },
        );
        let sum: f32 = raw.iter().map(|&r| area(r)).sum();
        assert!((sum - 24.0).abs() < EPS);
    }

    #[test]
    fn squarify_cells_stay_inside_and_do_not_overlap() {
        let sizes: Vec<u64> = (1..=40).map(|i| i * i * 13).collect();
        for (w, h) in [(640.0, 400.0), (300.0, 900.0), (17.0, 3.0)] {
            let cells = treemap(&flat(&sizes), root0(), &opts(w, h, 1));
            let rects: Vec<Rect> = cells.iter().map(|c| c.rect).collect();
            assert_inside_disjoint(
                &rects,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h,
                },
            );
        }
    }

    #[test]
    fn squarify_aspect_ratio_is_bounded() {
        let cells = treemap(&flat(&[10; 16]), root0(), &opts(400.0, 400.0, 1));
        assert_eq!(cells.len(), 16);
        for c in &cells {
            let ratio = (c.rect.w / c.rect.h).max(c.rect.h / c.rect.w);
            assert!(ratio <= 2.0 + EPS, "{c:?} ratio {ratio}");
        }
    }

    #[test]
    fn small_children_merge_into_others() {
        let mut o = opts(100.0, 100.0, 1);
        o.min_area = 24.0;

        // Three big files and five specks: the specks become one cell.
        let cells = treemap(
            &flat(&[1_000_000, 500_000, 250_000, 1, 1, 1, 1, 1]),
            root0(),
            &o,
        );
        assert_eq!(cells.len(), 4);
        assert_eq!(
            cells[3].target,
            CellTarget::Others {
                count: 5,
                allocated: 5
            }
        );
        let sum: f32 = cells.iter().map(|c| area(c.rect)).sum();
        assert!((sum - 10_000.0).abs() < 1.0);

        // A single speck stays itself.
        let cells = treemap(&flat(&[1_000_000, 1]), root0(), &o);
        assert_eq!(cells.len(), 2);
        assert!(
            cells
                .iter()
                .all(|c| matches!(c.target, CellTarget::Node(_)))
        );

        // All specks: one cell fills the parent.
        let cells = treemap(
            &flat(&[1, 1, 1]),
            root0(),
            &opts(4.0, 4.0, 1).with_min(24.0),
        );
        assert_eq!(cells.len(), 1);
        assert_eq!(
            cells[0].target,
            CellTarget::Others {
                count: 3,
                allocated: 3
            }
        );
        assert!((area(cells[0].rect) - 16.0).abs() < EPS);
    }

    impl TreemapOptions {
        fn with_min(mut self, min_area: f32) -> Self {
            self.min_area = min_area;
            self
        }
    }

    /// `/r/a/b/c/file` plus a sibling file at each level.
    fn deep() -> Tree {
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
        b.finish()
    }

    #[test]
    fn depth_limit_is_respected() {
        let tree = deep();
        for depth in 0..=5u8 {
            let cells = treemap(&tree, root0(), &opts(400.0, 400.0, depth));
            let max = cells.iter().map(|c| c.depth).max().unwrap_or(0);
            assert_eq!(max, depth.min(4), "depth {depth}");
        }
        let cells = treemap(&tree, root0(), &opts(400.0, 400.0, 3));
        // Parents come before their children, and children sit inside them.
        for (i, c) in cells.iter().enumerate() {
            if c.parent == root0() {
                assert_eq!(c.depth, 1);
                continue;
            }
            let p = cells[..i]
                .iter()
                .find(|p| p.target == CellTarget::Node(c.parent))
                .unwrap();
            assert_eq!(p.depth + 1, c.depth);
            assert_inside_disjoint(&[c.rect], p.rect);
        }
        for d in 2..=3 {
            let rects: Vec<Rect> = level(&cells, d).iter().map(|c| c.rect).collect();
            assert_inside_disjoint(
                &rects,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 400.0,
                    h: 400.0,
                },
            );
        }
    }

    #[test]
    fn max_cells_is_respected() {
        // The first level is cut, with the rest in one cell.
        let sizes: Vec<u64> = (1..=100).map(|i| i * 10).collect();
        let mut o = opts(800.0, 800.0, 1);
        o.max_cells = 10;
        let cells = treemap(&flat(&sizes), root0(), &o);
        assert_eq!(cells.len(), 10);
        assert!(matches!(
            cells[9].target,
            CellTarget::Others { count: 91, .. }
        ));
        let sum: f32 = cells.iter().map(|c| area(c.rect)).sum();
        assert!((sum - 640_000.0).abs() < 10.0);

        // A deeper level that does not fit is left out whole.
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0)).unwrap();
        let root = b.root();
        for d in ["x", "y"] {
            let dir = b
                .add_child(root, OsStr::new(d), info(NodeKind::Dir, 0))
                .unwrap();
            for i in 0..10 {
                b.add_child(dir, OsStr::new(&format!("{i}")), info(NodeKind::File, 50))
                    .unwrap();
            }
        }
        let tree = b.finish();
        let mut o = opts(800.0, 800.0, 2);
        o.max_cells = 15;
        assert_eq!(treemap(&tree, root, &o).len(), 2);
        o.max_cells = 22;
        assert_eq!(treemap(&tree, root, &o).len(), 22);
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
        let cells = treemap(&tree, root, &opts(100.0, 100.0, 2));
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].target, CellTarget::Node(big));
        assert!((area(cells[0].rect) - 10_000.0).abs() < EPS);
    }

    #[test]
    fn empty_and_single_child_dirs() {
        let empty = flat(&[]);
        assert!(treemap(&empty, root0(), &opts(100.0, 100.0, 3)).is_empty());

        let one = flat(&[42]);
        let cells = treemap(&one, root0(), &opts(120.0, 80.0, 3));
        assert_eq!(cells.len(), 1);
        assert_eq!(
            cells[0].rect,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 120.0,
                h: 80.0
            }
        );
        assert_eq!(cells[0].allocated, 42);

        // No room, unknown root, or no cells allowed: nothing.
        assert!(treemap(&one, root0(), &opts(0.0, 80.0, 3)).is_empty());
        assert!(treemap(&one, root0(), &opts(f32::NAN, 80.0, 3)).is_empty());
        assert!(
            treemap(
                &one,
                flat(&[1; 4]).node_id(4).unwrap(),
                &opts(120.0, 80.0, 3)
            )
            .is_empty()
        );
        let mut o = opts(120.0, 80.0, 3);
        o.max_cells = 0;
        assert!(treemap(&one, root0(), &o).is_empty());

        // A folder holding one file: the file fills the folder, inset by the padding.
        let mut b = TreeBuilder::new(OsStr::new("/r"), info(NodeKind::Dir, 0)).unwrap();
        let d = b
            .add_child(b.root(), OsStr::new("d"), info(NodeKind::Dir, 0))
            .unwrap();
        b.add_child(d, OsStr::new("f"), info(NodeKind::File, 9))
            .unwrap();
        let cells = treemap(&b.finish(), root0(), &opts(50.0, 50.0, 2));
        assert_eq!(cells.len(), 2);
        assert_eq!(
            cells[1].rect,
            Rect {
                x: 1.0,
                y: 1.0,
                w: 48.0,
                h: 48.0
            }
        );
    }
}
