//! Show, open and copy-path for a scanned node. The UI names a node; the path
//! always comes from the scan tree, never from the webview.
//!
//! Mirrors `ActionError` in `ui/src/lib/ipc.ts`;
//! `ipc_ts_mentions_every_action_tag` catches drift.

use std::fmt;

use cleany_platform::shell::{self, ShellError};
use serde::Serialize;

use crate::session::{ScanId, SessionError, Sessions};

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

#[cfg(test)]
mod tests {
    use super::ActionError;
    use crate::session::SessionError;
    use cleany_platform::shell::ShellError;

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
        for cmd in ["'reveal_node'", "'open_node'", "'get_path'"] {
            words.push(cmd.to_owned());
        }
        let missing: Vec<_> = words.iter().filter(|w| !ts.contains(w.as_str())).collect();
        assert!(missing.is_empty(), "ui/src/lib/ipc.ts lacks {missing:?}");
    }
}
