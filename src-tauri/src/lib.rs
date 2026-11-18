//! Tauri shell for Cleany. Commands here stay thin; logic lives in `cleany-core`.

mod access;
mod actions;
#[cfg(target_os = "macos")]
mod app_menu;
mod deletion_log;
mod export;
#[cfg(target_os = "macos")]
mod login_item;
mod session;
mod tray;
mod update;
mod volumes;

use std::path::PathBuf;

use cleany_core::ScanOptions;
use serde::Serialize;
use tauri::Manager;
use tauri::ipc::Channel;

use access::{AccessError, DiskAccess};
use actions::ActionError;
use export::{ExportError, ExportEvent, ExportReport, Exports, Format};
use session::{
    ChildrenPage, ExportScope, LargestEntry, LargestKind, NodeEntry, ScanEvent, ScanId,
    ScanSummary, SessionError, Sessions, SortKey, SortOrder, SunburstSlice, TrashReport,
    TreemapSlice,
};
use update::{PendingUpdate, UpdateError, UpdateEvent, UpdateInfo};
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

/// The `limit` largest files or folders anywhere below a node.
#[tauri::command(async)]
fn get_largest(
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    node: u32,
    kind: LargestKind,
    limit: u32,
) -> Result<Vec<LargestEntry>, SessionError> {
    sessions.largest(scan, node, kind, limit)
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

/// Opens a terminal in the node, or in its folder if it is not a folder.
#[tauri::command(async)]
fn open_terminal(
    sessions: tauri::State<'_, Sessions>,
    scan: ScanId,
    node: u32,
) -> Result<(), ActionError> {
    actions::open_terminal(&sessions, scan, node)
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
/// in Settings and asks for confirmation; this command never asks.
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

/// Asks where to save, then writes `scope` of the scan there as `format`.
/// `None` when the user closes the save dialog. The dialog runs here, so the
/// path never passes through the webview. Once writing starts, `on_event`
/// gets the file name and then the rows written, a few times a second;
/// `cancel_export` stops it and removes the partial file.
#[tauri::command(async)]
fn export_scan(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, Sessions>,
    exports: tauri::State<'_, Exports>,
    scan: ScanId,
    scope: ExportScope,
    format: Format,
    on_event: Channel<ExportEvent>,
) -> Result<Option<ExportReport>, ExportError> {
    use tauri_plugin_dialog::DialogExt;

    let scope_path = sessions.path(scan, scope.node())?;
    let scope_name = scope_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let now = std::time::SystemTime::now();
    let Some(chosen) = app
        .dialog()
        .file()
        .set_file_name(export::default_name(&scope_name, format, now))
        .add_filter(format.label(), &[format.extension()])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = chosen.into_path().map_err(|e| ExportError::Dialog {
        message: e.to_string(),
    })?;
    exports.begin();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    // The window may be gone; the export still finishes.
    let _ = on_event.send(ExportEvent::Started { file_name });
    let progress = exports.reporter(|p| {
        let _ = on_event.send(ExportEvent::Progress {
            rows: p.rows,
            total: p.total,
        });
    });
    let report = export::write_file(&sessions, scan, scope, format, &path, now, progress)?;
    exports.finished(path);
    Ok(Some(report))
}

/// Stops the running export at its next batch; it then fails with
/// `cancelled` and leaves no file behind.
#[tauri::command]
fn cancel_export(exports: tauri::State<'_, Exports>) {
    exports.cancel();
}

/// Shows the file the last successful export wrote in Finder / Explorer /
/// the file manager. `gone` when nothing was exported in this run.
#[tauri::command(async)]
fn reveal_export(exports: tauri::State<'_, Exports>) -> Result<(), ActionError> {
    let path = exports.last().ok_or(ActionError::Gone)?;
    Ok(cleany_platform::shell::reveal(&path)?)
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

/// Relabels the menu bar menu and the app menu in the UI's language.
#[tauri::command]
fn set_tray_locale(app: tauri::AppHandle, locale: tray::Locale) {
    tray::set_locale(&app, locale);
    update::set_locale(&app, locale);
    #[cfg(target_os = "macos")]
    app_menu::set_locale(&app, locale);
}

/// Whether Cleany starts at login.
#[tauri::command(async)]
fn launch_at_login(app: tauri::AppHandle) -> Result<bool, tray::LoginItemError> {
    tray::starts_at_login(&app)
}

/// Turns starting at login on or off; returns the state the OS reports after.
#[tauri::command(async)]
fn set_launch_at_login(app: tauri::AppHandle, on: bool) -> Result<bool, tray::LoginItemError> {
    tray::set_start_at_login(&app, on)
}

/// Looks for a newer release on GitHub. `None` when this copy is current.
#[tauri::command]
async fn check_update(
    app: tauri::AppHandle,
    pending: tauri::State<'_, PendingUpdate>,
) -> Result<Option<UpdateInfo>, UpdateError> {
    update::check(&app, &pending).await
}

/// Downloads and installs the update found by `check_update`, then restarts.
/// Progress arrives on `on_event`.
#[tauri::command]
async fn install_update(
    app: tauri::AppHandle,
    pending: tauri::State<'_, PendingUpdate>,
    on_event: Channel<UpdateEvent>,
) -> Result<(), UpdateError> {
    update::install(&app, pending, on_event).await
}

/// Builds and runs the Tauri application.
///
/// # Panics
/// Never by design: a startup failure is reported on stderr and the process exits non-zero.
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args([tray::HIDDEN_ARG])
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Sessions::default())
        .manage(PendingUpdate::default())
        .manage(Exports::default())
        .setup(|app| {
            let hidden = std::env::args().any(|a| a == tray::HIDDEN_ARG);
            #[cfg(target_os = "macos")]
            {
                // A hidden start while Cleany already runs is the agent's own
                // first run after registering; the running copy stays.
                if !login_item::claim_instance(app.handle()) && hidden {
                    std::process::exit(0);
                }
                if login_item::supported() {
                    login_item::migrate(app.handle());
                }
            }
            tray::install(app.handle())?;
            #[cfg(target_os = "macos")]
            app_menu::install(app.handle())?;
            if hidden && let Some(window) = app.get_webview_window(tray::MAIN_WINDOW) {
                window.hide()?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing keeps Cleany in the menu bar; "Quit" or Cmd+Q ends it.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.label() == tray::MAIN_WINDOW
            {
                api.prevent_close();
                if let Err(e) = window.hide() {
                    eprintln!("cleany: hide window: {e}");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_volumes,
            start_scan,
            cancel_scan,
            attach_scan,
            get_children,
            get_largest,
            get_ancestors,
            get_treemap,
            get_sunburst,
            reveal_node,
            open_node,
            open_terminal,
            get_path,
            trash_nodes,
            delete_nodes,
            export_scan,
            cancel_export,
            reveal_export,
            disk_access,
            open_disk_access_settings,
            set_tray_locale,
            launch_at_login,
            set_launch_at_login,
            check_update,
            install_update
        ])
        .build(tauri::generate_context!());
    let app = match app {
        Ok(app) => app,
        Err(err) => {
            eprintln!("cleany: failed to start: {err}");
            std::process::exit(1);
        }
    };
    app.run(|app, event| {
        // A Dock click while the window is hidden brings it back.
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            tray::show_main(app);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
}
