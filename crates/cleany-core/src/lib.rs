//! Cleany core: scan data model, aggregation and analysis.
//!
//! The arena tree lives in [`tree`], the parallel scanner in [`scan`]
//! (see docs/PLAN.md §3), live in-place updates of a finished scan in
//! [`refresh`], the treemap layout in [`mod@treemap`] and the
//! sunburst layout in [`mod@sunburst`].

mod firmlinks;
pub mod largest;
mod names;
pub mod refresh;
pub mod scan;
pub mod sunburst;
pub mod tree;
pub mod treemap;

pub use largest::{largest_dirs, largest_files};
pub use refresh::{
    Applied, Change, Coalescer, Depth, DirRequest, DirUpdate, Outcome, RefreshPlan, refresh_dir,
};
pub use scan::{
    CancelToken, PROGRESS_INTERVAL, Progress, ScanError, ScanIssue, ScanOptions, ScanResult,
    ScanStats, scan,
};
pub use sunburst::{SunburstArc, SunburstOptions, sunburst};
pub use tree::{
    Children, EntryInfo, Node, NodeFlags, NodeId, NodeKind, Removed, Tree, TreeBuilder, TreeError,
};
pub use treemap::{Cell, CellTarget, Rect, TreemapOptions, treemap};

use serde::Serialize;

/// Size of a filesystem entry, reported both ways (see docs/PLAN.md §3.3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Size {
    /// Logical length in bytes (what `ls -l` shows).
    pub logical: u64,
    /// Bytes actually allocated on disk (sparse/compressed/block rounding).
    pub allocated: u64,
}

impl Size {
    /// Adds another size, saturating instead of overflowing.
    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self {
            logical: self.logical.saturating_add(other.logical),
            allocated: self.allocated.saturating_add(other.allocated),
        }
    }

    /// Subtracts another size, stopping at zero instead of underflowing.
    #[must_use]
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self {
            logical: self.logical.saturating_sub(other.logical),
            allocated: self.allocated.saturating_sub(other.allocated),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Size;

    #[test]
    fn saturating_add_sums_both_fields() {
        let a = Size {
            logical: 10,
            allocated: 4096,
        };
        let b = Size {
            logical: 5,
            allocated: 4096,
        };
        assert_eq!(
            a.saturating_add(b),
            Size {
                logical: 15,
                allocated: 8192
            }
        );
    }

    #[test]
    fn saturating_add_does_not_overflow() {
        let a = Size {
            logical: u64::MAX,
            allocated: u64::MAX,
        };
        let b = Size {
            logical: 1,
            allocated: 1,
        };
        assert_eq!(a.saturating_add(b), a);
    }

    #[test]
    fn saturating_sub_stops_at_zero() {
        let a = Size {
            logical: 10,
            allocated: 4096,
        };
        let b = Size {
            logical: 4,
            allocated: 8192,
        };
        assert_eq!(
            a.saturating_sub(b),
            Size {
                logical: 6,
                allocated: 0
            }
        );
    }
}
