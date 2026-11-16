//! Show, open, copy-path and move-to-trash for scanned nodes. The UI names a
//! node; the path always comes from the scan tree, never from the webview.
//!
//! Mirrors `ActionError` in `ui/src/lib/ipc.ts`;
//! `ipc_ts_mentions_every_action_tag` catches drift.

use std::fmt;
use std::path::Path;

use cleany_core::NodeKind;
use cleany_platform::shell::{self, ShellError};
use cleany_platform::trash::{self as os_trash, TrashError};
use serde::Serialize;

use crate::session::{ScanId, SessionError, Sessions, TrashOutcome, TrashReport};

/// Why a show / open / copy-path request failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ActionError {
    /// The id is not the current scan.
    StaleScan,
    /// The current scan has no tree.
    NotReady,
    /// The node index is not in the tree.
    UnknownNode,
    /// The item was deleted or moved since the scan.
    Gone,
    /// The file manager or default app could not be started.
    Launch {
        /// What went wrong, for the log; the UI shows its own text.
        message: String,
    },
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleScan => f.write_str("scan was replaced by a newer one"),
            Self::NotReady => f.write_str("scan has no results"),
            Self::UnknownNode => f.write_str("unknown node"),
            Self::Gone => f.write_str("the item no longer exists"),
            Self::Launch { message } => write!(f, "cannot launch: {message}"),
        }
    }
}

impl std::error::Error for ActionError {}

impl From<SessionError> for ActionError {
    fn from(e: SessionError) -> Self {
        match e {
            SessionError::StaleScan => Self::StaleScan,
            SessionError::NotReady => Self::NotReady,
            SessionError::UnknownNode => Self::UnknownNode,
            // Not produced by a path lookup; keep the text if one ever is.
            SessionError::InvalidPath | SessionError::Spawn { .. } => Self::Launch {
                message: e.to_string(),
            },
        }
    }
}

impl From<ShellError> for ActionError {
    fn from(e: ShellError) -> Self {
        match e {
            ShellError::Gone => Self::Gone,
            ShellError::Unsupported | ShellError::Launch(_) => Self::Launch {
                message: e.to_string(),
            },
        }
    }
}

/// Shows the node selected in the OS file manager.
pub fn reveal(sessions: &Sessions, scan: ScanId, node: u32) -> Result<(), ActionError> {
    let path = sessions.path(scan, node)?;
    Ok(shell::reveal(&path)?)
}

/// Opens the node with its default app.
pub fn open(sessions: &Sessions, scan: ScanId, node: u32) -> Result<(), ActionError> {
    let path = sessions.path(scan, node)?;
    Ok(shell::open(&path)?)
}

/// The node's full path as text, for the clipboard. Non-UTF-8 bytes become
/// U+FFFD; such a path still shows, though pasting it may not find the file.
pub fn path_text(sessions: &Sessions, scan: ScanId, node: u32) -> Result<String, ActionError> {
    let path = sessions.path(scan, node)?;
    Ok(path.to_string_lossy().into_owned())
}

/// Moves `nodes` to the OS trash and takes them out of the scan tree.
/// Per-item results are in the report; only request-level problems are errors.
pub fn trash(sessions: &Sessions, scan: ScanId, nodes: &[u32]) -> Result<TrashReport, ActionError> {
    Ok(sessions.trash(scan, nodes, move_one)?)
}

/// Trashes one item after the checks the tree cannot make: the protected
/// paths on disk, and that the item still is what the scan saw.
fn move_one(path: &Path, kind: NodeKind, scan_root: &Path) -> TrashOutcome {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return TrashOutcome::Gone;
    };
    if os_trash::is_protected(path, scan_root) {
        return TrashOutcome::Protected;
    }
    if !same_kind(meta.file_type(), kind) {
        return TrashOutcome::Failed {
            message: "changed kind since the scan".to_owned(),
        };
    }
    match os_trash::move_to_trash(path) {
        Ok(()) => TrashOutcome::Trashed,
        Err(TrashError::Gone) => TrashOutcome::Gone,
        Err(e @ TrashError::Failed(_)) => TrashOutcome::Failed {
            message: e.to_string(),
        },
    }
}

/// Whether the entry on disk is still the kind the scan recorded.
fn same_kind(ft: std::fs::FileType, kind: NodeKind) -> bool {
    match kind {
        NodeKind::Dir => ft.is_dir(),
        NodeKind::File => ft.is_file(),
        NodeKind::Symlink => ft.is_symlink(),
        NodeKind::Other => !ft.is_dir() && !ft.is_file() && !ft.is_symlink(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionError, move_one};
    use crate::session::{SessionError, TrashOutcome};
    use cleany_core::NodeKind;
    use cleany_platform::shell::ShellError;
    use std::fs;

    #[test]
    fn errors_map_to_ui_kinds() {
        assert_eq!(
            ActionError::from(SessionError::StaleScan),
            ActionError::StaleScan
        );
        assert_eq!(ActionError::from(ShellError::Gone), ActionError::Gone);
        assert!(matches!(
            ActionError::from(ShellError::Unsupported),
            ActionError::Launch { .. }
        ));
    }

    #[test]
    fn move_one_checks_before_touching_the_trash() {
        let dir = std::env::temp_dir().join(format!("cleany-move-one-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        // Missing on disk.
        assert_eq!(
            move_one(&dir.join("nope"), NodeKind::File, &dir),
            TrashOutcome::Gone
        );
        // The scan root itself.
        assert_eq!(move_one(&dir, NodeKind::Dir, &dir), TrashOutcome::Protected);
        // A folder the scan saw as a file.
        assert!(matches!(
            move_one(&dir.join("sub"), NodeKind::File, &dir),
            TrashOutcome::Failed { .. }
        ));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ipc_ts_mentions_every_action_tag() {
        let ts = include_str!("../../ui/src/lib/ipc.ts");
        let mut words = vec!["ActionError".to_owned(), "message:".to_owned()];
        for e in [
            ActionError::StaleScan,
            ActionError::NotReady,
            ActionError::UnknownNode,
            ActionError::Gone,
            ActionError::Launch {
                message: String::new(),
            },
        ] {
            let v = serde_json::to_value(e).unwrap();
            words.push(format!("'{}'", v["kind"].as_str().unwrap()));
        }
        for cmd in [
            "'reveal_node'",
            "'open_node'",
            "'get_path'",
            "'trash_nodes'",
        ] {
            words.push(cmd.to_owned());
        }
        let missing: Vec<_> = words.iter().filter(|w| !ts.contains(w.as_str())).collect();
        assert!(missing.is_empty(), "ui/src/lib/ipc.ts lacks {missing:?}");
    }
}
