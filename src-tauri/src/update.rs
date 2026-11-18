//! Checks GitHub Releases for a newer Cleany and installs it when the user agrees.
//!
//! The UI asks once at launch; nothing downloads until the user picks
//! "Install and restart". The plugin verifies the download against the
//! public key in `tauri.conf.json` before it touches the installed app.

use std::fmt;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::menu::MenuItem;
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::tray::{self, Locale};

/// Id of the update item in the app menu and the menu bar menu.
pub const MENU_ID: &str = "update";

/// The DOM event that asks the UI to check for an update.
const CHECK_EVENT: &str = "cleany:check-update";

/// The DOM event that asks the UI to install the update it found.
const INSTALL_EVENT: &str = "cleany:install-update";

/// Least time between two [`UpdateEvent::Progress`] events.
const PROGRESS_EVERY: Duration = Duration::from_millis(150);

/// What the update menu item offers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum MenuPhase {
    /// Nothing known; a click checks.
    #[default]
    Idle,
    /// A check is running.
    Checking,
    /// A check found this version; a click installs it.
    Available {
        /// The new version, without a leading `v`.
        version: String,
    },
    /// The update is downloading or installing.
    Installing,
}

#[derive(Default)]
struct MenuState {
    phase: MenuPhase,
    locale: Locale,
}

/// The update items, one per menu, kept showing the same state.
pub struct UpdateMenu<R: Runtime> {
    items: Mutex<Vec<MenuItem<R>>>,
    state: Mutex<MenuState>,
}

impl<R: Runtime> Default for UpdateMenu<R> {
    fn default() -> Self {
        Self {
            items: Mutex::new(Vec::new()),
            state: Mutex::new(MenuState::default()),
        }
    }
}

impl<R: Runtime> UpdateMenu<R> {
    /// Changes the state, then relabels the items on the main thread.
    ///
    /// `set_text` off the main thread waits for the main thread, so no lock may
    /// be held across it: the main thread can be waiting for that same lock
    /// (the UI's locale change), and both would stop. The relabel reads the
    /// state when it runs, so the last change wins whatever the order.
    fn update(&self, change: impl FnOnce(&mut MenuState)) {
        change(&mut self.state.lock().unwrap_or_else(PoisonError::into_inner));
        let items = self
            .items
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let Some(app) = items.first().map(|item| item.app_handle().clone()) else {
            return;
        };
        let relabel = app.clone();
        report_menu(app.run_on_main_thread(move || {
            let menu = relabel.state::<UpdateMenu<R>>();
            let (text, enabled) = {
                let state = menu.state.lock().unwrap_or_else(PoisonError::into_inner);
                label(&state.phase, state.locale)
            };
            for item in &items {
                report_menu(item.set_text(&text));
                report_menu(item.set_enabled(enabled));
            }
        }));
    }
}

/// Makes the update item for one menu. Every item made here shows the same state.
///
/// # Errors
/// When the item cannot be created.
pub fn menu_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<MenuItem<R>> {
    if app.try_state::<UpdateMenu<R>>().is_none() {
        app.manage(UpdateMenu::<R>::default());
    }
    // Managed just above, so the lookup cannot fail.
    let menu = app.state::<UpdateMenu<R>>();
    let (text, enabled) = {
        let state = menu.state.lock().unwrap_or_else(PoisonError::into_inner);
        label(&state.phase, state.locale)
    };
    let item = MenuItem::with_id(app, MENU_ID, text, enabled, None::<&str>)?;
    menu.items
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(item.clone());
    Ok(item)
}

/// Relabels the update items in the UI's language.
pub fn set_locale<R: Runtime>(app: &AppHandle<R>, locale: Locale) {
    if let Some(menu) = app.try_state::<UpdateMenu<R>>() {
        menu.update(|state| state.locale = locale);
    }
}

/// Answers a click on the update item: shows the window and asks the UI to
/// check, or to install what the last check found.
pub fn on_menu_click<R: Runtime>(app: &AppHandle<R>) {
    let Some(menu) = app.try_state::<UpdateMenu<R>>() else {
        return;
    };
    let event = match menu
        .state
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .phase
    {
        MenuPhase::Idle => CHECK_EVENT,
        MenuPhase::Available { .. } => INSTALL_EVENT,
        MenuPhase::Checking | MenuPhase::Installing => return,
    };
    tray::show_main(app);
    if let Some(window) = app.get_webview_window(tray::MAIN_WINDOW)
        && let Err(e) = window.eval(format!("window.dispatchEvent(new Event('{event}'))"))
    {
        eprintln!("cleany: update menu: {e}");
    }
}

fn set_phase<R: Runtime>(app: &AppHandle<R>, phase: MenuPhase) {
    if let Some(menu) = app.try_state::<UpdateMenu<R>>() {
        menu.update(|state| state.phase = phase);
    }
}

/// The item's text, and whether it can be clicked.
fn label(phase: &MenuPhase, locale: Locale) -> (String, bool) {
    match (phase, locale) {
        (MenuPhase::Idle, Locale::Tr) => ("Güncellemeleri denetle…".to_owned(), true),
        (MenuPhase::Idle, Locale::En) => ("Check for Updates…".to_owned(), true),
        (MenuPhase::Checking, Locale::Tr) => ("Güncellemeler denetleniyor…".to_owned(), false),
        (MenuPhase::Checking, Locale::En) => ("Checking for Updates…".to_owned(), false),
        (MenuPhase::Available { version }, Locale::Tr) => {
            (format!("Cleany {version} sürümünü yükle…"), true)
        }
        (MenuPhase::Available { version }, Locale::En) => {
            (format!("Install Cleany {version}…"), true)
        }
        (MenuPhase::Installing, Locale::Tr) => ("Güncelleme yükleniyor…".to_owned(), false),
        (MenuPhase::Installing, Locale::En) => ("Installing Update…".to_owned(), false),
    }
}

fn report_menu(result: tauri::Result<()>) {
    if let Err(e) = result {
        eprintln!("cleany: update menu: {e}");
    }
}

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
    set_phase(app, MenuPhase::Checking);
    let update = find(app).await;
    set_phase(
        app,
        match &update {
            Ok(Some(u)) => MenuPhase::Available {
                version: u.version.clone(),
            },
            Ok(None) | Err(_) => MenuPhase::Idle,
        },
    );
    let update = update?;
    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        can_install: installs_in_place(u.download_url.path()),
    });
    *pending.0.lock().map_err(failed)? = update;
    Ok(info)
}

async fn find<R: Runtime>(app: &AppHandle<R>) -> Result<Option<Update>, UpdateError> {
    #[allow(unused_mut)]
    let mut builder = app.updater_builder();
    #[cfg(debug_assertions)]
    if let Some(endpoint) = std::env::var_os("CLEANY_UPDATE_ENDPOINT") {
        let url = endpoint.to_string_lossy().parse().map_err(failed)?;
        builder = builder.endpoints(vec![url]).map_err(failed)?;
    }
    builder
        .build()
        .map_err(failed)?
        .check()
        .await
        .map_err(failed)
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
    set_phase(app, MenuPhase::Installing);
    let installed = update
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
        .map_err(failed);
    if installed.is_err() {
        // The pending update was taken; a new check has to find it again.
        set_phase(app, MenuPhase::Idle);
    }
    installed?;
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
    fn menu_label_follows_phase_and_locale() {
        assert_eq!(
            label(&MenuPhase::Idle, Locale::En),
            ("Check for Updates…".to_owned(), true)
        );
        assert_eq!(
            label(&MenuPhase::Checking, Locale::Tr),
            ("Güncellemeler denetleniyor…".to_owned(), false)
        );
        let found = MenuPhase::Available {
            version: "0.1.2".to_owned(),
        };
        assert_eq!(
            label(&found, Locale::Tr),
            ("Cleany 0.1.2 sürümünü yükle…".to_owned(), true)
        );
        assert_eq!(
            label(&found, Locale::En),
            ("Install Cleany 0.1.2…".to_owned(), true)
        );
        assert!(!label(&MenuPhase::Installing, Locale::En).1);
    }

    #[test]
    fn ui_listens_for_menu_events() {
        let ui = include_str!("../../ui/src/screens/UpdateStatus.tsx");
        assert!(
            ui.contains(CHECK_EVENT),
            "UpdateStatus.tsx misses {CHECK_EVENT}"
        );
        assert!(
            ui.contains(INSTALL_EVENT),
            "UpdateStatus.tsx misses {INSTALL_EVENT}"
        );
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
