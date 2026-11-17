//! Show, open, open-in-terminal, copy-path, move-to-trash and delete for
//! scanned nodes. The UI names a
//! node; the path always comes from the scan tree, never from the webview.
//!
//! Mirrors `ActionError` in `ui/src/lib/ipc.ts`;
//! `ipc_ts_mentions_every_action_tag` catches drift.

use std::fmt;
use std::path::Path;

use cleany_core::NodeKind;
use cleany_platform::shell::{self, ShellError};
use cleany_platform::trash::{self as os_trash, DeleteError, Protection, TrashError};
use serde::Serialize;

use crate::deletion_log::{self, Removal};
use crate::session::{Deletion, ScanId, SessionError, Sessions, TrashOutcome, TrashReport};

/// Why a show / open / open-in-terminal / copy-path request failed.
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
    /// The file manager, default app or terminal could not be started.
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

/// Opens a terminal in the node if it is a folder, else in its parent.
pub fn open_terminal(sessions: &Sessions, scan: ScanId, node: u32) -> Result<(), ActionError> {
    let path = sessions.path(scan, node)?;
    Ok(shell::open_terminal(&path)?)
}

/// The node's full path as text, for the clipboard. Non-UTF-8 bytes become
/// U+FFFD; such a path still shows, though pasting it may not find the file.
pub fn path_text(sessions: &Sessions, scan: ScanId, node: u32) -> Result<String, ActionError> {
    let path = sessions.path(scan, node)?;
    Ok(path.to_string_lossy().into_owned())
}

/// Moves `nodes` to the OS trash and takes them out of the scan tree, then
/// appends every attempt to the deletion log in `log_dir`. Per-item results
/// are in the report; only request-level problems are errors. A log that
/// cannot be written is reported on stderr and does not fail the request,
/// since the items have already moved.
pub fn trash(
    sessions: &Sessions,
    scan: ScanId,
    nodes: &[u32],
    log_dir: Option<&Path>,
) -> Result<TrashReport, ActionError> {
    let protection = Protection::current();
    let (report, deletions) = sessions.trash(scan, nodes, |path, kind, root| {
        move_one(&protection, path, kind, root)
    })?;
    log(log_dir, Removal::Trash, &deletions);
    Ok(report)
}

/// Deletes `nodes` from disk for good, skipping the trash, and takes them
/// out of the scan tree. Same checks, report and log as [`trash`]; the UI
/// asks for confirmation first.
pub fn delete(
    sessions: &Sessions,
    scan: ScanId,
    nodes: &[u32],
    log_dir: Option<&Path>,
) -> Result<TrashReport, ActionError> {
    let protection = Protection::current();
    let (report, deletions) = sessions.trash(scan, nodes, |path, kind, root| {
        remove_one(&protection, path, kind, root)
    })?;
    log(log_dir, Removal::Delete, &deletions);
    Ok(report)
}

/// Appends to the deletion log; a failure goes to stderr only.
fn log(log_dir: Option<&Path>, removal: Removal, deletions: &[Deletion]) {
    if let Some(dir) = log_dir
        && let Err(e) = deletion_log::append(dir, removal, deletions)
    {
        eprintln!("cleany: deletion log: {e}");
    }
}

/// The checks the tree cannot make: the protected paths on disk, and that
/// the item still is what the scan saw. `Some` stops the removal.
fn precheck(
    protection: &Protection,
    path: &Path,
    kind: NodeKind,
    scan_root: &Path,
) -> Option<TrashOutcome> {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return Some(TrashOutcome::Gone);
    };
    if protection.covers(path, scan_root) {
        return Some(TrashOutcome::Protected);
    }
    if !same_kind(meta.file_type(), kind) {
        return Some(TrashOutcome::Failed {
            message: "changed kind since the scan".to_owned(),
        });
    }
    None
}

/// Trashes one item after [`precheck`].
fn move_one(
    protection: &Protection,
    path: &Path,
    kind: NodeKind,
    scan_root: &Path,
) -> TrashOutcome {
    if let Some(stop) = precheck(protection, path, kind, scan_root) {
        return stop;
    }
    match os_trash::move_to_trash(path) {
        Ok(()) => TrashOutcome::Trashed,
        Err(TrashError::Gone) => TrashOutcome::Gone,
        Err(e @ TrashError::Failed(_)) => TrashOutcome::Failed {
            message: e.to_string(),
        },
    }
}

/// Deletes one item for good after [`precheck`].
fn remove_one(
    protection: &Protection,
    path: &Path,
    kind: NodeKind,
    scan_root: &Path,
) -> TrashOutcome {
    if let Some(stop) = precheck(protection, path, kind, scan_root) {
        return stop;
    }
    match os_trash::delete_permanently(path) {
        Ok(()) => TrashOutcome::Deleted,
        Err(DeleteError::Gone) => TrashOutcome::Gone,
        Err(e @ DeleteError::Failed(_)) => TrashOutcome::Failed {
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
    use super::{ActionError, move_one, remove_one};
    use crate::session::{SessionError, TrashOutcome};
    use cleany_core::NodeKind;
    use cleany_platform::shell::ShellError;
    use cleany_platform::trash::Protection;
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
        let p = Protection::current();
        let move_one =
            |path: &std::path::Path, kind, root: &std::path::Path| move_one(&p, path, kind, root);
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
    fn remove_one_checks_before_deleting() {
        let dir = std::env::temp_dir().join(format!("cleany-remove-one-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("f"), b"x").unwrap();
        let p = Protection::current();
        assert_eq!(
            remove_one(&p, &dir.join("nope"), NodeKind::File, &dir),
            TrashOutcome::Gone
        );
        assert_eq!(
            remove_one(&p, &dir, NodeKind::Dir, &dir),
            TrashOutcome::Protected
        );
        assert!(matches!(
            remove_one(&p, &dir.join("sub"), NodeKind::File, &dir),
            TrashOutcome::Failed { .. }
        ));
        assert!(dir.join("sub").is_dir());
        assert_eq!(
            remove_one(&p, &dir.join("f"), NodeKind::File, &dir),
            TrashOutcome::Deleted
        );
        assert!(!dir.join("f").exists());
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
            "'open_terminal'",
            "'get_path'",
            "'trash_nodes'",
            "'delete_nodes'",
        ] {
            words.push(cmd.to_owned());
        }
        let missing: Vec<_> = words.iter().filter(|w| !ts.contains(w.as_str())).collect();
        assert!(missing.is_empty(), "ui/src/lib/ipc.ts lacks {missing:?}");
    }
}
