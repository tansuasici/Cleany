//! Tauri shell for Cleany. Commands here stay thin; logic lives in `cleany-core`.

mod access;
mod actions;
mod deletion_log;
mod session;
mod volumes;

use std::path::PathBuf;

use cleany_core::ScanOptions;
use serde::Serialize;
use tauri::Manager;
use tauri::ipc::Channel;

use access::{AccessError, DiskAccess};
use actions::ActionError;
use session::{
    ChildrenPage, NodeEntry, ScanEvent, ScanId, ScanSummary, SessionError, Sessions, SortKey,
    SortOrder, SunburstSlice, TrashReport, TreemapSlice,
};
use volumes::VolumeEntry;

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

/// Mounted volumes the user can scan, sorted by mount point.
#[tauri::command(async)]
fn list_volumes() -> Vec<VolumeEntry> {
    volumes::list()
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

/// The current scan's summary if it finished, so a reloaded webview can
/// reopen it without scanning again. `None` otherwise. Live updates of that
/// scan (`changed`, `outdated`) go to `on_event` from then on.
#[tauri::command]
fn attach_scan(
    sessions: tauri::State<'_, Sessions>,
    on_event: Channel<ScanEvent>,
) -> Option<ScanSummary> {
    sessions.attach(move |event| {
        // The webview may be gone; the next attach brings a new channel.
        let _ = on_event.send(event);
    })
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

/// A node's treemap in a `width` × `height` CSS-pixel box.
#[tauri::command(async)]
fn get_treemap(
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    node: u32,
    width: f32,
    height: f32,
    depth: u8,
    min_area: f32,
) -> Result<TreemapSlice, SessionError> {
    sessions.treemap(scan, node, width, height, depth, min_area)
}

/// A node's sunburst, `depth` rings deep.
#[tauri::command(async)]
fn get_sunburst(
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    node: u32,
    depth: u8,
    min_angle: f32,
) -> Result<SunburstSlice, SessionError> {
    sessions.sunburst(scan, node, depth, min_angle)
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

/// Shows the node selected in Finder / Explorer / the file manager.
#[tauri::command(async)]
fn reveal_node(
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    node: u32,
) -> Result<(), ActionError> {
    actions::reveal(&sessions, scan, node)
}

/// Opens the node with its default app, as a double-click would.
#[tauri::command(async)]
fn open_node(
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    node: u32,
) -> Result<(), ActionError> {
    actions::open(&sessions, scan, node)
}

/// The node's full path, for the clipboard.
#[tauri::command(async)]
fn get_path(
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    node: u32,
) -> Result<String, ActionError> {
    actions::path_text(&sessions, scan, node)
}

/// Moves the nodes to the OS trash and reports what happened to each. The
/// UI confirms first; this command never asks. Every attempt is also written
/// to the deletion log in the app's log folder.
#[tauri::command(async)]
fn trash_nodes(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    nodes: Vec<u32>,
) -> Result<TrashReport, ActionError> {
    let log_dir = app
        .path()
        .app_log_dir()
        .inspect_err(|e| eprintln!("cleany: deletion log: no log folder: {e}"))
        .ok();
    actions::trash(&sessions, scan, &nodes, log_dir.as_deref())
}

/// Deletes the nodes from disk for good, skipping the trash, and reports
/// what happened to each. The UI offers it only when the user turned it on
/// in Settings and asks for a typed confirmation; this command never asks.
/// Every attempt is written to the deletion log.
#[tauri::command(async)]
fn delete_nodes(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    nodes: Vec<u32>,
) -> Result<TrashReport, ActionError> {
    let log_dir = app
        .path()
        .app_log_dir()
        .inspect_err(|e| eprintln!("cleany: deletion log: no log folder: {e}"))
        .ok();
    actions::delete(&sessions, scan, &nodes, log_dir.as_deref())
}

/// Best-effort macOS Full Disk Access status; `notApplicable` elsewhere.
#[tauri::command(async)]
fn disk_access() -> DiskAccess {
    access::status()
}

/// Opens System Settings at the Full Disk Access pane (macOS only).
#[tauri::command(async)]
fn open_disk_access_settings() -> Result<(), AccessError> {
    access::open_settings()
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
            list_volumes,
            start_scan,
            cancel_scan,
            attach_scan,
            get_children,
            get_ancestors,
            get_treemap,
            get_sunburst,
            reveal_node,
            open_node,
            get_path,
            trash_nodes,
            delete_nodes,
            disk_access,
            open_disk_access_settings
        ])
        .run(tauri::generate_context!());
    if let Err(err) = result {
        eprintln!("cleany: failed to start: {err}");
        std::process::exit(1);
    }
}
