//! Tauri shell for Cleany. Commands here stay thin; logic lives in `cleany-core`.

mod session;

use std::path::PathBuf;

use cleany_core::ScanOptions;
use serde::Serialize;
use tauri::ipc::Channel;

use session::{
    ChildrenPage, NodeEntry, ScanEvent, ScanId, SessionError, Sessions, SortKey, SortOrder,
};

/// Basic build/runtime info shown in the UI shell. Mirrors `AppInfo` in `ui/src/lib/ipc.ts`.
#[derive(Debug, Serialize)]
pub struct AppInfo {
    name: &'static str,
    version: &'static str,
    os: &'static str,
    arch: &'static str,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "Cleany",
        version: env!("CARGO_PKG_VERSION"),
        os: cleany_platform::Os::current().as_str(),
        arch: std::env::consts::ARCH,
    }
}

/// Starts scanning `path`, replacing (and cancelling) any previous scan.
/// Progress and the final result arrive on `on_event`.
#[tauri::command]
fn start_scan(
    sessions: tauri::State<'_, Sessions>,
    path: PathBuf,
    cross_mounts: bool,
    on_event: Channel<ScanEvent>,
) -> Result<ScanId, SessionError> {
    let options = ScanOptions {
        cross_mounts,
        ..ScanOptions::default()
    };
    sessions.start(path, options, move |event| {
        // The window may be gone; there is no one left to tell.
        let _ = on_event.send(event);
    })
}

/// Asks a running scan to stop; its stream then ends with `cancelled`.
#[tauri::command]
fn cancel_scan(sessions: tauri::State<'_, Sessions>, scan: ScanId) -> Result<(), SessionError> {
    sessions.cancel(scan)
}

/// One sorted page of a node's children.
#[tauri::command(async)]
fn get_children(
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    node: u32,
    sort: SortKey,
    order: SortOrder,
    offset: u32,
    limit: u32,
) -> Result<ChildrenPage, SessionError> {
    sessions.children(scan, node, sort, order, offset, limit)
}

/// A node and its ancestors, root first.
#[tauri::command(async)]
fn get_ancestors(
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    node: u32,
) -> Result<Vec<NodeEntry>, SessionError> {
    sessions.ancestors(scan, node)
}

/// Builds and runs the Tauri application.
///
/// # Panics
/// Never by design: a startup failure is reported on stderr and the process exits non-zero.
pub fn run() {
    let result = tauri::Builder::default()
        .manage(Sessions::default())
        .invoke_handler(tauri::generate_handler![
            app_info,
            start_scan,
            cancel_scan,
            get_children,
            get_ancestors
        ])
        .run(tauri::generate_context!());
    if let Err(err) = result {
        eprintln!("cleany: failed to start: {err}");
        std::process::exit(1);
    }
}
