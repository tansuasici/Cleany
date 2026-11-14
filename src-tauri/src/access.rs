//! macOS Full Disk Access: a best-effort check and a way to the settings pane.
//! Mirrors `DiskAccess` and `AccessError` in `ui/src/lib/ipc.ts`;
//! `ipc_ts_mentions_every_access_tag` catches drift.
//!
//! macOS has no public API that reports whether an app holds Full Disk
//! Access, so the check opens files that only such an app may open. It reads
//! nothing from them.

use std::fmt;
use std::io;

use serde::Serialize;

/// Whether the app may read the folders macOS protects (Mail, Safari, other apps' data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiskAccess {
    /// A protected file opened.
    Granted,
    /// macOS refused a protected file: some folders will be left out of a scan.
    Denied,
    /// No probe gave an answer (files missing, or another error).
    Unknown,
    /// Not macOS; there is nothing to grant.
    #[cfg_attr(
        all(target_os = "macos", not(test)),
        expect(dead_code, reason = "only built off macOS")
    )]
    NotApplicable,
}

/// Why the settings pane could not be opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AccessError {
    /// Only macOS has the pane.
    #[cfg_attr(
        all(target_os = "macos", not(test)),
        expect(dead_code, reason = "only built off macOS")
    )]
    Unsupported,
    /// `open` could not be run or failed.
    Launch {
        /// The OS error or exit status.
        message: String,
    },
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("full disk access is a macOS setting"),
            Self::Launch { message } => write!(f, "cannot open system settings: {message}"),
        }
    }
}

impl std::error::Error for AccessError {}

/// What one probe's result says, if anything.
fn classify(result: io::Result<()>) -> Option<DiskAccess> {
    match result {
        Ok(()) => Some(DiskAccess::Granted),
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => Some(DiskAccess::Denied),
        // Missing or unreadable for another reason: let the next probe decide.
        Err(_) => None,
    }
}

/// The first probe that gives an answer decides; with none, `Unknown`.
fn decide(probes: impl IntoIterator<Item = io::Result<()>>) -> DiskAccess {
    probes
        .into_iter()
        .find_map(classify)
        .unwrap_or(DiskAccess::Unknown)
}

/// Best-effort Full Disk Access status of this process.
#[cfg(target_os = "macos")]
pub fn status() -> DiskAccess {
    use std::fs::File;
    use std::path::PathBuf;

    let mut paths = vec![PathBuf::from(
        "/Library/Application Support/com.apple.TCC/TCC.db",
    )];
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home).join("Library/Safari"));
    }
    decide(paths.iter().map(|p| File::open(p).map(drop)))
}

/// Best-effort Full Disk Access status of this process.
#[cfg(not(target_os = "macos"))]
pub fn status() -> DiskAccess {
    DiskAccess::NotApplicable
}

/// The Privacy & Security → Full Disk Access pane. Fixed: nothing from the UI reaches `open`.
#[cfg(target_os = "macos")]
const SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles";

/// Opens System Settings at the Full Disk Access pane.
///
/// # Errors
/// `Launch` when `open` cannot run or exits with an error.
#[cfg(target_os = "macos")]
pub fn open_settings() -> Result<(), AccessError> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg(SETTINGS_URL)
        .status()
        .map_err(|err| AccessError::Launch {
            message: err.to_string(),
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(AccessError::Launch {
            message: status.to_string(),
        })
    }
}

/// Opens System Settings at the Full Disk Access pane.
///
/// # Errors
/// Always `Unsupported`: only macOS has the pane.
#[cfg(not(target_os = "macos"))]
pub fn open_settings() -> Result<(), AccessError> {
    Err(AccessError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::{AccessError, DiskAccess, decide};
    use std::io::{Error, ErrorKind};

    #[test]
    fn permission_denied_means_denied() {
        assert_eq!(
            decide([Err(Error::from(ErrorKind::PermissionDenied))]),
            DiskAccess::Denied
        );
    }

    #[test]
    fn opened_means_granted() {
        assert_eq!(decide([Ok(())]), DiskAccess::Granted);
    }

    #[test]
    fn missing_file_defers_to_next_probe() {
        assert_eq!(
            decide([Err(Error::from(ErrorKind::NotFound)), Ok(())]),
            DiskAccess::Granted
        );
        assert_eq!(
            decide([
                Err(Error::from(ErrorKind::NotFound)),
                Err(Error::from(ErrorKind::PermissionDenied)),
            ]),
            DiskAccess::Denied
        );
    }

    #[test]
    fn no_answer_is_unknown() {
        assert_eq!(decide([]), DiskAccess::Unknown);
        assert_eq!(
            decide([Err(Error::from(ErrorKind::NotFound))]),
            DiskAccess::Unknown
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_is_not_applicable() {
        assert_eq!(super::status(), DiskAccess::NotApplicable);
        assert_eq!(super::open_settings(), Err(AccessError::Unsupported));
    }

    #[test]
    fn json_shape() {
        assert_eq!(
            serde_json::to_value(DiskAccess::NotApplicable).unwrap(),
            serde_json::json!("notApplicable")
        );
        assert_eq!(
            serde_json::to_value(AccessError::Launch {
                message: "m".to_owned()
            })
            .unwrap(),
            serde_json::json!({"kind": "launch", "message": "m"})
        );
    }

    #[test]
    fn ipc_ts_mentions_every_access_tag() {
        let ts = include_str!("../../ui/src/lib/ipc.ts");
        let mut words = Vec::new();
        for s in [
            DiskAccess::Granted,
            DiskAccess::Denied,
            DiskAccess::Unknown,
            DiskAccess::NotApplicable,
        ] {
            words.push(format!(
                "'{}'",
                serde_json::to_value(s).unwrap().as_str().unwrap()
            ));
        }
        for e in [
            AccessError::Unsupported,
            AccessError::Launch {
                message: String::new(),
            },
        ] {
            let v = serde_json::to_value(e).unwrap();
            words.push(format!("'{}'", v["kind"].as_str().unwrap()));
        }
        words.push("message:".to_owned());
        let missing: Vec<_> = words.iter().filter(|w| !ts.contains(w.as_str())).collect();
        assert!(missing.is_empty(), "ui/src/lib/ipc.ts lacks {missing:?}");
    }
}
