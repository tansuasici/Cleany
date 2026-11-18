//! Checks GitHub Releases for a newer Cleany and installs it when the user agrees.
//!
//! The UI asks once at launch; nothing downloads until the user picks
//! "Install and restart". The plugin verifies the download against the
//! public key in `tauri.conf.json` before it touches the installed app.

use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Least time between two [`UpdateEvent::Progress`] events.
const PROGRESS_EVERY: Duration = Duration::from_millis(150);

/// The update found by the last check, kept until the user installs it.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

/// A newer release. Mirrors `UpdateInfo` in `ui/src/lib/ipc.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// The new version, without a leading `v`.
    version: String,
    /// The version running now.
    current_version: String,
    /// Whether this copy can replace itself. False for a Linux install that
    /// came from a package the release does not offer as an update.
    can_install: bool,
}

/// Download progress while installing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UpdateEvent {
    /// Bytes received so far.
    Progress {
        /// Bytes received.
        received: u64,
        /// Size of the download, when the server sends it.
        total: Option<u64>,
    },
    /// The download is complete and verified; installing starts.
    Installing,
}

/// Checking for or installing an update failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UpdateError {
    /// The release could not be reached, read or verified.
    Failed {
        /// What went wrong, for the log; the UI shows its own text.
        message: String,
    },
    /// Install was asked for without a pending update.
    NothingPending,
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed { message } => write!(f, "update: {message}"),
            Self::NothingPending => write!(f, "update: nothing to install"),
        }
    }
}

impl std::error::Error for UpdateError {}

fn failed(err: impl fmt::Display) -> UpdateError {
    UpdateError::Failed {
        message: err.to_string(),
    }
}

/// Looks for a newer release and keeps it for [`install`].
///
/// Debug builds read the manifest from `CLEANY_UPDATE_ENDPOINT` when it is
/// set, so the flow can be tried against a local server.
///
/// # Errors
/// [`UpdateError::Failed`] when the manifest cannot be fetched or read.
pub async fn check<R: Runtime>(
    app: &AppHandle<R>,
    pending: &PendingUpdate,
) -> Result<Option<UpdateInfo>, UpdateError> {
    #[allow(unused_mut)]
    let mut builder = app.updater_builder();
    #[cfg(debug_assertions)]
    if let Some(endpoint) = std::env::var_os("CLEANY_UPDATE_ENDPOINT") {
        let url = endpoint.to_string_lossy().parse().map_err(failed)?;
        builder = builder.endpoints(vec![url]).map_err(failed)?;
    }
    let update = builder
        .build()
        .map_err(failed)?
        .check()
        .await
        .map_err(failed)?;
    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        can_install: installs_in_place(u.download_url.path()),
    });
    *pending.0.lock().map_err(failed)? = update;
    Ok(info)
}

/// Downloads and installs the pending update, then restarts Cleany.
///
/// On Windows the installer closes Cleany itself and starts the new version.
///
/// # Errors
/// [`UpdateError::NothingPending`] without a prior successful [`check`];
/// [`UpdateError::Failed`] when the download, its signature or the install fails.
pub async fn install<R: Runtime>(
    app: &AppHandle<R>,
    pending: State<'_, PendingUpdate>,
    on_event: Channel<UpdateEvent>,
) -> Result<(), UpdateError> {
    let update = pending
        .0
        .lock()
        .map_err(failed)?
        .take()
        .ok_or(UpdateError::NothingPending)?;
    let mut received: u64 = 0;
    let mut last = Instant::now();
    let installing = on_event.clone();
    update
        .download_and_install(
            |chunk, total| {
                received += chunk as u64;
                if last.elapsed() >= PROGRESS_EVERY || total == Some(received) {
                    last = Instant::now();
                    report(on_event.send(UpdateEvent::Progress { received, total }));
                }
            },
            || report(installing.send(UpdateEvent::Installing)),
        )
        .await
        .map_err(failed)?;
    app.restart()
}

fn report(result: tauri::Result<()>) {
    if let Err(e) = result {
        eprintln!("cleany: update progress: {e}");
    }
}

/// Whether the download at `path` can replace this copy. On Linux the plugin
/// installs a `.deb` or `.rpm` only over the same kind of package, and an
/// AppImage only over an AppImage; anything else is updated by the user.
fn installs_in_place(path: &str) -> bool {
    if !cfg!(target_os = "linux") {
        return true;
    }
    let appimage = path.ends_with(".AppImage") || path.ends_with(".AppImage.tar.gz");
    let running_appimage = std::env::var_os("APPIMAGE").is_some();
    if running_appimage {
        appimage
    } else {
        path.ends_with(".deb") || path.ends_with(".rpm")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_linux_always_installs() {
        if !cfg!(target_os = "linux") {
            assert!(installs_in_place("/a/Cleany.app.tar.gz"));
            assert!(installs_in_place("/a/Cleany_0.2.0_x64-setup.exe"));
        }
    }

    #[test]
    fn ipc_ts_mentions_every_update_key() {
        let ts = include_str!("../../ui/src/lib/ipc.ts");
        for key in [
            "currentVersion",
            "canInstall",
            "'progress'",
            "received",
            "'installing'",
            "'failed'",
            "'nothingPending'",
            "'check_update'",
            "'install_update'",
        ] {
            assert!(ts.contains(key), "ipc.ts is missing {key}");
        }
    }

    #[test]
    fn error_names_its_kind() {
        let json = serde_json::to_string(&UpdateError::NothingPending).unwrap();
        assert_eq!(json, r#"{"kind":"nothingPending"}"#);
    }

    #[test]
    fn progress_is_camel_case() {
        let json = serde_json::to_string(&UpdateEvent::Progress {
            received: 5,
            total: Some(10),
        })
        .unwrap();
        assert_eq!(json, r#"{"kind":"progress","received":5,"total":10}"#);
    }
}
