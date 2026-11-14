//! Cleany core: scan data model, aggregation and analysis.
//!
//! The arena tree lives in [`tree`], the parallel scanner in [`scan`]
//! (see docs/PLAN.md §3) and the treemap layout in [`mod@treemap`].

mod names;
pub mod scan;
pub mod tree;
pub mod treemap;

pub use scan::{
    CancelToken, PROGRESS_INTERVAL, Progress, ScanError, ScanIssue, ScanOptions, ScanResult,
    ScanStats, scan,
};
pub use tree::{
    Children, EntryInfo, Node, NodeFlags, NodeId, NodeKind, Tree, TreeBuilder, TreeError,
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
}
